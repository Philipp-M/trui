use crate::{
    geometry::{Point, Size},
    view::{Cx, View},
    widget::{
        BoxConstraints, CxState, Event, EventCx, LayoutCx, LifeCycle, LifeCycleCx, Message,
        PaintCx, Pod, PodFlags, ViewContext, WidgetState,
    },
};
use anyhow::Result;

#[cfg(not(any(test, doctest, feature = "doctests")))]
use crossterm::{
    cursor,
    event::{DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
    execute, queue,
    terminal::{
        disable_raw_mode, enable_raw_mode, BeginSynchronizedUpdate, EndSynchronizedUpdate,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};

use crossterm::event::{poll, read, Event as CxEvent, KeyCode, KeyEvent};
use ratatui::Terminal;

#[cfg(not(any(test, doctest, feature = "doctests")))]
use std::io::stdout;

use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use xilem_core::{AsyncWake, Id, IdPath, MessageResult};

#[cfg(any(test, doctest, feature = "doctests"))]
use ratatui::backend::TestBackend;

#[cfg(not(any(test, doctest, feature = "doctests")))]
use ratatui::backend::CrosstermBackend;
#[cfg(not(any(test, doctest, feature = "doctests")))]
use std::io::{Stdout, Write};

pub struct App<T: Send + 'static, V: View<T> + 'static> {
    req_chan: tokio::sync::mpsc::Sender<AppMessage>,
    render_response_chan: tokio::sync::mpsc::Receiver<RenderResponse<V, V::State>>,
    return_chan: tokio::sync::mpsc::Sender<(V, V::State, HashSet<Id>)>,
    event_chan: tokio::sync::mpsc::Receiver<Event>,

    #[cfg(any(test, doctest, feature = "doctests"))]
    event_tx: tokio::sync::mpsc::Sender<Event>,

    #[cfg(any(test, doctest, feature = "doctests"))]
    terminal: Terminal<TestBackend>,

    #[cfg(not(any(test, doctest, feature = "doctests")))]
    terminal: Terminal<CrosstermBackend<Stdout>>,
    size: Size,
    request_render_notifier: Arc<tokio::sync::Notify>,
    cursor_pos: Option<Point>,
    events: Vec<Message>,
    root_state: WidgetState,
    root_pod: Option<Pod>,
    cx: Cx,
    id: Option<Id>,
}

/// The standard delay for waiting for async futures.
const RENDER_DELAY: Duration = Duration::from_millis(5);

/// This is the view logic of Xilem.
///
/// It contains no information about how to interact with the User (browser, native, terminal).
/// It is created by [`App`] and kept in a separate task for updating the apps contents.
/// The App can send [AppMessage] to inform the the AppTask about an user interaction.
struct AppTask<T, V: View<T>, F: FnMut(&mut T) -> V> {
    req_chan: tokio::sync::mpsc::Receiver<AppMessage>,
    response_chan: tokio::sync::mpsc::Sender<RenderResponse<V, V::State>>,
    return_chan: tokio::sync::mpsc::Receiver<(V, V::State, HashSet<Id>)>,
    event_chan: tokio::sync::mpsc::Sender<Event>,

    data: T,
    app_logic: F,
    view: Option<V>,
    state: Option<V::State>,
    pending_async: HashSet<Id>,
    ui_state: UiState,
}

// TODO maybe rename this, so that it is clear that these events are sent to the AppTask (AppTask name is also for debate IMO)
/// A message sent from the main UI thread ([`App`]) to the [`AppTask`].
pub(crate) enum AppMessage {
    Events(Vec<Message>),
    Wake(IdPath),
    // Parameter indicates whether it should be delayed for async
    Render(bool),
}

/// A message sent from [`AppTask`] to [`App`] in response to a render request.
struct RenderResponse<V, S> {
    prev: Option<V>,
    view: V,
    state: Option<S>,
}

/// The state of the  [`AppTask`].
///
/// While the [`App`] follows a strict order of UIEvents -> Render -> Paint (this is simplified)
/// the [`AppTask`] can receive different requests at any time. This enum keeps track of the state
/// the AppTask is in because of previous requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum UiState {
    /// Starting state, ready for events and render requests.
    Start,
    /// Received render request, haven't responded yet.
    Delayed,
    /// An async completion woke the UI thread.
    WokeUI,
}

impl<T: Send + 'static, V: View<T> + 'static> App<T, V> {
    pub fn new(data: T, app_logic: impl FnMut(&mut T) -> V + Send + 'static) -> Self {
        #[cfg(not(any(test, doctest, feature = "doctests")))]
        let backend = CrosstermBackend::new(stdout()); // TODO handle errors...

        #[cfg(any(test, doctest, feature = "doctests"))]
        let backend = TestBackend::new(80, 40);

        let terminal = Terminal::new(backend).unwrap();

        // Create a new tokio runtime. Doing it here is hacky, we should allow
        // the client to do it.
        let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());

        // Note: there is danger of deadlock if exceeded; think this through.
        const CHANNEL_SIZE: usize = 1000;
        let (message_tx, message_rx) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let (response_tx, response_rx) = tokio::sync::mpsc::channel(1);
        let (return_tx, return_rx) = tokio::sync::mpsc::channel(1);

        // We have a separate thread to forward wake requests (mostly generated
        // by the custom waker when we poll) to the async task. Maybe there's a
        // better way, but this is expedient.
        //
        // It's a sync_channel because sender needs to be sync to work in an async
        // context. Consider crossbeam and flume channels as alternatives.
        let message_tx_clone = message_tx.clone();
        let (wake_tx, wake_rx) = std::sync::mpsc::sync_channel(10);
        std::thread::spawn(move || {
            while let Ok(id_path) = wake_rx.recv() {
                let _ = message_tx_clone.blocking_send(AppMessage::Wake(id_path));
            }
        });

        let request_render_notifier = Arc::new(tokio::sync::Notify::new());

        let request_render_notifier_clone = Arc::clone(&request_render_notifier);
        let event_tx_clone = event_tx.clone();

        // Until we have a solid way to sync with the screen refresh rate, do an update every 1/60 secs when it is requested
        rt.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs_f64(1.0 / 60.0));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                request_render_notifier_clone.notified().await;
                interval.tick().await;
                if event_tx_clone.send(Event::Wake).await.is_err() {
                    break;
                }
            }
        });

        // spawn io event proxy task
        let event_tx_clone = event_tx.clone();
        std::thread::spawn(move || {
            loop {
                if let Ok(true) = poll(Duration::from_millis(100)) {
                    let event = match read() {
                        // TODO quit app at least for now, until proper key handling is implemented, then this thread might need a signal to quit itself
                        Ok(CxEvent::Key(KeyEvent {
                            code: KeyCode::Esc, ..
                        })) => Event::Quit,
                        Ok(CxEvent::Key(key_event)) => Event::Key(key_event),
                        Ok(CxEvent::Mouse(mouse_event)) => Event::Mouse(mouse_event.into()),
                        Ok(CxEvent::FocusGained) => Event::FocusGained,
                        Ok(CxEvent::FocusLost) => Event::FocusLost,
                        // CxEvent::Paste(_) => todo!(),
                        Ok(CxEvent::Resize(width, height)) => Event::Resize { width, height },
                        _ => continue, // TODO handle other kinds of events and errors
                    };

                    let quit = matches!(event, Event::Quit);

                    let _ = event_tx_clone.blocking_send(event);

                    if quit {
                        break;
                    }
                }
            }
        });

        // Send this event here, so that the app renders directly when it is run.
        let _ = event_tx.blocking_send(Event::Start);

        let event_tx_clone = event_tx.clone();
        // spawn app task
        rt.spawn(async move {
            let mut app_task = AppTask {
                req_chan: message_rx,
                response_chan: response_tx,
                return_chan: return_rx,
                event_chan: event_tx_clone,
                data,
                app_logic,
                view: None,
                state: None,
                pending_async: HashSet::new(),
                ui_state: UiState::Start,
            };
            app_task.run().await;
        });

        let cx = Cx::new(&wake_tx, rt);

        App {
            req_chan: message_tx,
            render_response_chan: response_rx,
            return_chan: return_tx,
            event_chan: event_rx,

            #[cfg(any(test, doctest, feature = "doctests"))]
            event_tx: event_tx.clone(),

            terminal,
            size: Size::default(),
            cursor_pos: None,
            root_pod: None,
            cx,
            id: None,
            root_state: WidgetState::new(),
            events: Vec::new(),
            request_render_notifier,
        }
    }

    fn send_events(&mut self) {
        if !self.events.is_empty() {
            let events = std::mem::take(&mut self.events);
            let _ = self.req_chan.blocking_send(AppMessage::Events(events));
        }
    }

    /// Run the app logic and update the widget tree.
    /// Returns whether a rerender should be scheduled
    #[tracing::instrument(skip(self))]
    fn render(&mut self, time_since_last_render: Duration) -> Result<bool> {
        if self.build_widget_tree(false) {
            self.build_widget_tree(true);
        }
        let root_pod = self.root_pod.as_mut().unwrap();
        let cx_state = &mut CxState::new(&mut self.events, time_since_last_render);

        // TODO via event (Event::Resize)?
        self.terminal.autoresize()?;

        let term_rect = self.terminal.size()?;
        let ratatui::layout::Rect { width, height, .. } = term_rect;
        let term_size = Size {
            width: width as f64,
            height: height as f64,
        };

        if root_pod.state.flags.contains(PodFlags::REQUEST_ANIMATION) {
            root_pod.lifecycle(
                &mut LifeCycleCx {
                    cx_state,
                    widget_state: &mut self.root_state,
                },
                &LifeCycle::Animate,
            );
        }

        let needs_layout_recomputation = root_pod
            .state
            .flags
            .intersects(PodFlags::REQUEST_LAYOUT | PodFlags::TREE_CHANGED)
            || term_size != self.size;

        if needs_layout_recomputation {
            let _ = tracing::debug_span!("compute layout");
            self.size = term_size;
            let mut layout_cx = LayoutCx {
                widget_state: &mut self.root_state,
                cx_state,
            };
            let bc = BoxConstraints::tight(self.size).loosen();
            root_pod.layout(&mut layout_cx, &bc);
            root_pod.set_origin(&mut layout_cx, Point::ORIGIN);
        }
        if root_pod
            .state
            .flags
            .contains(PodFlags::VIEW_CONTEXT_CHANGED)
        {
            let view_context = ViewContext {
                window_origin: Point::ORIGIN,
                // clip: Rect::from_origin_size(Point::ORIGIN, root_pod.state.size),
                mouse_position: self.cursor_pos,
            };
            let mut lifecycle_cx = LifeCycleCx {
                cx_state,
                widget_state: &mut self.root_state,
            };
            root_pod.lifecycle(
                &mut lifecycle_cx,
                &LifeCycle::ViewContextChanged(view_context),
            );
        }

        if root_pod.state.flags.intersects(PodFlags::REQUEST_PAINT) || needs_layout_recomputation {
            let _paint_span = tracing::debug_span!("paint");
            let mut paint_cx = PaintCx {
                widget_state: &mut self.root_state,
                cx_state,
                terminal: &mut self.terminal,
                override_style: ratatui::style::Style::default(),
            };

            root_pod.paint(&mut paint_cx);

            #[cfg(not(any(test, doctest, feature = "doctests")))]
            queue!(stdout(), BeginSynchronizedUpdate)?;

            self.terminal.flush()?;

            #[cfg(not(any(test, doctest, feature = "doctests")))]
            execute!(stdout(), EndSynchronizedUpdate)?;

            self.terminal.swap_buffers();

            #[cfg(not(any(test, doctest, feature = "doctests")))]
            self.terminal.backend_mut().flush()?;
        }

        // currently only an animation update can request a rerender
        Ok(root_pod.state.flags.contains(PodFlags::REQUEST_ANIMATION))
    }

    /// Run one pass of app logic.
    ///
    /// Return value is whether there are any pending async futures.
    fn build_widget_tree(&mut self, delay: bool) -> bool {
        self.cx.pending_async.clear();
        let _ = self.req_chan.blocking_send(AppMessage::Render(delay));
        if let Some(response) = self.render_response_chan.blocking_recv() {
            let state = if let Some(widget) = self.root_pod.as_mut() {
                let mut state = response.state.unwrap();
                let changes = response.view.rebuild(
                    &mut self.cx,
                    response.prev.as_ref().unwrap(),
                    self.id.as_mut().unwrap(),
                    &mut state,
                    //TODO: fail more gracefully but make it explicit that this is a bug
                    widget
                        .downcast_mut()
                        .expect("the root widget changed its type, this should never happen!"),
                );
                let _ = self.root_pod.as_mut().unwrap().mark(changes);
                assert!(self.cx.is_empty(), "id path imbalance on rebuild");
                state
            } else {
                let (id, state, widget) = response.view.build(&mut self.cx);
                assert!(self.cx.is_empty(), "id path imbalance on build");
                self.root_pod = Some(Pod::new(widget));
                self.id = Some(id);
                state
            };
            let pending = std::mem::take(&mut self.cx.pending_async);
            let has_pending = !pending.is_empty();
            let _ = self
                .return_chan
                .blocking_send((response.view, state, pending));
            has_pending
        } else {
            false
        }
    }

    pub fn run(mut self) -> Result<()> {
        #[cfg(not(any(test, doctest, feature = "doctests")))]
        self.init_terminal()?;

        self.terminal.clear()?;

        let main_loop_tracing_span = tracing::debug_span!("main loop");
        let mut time_of_last_render = Instant::now();
        let mut time_since_last_render_request = Duration::ZERO;
        while let Some(event) = self.event_chan.blocking_recv() {
            let mut events = vec![event];
            // batch events
            while let Ok(event) = self.event_chan.try_recv() {
                events.push(event);
            }

            let quit = events.iter().any(|e| matches!(e, Event::Quit));

            if let Some(Event::Mouse(mouse)) = events
                .iter()
                .rev()
                .find(|event| matches!(event, Event::Mouse(_)))
            {
                self.cursor_pos = Some(Point::new(mouse.column as f64, mouse.row as f64));
            }

            if let Some(root_pod) = self.root_pod.as_mut() {
                let cx_state = &mut CxState::new(&mut self.events, time_since_last_render_request);

                let mut cx = EventCx {
                    is_handled: false,
                    widget_state: &mut self.root_state,
                    cx_state,
                };
                for event in events {
                    // TODO filter out some events like Event::Wake?
                    root_pod.event(&mut cx, &event);
                }
            }
            self.send_events();

            let rerender_requested = self.render(time_since_last_render_request)?;
            // TODO this is a workaround (I consider this at least as that) for getting animations right
            // There's likely a cleaner solution
            if rerender_requested {
                self.request_render_notifier.notify_one();
                time_since_last_render_request = time_of_last_render.elapsed();
            } else {
                time_since_last_render_request = Duration::ZERO;
            }
            time_of_last_render = Instant::now();

            if quit {
                break;
            }
        }
        drop(main_loop_tracing_span);

        Ok(())
    }

    #[cfg(not(any(test, doctest, feature = "doctests")))]
    fn init_terminal(&self) -> Result<()> {
        enable_raw_mode()?;
        execute!(
            stdout(),
            EnterAlternateScreen,
            EnableFocusChange,
            EnableMouseCapture,
            cursor::Hide
        )?;
        Ok(())
    }

    #[cfg(not(any(test, doctest, feature = "doctests")))]
    fn restore_terminal(&self) -> Result<()> {
        execute!(
            stdout(),
            cursor::Show,
            LeaveAlternateScreen,
            DisableFocusChange,
            DisableMouseCapture
        )?;
        disable_raw_mode()?;
        Ok(())
    }

    #[cfg(any(test, doctest, feature = "doctests"))]
    pub fn event_tx(&self) -> tokio::sync::mpsc::Sender<Event> {
        self.event_tx.clone()
    }

    #[cfg(any(test, doctest, feature = "doctests"))]
    pub fn terminal_mut(&mut self) -> &mut Terminal<TestBackend> {
        &mut self.terminal
    }
}

/// Restore the terminal no matter how the app exits
impl<T: Send + 'static, V: View<T> + 'static> Drop for App<T, V> {
    fn drop(&mut self) {
        #[cfg(not(any(test, doctest, feature = "doctests")))]
        self.restore_terminal()
            .unwrap_or_else(|e| eprint!("Restoring the terminal failed: {e}"));
    }
}

impl<T, V: View<T>, F: FnMut(&mut T) -> V> AppTask<T, V, F> {
    async fn run(&mut self) {
        let mut deadline = None;
        loop {
            let rx = self.req_chan.recv();
            let req = match deadline {
                Some(deadline) => tokio::time::timeout_at(deadline, rx).await,
                None => Ok(rx.await),
            };
            match req {
                Ok(Some(req)) => match req {
                    AppMessage::Events(events) => {
                        for event in events {
                            let id_path = &event.id_path[1..];
                            self.view.as_ref().unwrap().message(
                                id_path,
                                self.state.as_mut().unwrap(),
                                event.body,
                                &mut self.data,
                            );
                        }
                    }
                    AppMessage::Wake(id_path) => {
                        let needs_rebuild;
                        {
                            let result = self.view.as_ref().unwrap().message(
                                &id_path[1..],
                                self.state.as_mut().unwrap(),
                                Box::new(AsyncWake),
                                &mut self.data,
                            );
                            needs_rebuild = matches!(result, MessageResult::RequestRebuild);
                            tracing::debug!("Needs rebuild after wake: {needs_rebuild}");
                        }

                        if needs_rebuild {
                            // request re-render from UI thread
                            if self.ui_state == UiState::Start {
                                self.ui_state = UiState::WokeUI;
                                tracing::debug!("Sending wake event");
                                if self.event_chan.send(Event::Wake).await.is_err() {
                                    break;
                                }
                            }
                            let id = id_path.last().unwrap();
                            self.pending_async.remove(id);
                            if self.pending_async.is_empty() && self.ui_state == UiState::Delayed {
                                tracing::debug!("Render with delayed ui state");
                                self.render().await;
                                deadline = None;
                            }
                        }
                    }
                    AppMessage::Render(delay) => {
                        if !delay || self.pending_async.is_empty() {
                            tracing::debug!("Render without delay");
                            self.render().await;
                            deadline = None;
                        } else {
                            tracing::debug!(
                                "Pending async, delay rendering by {} us",
                                RENDER_DELAY.as_micros()
                            );
                            deadline = Some(tokio::time::Instant::now() + RENDER_DELAY);
                            self.ui_state = UiState::Delayed;
                        }
                    }
                },
                Ok(None) => break,
                Err(_) => {
                    tracing::debug!("Render after delay");
                    self.render().await;
                    deadline = None;
                }
            }
        }
    }

    async fn render(&mut self) {
        let view = (self.app_logic)(&mut self.data);
        let response = RenderResponse {
            prev: self.view.take(),
            view,
            state: self.state.take(),
        };
        if self.response_chan.send(response).await.is_err() {
            tracing::error!("error sending render response");
        }
        if let Some((view, state, pending)) = self.return_chan.recv().await {
            self.view = Some(view);
            self.state = Some(state);
            self.pending_async = pending;
        }
        self.ui_state = UiState::Start;
    }
}
