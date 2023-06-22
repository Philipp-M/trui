use crate::{
    view::{Cx, View},
    widget::{CxState, Event, EventCx, LayoutCx, PaintCx, Pod, PodFlags, WidgetState},
};
use anyhow::Result;
use crossterm::{
    cursor,
    event::{
        read, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture,
        KeyCode, KeyEvent,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::{
    io::{stdout, Stdout, Write},
    path::PathBuf,
};
use taffy::{style::AvailableSpace, Taffy};
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, Registry};
use xilem_core::{Id, Message};

// TODO less hardcoding and cross-platform support
fn setup_logging(log_level: tracing::Level) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let cache_dir = PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache/trui");
    let tracing_file_appender = tracing_appender::rolling::never(cache_dir, "trui.log");
    let (tracing_file_writer, guard) = tracing_appender::non_blocking(tracing_file_appender);

    let subscriber = Registry::default().with(
        tracing_subscriber::fmt::Layer::default()
            .with_writer(tracing_file_writer.with_max_level(log_level)),
    );
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(guard)
}

pub struct App<T: 'static, V: View<T> + 'static, F: FnMut(&mut T) -> V> {
    app_logic: F,
    data: T,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    view: Option<V>,
    state: Option<V::State>,
    events: Vec<Message>,
    root_state: WidgetState,
    root_pod: Option<Pod>,
    taffy: Taffy,
    cx: Cx,
    id: Option<Id>,
}

impl<T, V: View<T>, F: FnMut(&mut T) -> V> App<T, V, F> {
    pub fn new(data: T, app_logic: F) -> Self {
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend).unwrap(); // TODO handle errors...

        App {
            app_logic,
            data,
            terminal,
            root_pod: None,
            view: None,
            state: None,
            cx: Default::default(),
            id: None,
            root_state: WidgetState::new(),
            events: Vec::new(),
            taffy: Taffy::new(),
        }
    }

    /// returns true if it has repainted (i.e. needs to swap the buffers)
    #[tracing::instrument(skip(self))]
    fn render(&mut self, width: u16, height: u16) -> bool {
        let view = (self.app_logic)(&mut self.data);

        let mut cx = Cx::default();

        if let Some(element) = self.root_pod.as_mut() {
            let changes = view.rebuild(
                &mut cx,
                self.view.as_ref().unwrap(),
                self.id.as_mut().unwrap(),
                self.state.as_mut().unwrap(),
                element
                    .downcast_mut()
                    .expect("the root widget changed its type, this should never happen!"),
            );

            let changes = self.root_pod.as_mut().unwrap().mark(changes);
            tracing::debug!("changes after view rebuild: {changes:?}");
            assert!(self.cx.is_empty(), "id path imbalance on rebuild");
        } else {
            let (id, state, element) = view.build(&mut self.cx);

            assert!(self.cx.is_empty(), "id path imbalance on build");
            self.root_pod = Some(Pod::new(element));
            self.id = Some(id);
            self.state = Some(state);
        }

        self.view = Some(view);
        let root_pod = self.root_pod.as_mut().unwrap();

        let cx_state = &mut CxState::new(&mut self.events);

        if root_pod
            .state
            .flags
            .intersects(PodFlags::REQUEST_LAYOUT | PodFlags::TREE_CHANGED)
        {
            let _ = tracing::debug_span!("compute layout");
            let mut layout_cx = LayoutCx {
                taffy: &mut self.taffy,
                widget_state: &mut self.root_state,
                cx_state,
            };
            let layout_node = root_pod.layout(&mut layout_cx);
            self.taffy
                .compute_layout(
                    layout_node,
                    taffy::prelude::Size {
                        width: AvailableSpace::Definite(width as f32),
                        height: AvailableSpace::Definite(height as f32),
                    },
                )
                .ok();
        }

        let needs_paint = root_pod.state.flags.intersects(PodFlags::REQUEST_PAINT);
        if needs_paint {
            let _paint_span = tracing::debug_span!("paint");
            let mut paint_cx = PaintCx {
                widget_state: &mut self.root_state,
                cx_state,
                terminal: &mut self.terminal,
                taffy: &mut self.taffy,
                override_style: ratatui::style::Style::default(),
            };

            root_pod.paint(
                &mut paint_cx,
                Rect {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
            );
        }
        needs_paint
    }

    pub fn run(mut self) -> Result<()> {
        let _guard = setup_logging(tracing::Level::DEBUG)?;

        enable_raw_mode()?;
        execute!(
            stdout(),
            EnterAlternateScreen,
            EnableFocusChange,
            EnableMouseCapture,
            cursor::Hide
        )?;

        self.terminal.clear()?;

        let span = tracing::debug_span!("main loop");
        loop {
            self.terminal.autoresize()?;
            let size = self.terminal.size()?;
            let needs_update = self.render(size.width, size.height);
            if needs_update {
                self.terminal.flush()?;
                self.terminal.swap_buffers();
                self.terminal.backend_mut().flush()?;
            }

            let event = match read()? {
                crossterm::event::Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) => break,
                crossterm::event::Event::Key(key_event) => Event::Key(key_event),
                crossterm::event::Event::Mouse(mouse_event) => Event::Mouse(mouse_event),
                crossterm::event::Event::FocusGained => Event::FocusGained,
                crossterm::event::Event::FocusLost => Event::FocusLost,
                // crossterm::event::Event::Paste(_) => todo!(),
                crossterm::event::Event::Resize(width, height) => Event::Resize { width, height },
                _ => continue, // TODO handle other kinds of events
            };

            let cx_state = &mut CxState::new(&mut self.events);

            let mut cx = EventCx {
                is_handled: false,
                widget_state: &mut self.root_state,
                cx_state,
            };

            self.root_pod.as_mut().unwrap().event(&mut cx, &event);

            let events = std::mem::take(&mut self.events);

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
        drop(span);

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
}
