use std::env;
use std::io::stdout;
use std::marker::PhantomData;
use std::sync::Arc;

use ratatui::layout::Size;
use ratatui::prelude::*;

use ratatui::{Terminal, TerminalOptions, Viewport};
use tokio::sync::mpsc;
use xilem_core::MessageResult;

use crate::widget::{BoxConstraints, ChangeFlags, Event};
use crate::widget::{Pod, Widget};
use crate::{App, Cx, View, ViewMarker};

/// Render a view and return the terminal to check the generated output
///
/// * `buffer_size` - The terminal output buffer is set to that size.
/// * `sut` - A closure that returns the widget to test when called.
///   This expects a function so it can create it inside the thread of the [`App`].
/// * `state` - Is the state for the [`App`]
pub fn render_view<T: Send + 'static>(
    buffer_size: Size,
    sut: Arc<impl View<T> + 'static>,
    state: T,
) -> Buffer {
    const CHANNEL_SIZE: usize = 3;
    let (message_tx, mut message_rx) = mpsc::channel::<Buffer>(CHANNEL_SIZE);
    let (event_tx, mut event_rx) = mpsc::channel(CHANNEL_SIZE);

    let event_tx_clone = event_tx.clone();

    let join_handle = std::thread::spawn(move || {
        let mut app = App::new(state, move |_state| {
            debug_view(sut.clone(), message_tx.clone())
        });
        event_tx_clone.blocking_send(app.event_tx()).unwrap();

        app.terminal_mut()
            .backend_mut()
            .resize(buffer_size.width, buffer_size.height);

        app.run().unwrap()
    });

    let event_tx = event_rx.blocking_recv().unwrap();

    let buffer = message_rx.blocking_recv();
    let send_quit_ack = event_tx.blocking_send(Event::Quit);

    join_handle.join().unwrap();

    // delay unwrapping until after join_handle.join() to not mask errors from the spawned thread
    send_quit_ack.unwrap();
    let buffer = buffer.unwrap();

    print_buffer(&buffer).unwrap();

    buffer
}

/// This widget provides access to the terminal output of its children
///
/// After its children were painted it calls the flush() and clones the
/// terminal's buffer and sends it using the passed [Sender<Buffer>].
///
/// This is handy for snapshot tests.
pub struct DebugView<V, T, A> {
    content: V,
    debug_chan_tx: mpsc::Sender<Buffer>,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub fn debug_view<V, T, A>(content: V, debug_chan_tx: mpsc::Sender<Buffer>) -> DebugView<V, T, A> {
    DebugView {
        content,
        debug_chan_tx,
        phantom: PhantomData,
    }
}

impl<T, A, V> ViewMarker for DebugView<V, T, A> {}

impl<T, A, V: View<T, A>> View<T, A> for DebugView<V, T, A> {
    type State = V::State;

    type Element = DebugWidget;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, state, element) = self.content.build(cx);
        (
            id,
            state,
            DebugWidget::new(element, self.debug_chan_tx.clone()),
        )
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let element = element
            .content
            .downcast_mut()
            .expect("The DebugView content widget changed its type, this should never happen!");
        self.content.rebuild(cx, &prev.content, id, state, element)
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.content.message(id_path, state, message, app_state)
    }
}

pub struct DebugWidget {
    content: Pod,
    debug_chan_tx: mpsc::Sender<Buffer>,
}

impl DebugWidget {
    pub(crate) fn new(content: impl Widget, debug_chan_tx: mpsc::Sender<Buffer>) -> Self {
        Self {
            content: Pod::new(content),
            debug_chan_tx,
        }
    }
}

impl Widget for DebugWidget {
    fn paint(&mut self, cx: &mut crate::widget::PaintCx) {
        self.content.paint(cx);

        cx.terminal.flush().unwrap();

        let buffer = cx.terminal.backend().buffer().to_owned();
        self.debug_chan_tx.blocking_send(buffer).unwrap();
    }

    fn layout(&mut self, cx: &mut crate::widget::LayoutCx, bc: &BoxConstraints) -> kurbo::Size {
        self.content.layout(cx, bc)
    }

    fn event(&mut self, cx: &mut crate::widget::EventCx, event: &crate::widget::Event) {
        self.content.event(cx, event)
    }

    fn lifecycle(&mut self, cx: &mut crate::widget::LifeCycleCx, event: &crate::widget::LifeCycle) {
        self.content.lifecycle(cx, event);
    }
}

/// Utility for visual snapshot test debugging
///
/// If the environment variable `DEBUG_SNAPSHOT` is set when tests are run, the terminal buffer is
/// dumped to stdout.
///
/// ```sh
/// DEBUG_SNAPSHOT=1 cargo test --lib -- --nocapture --test simple_border_test
/// ```
///
/// !!! The normal test output frequently interferes which results in scrambled output, especially
/// when multiple tests are run at once.
/// Running it multiple times might usually leads to good output (for now, with small widget output)
pub fn print_buffer(buffer: &Buffer) -> std::io::Result<()> {
    if env::var("DEBUG_SNAPSHOT").is_ok() {
        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(stdout()),
            TerminalOptions {
                viewport: Viewport::Fixed(buffer.area),
            },
        )?;

        terminal.clear()?;
        terminal.current_buffer_mut().clone_from(buffer);
        terminal.flush()?;
        crossterm::queue!(stdout(), crossterm::cursor::MoveTo(0, buffer.area.height))?;
    };
    Ok(())
}
