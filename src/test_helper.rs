use std::marker::PhantomData;

use ratatui::backend::TestBackend;
use ratatui::layout::Size;
use ratatui::prelude::Buffer;
use ratatui::style::Style;
use ratatui::Terminal;
use tokio::sync::mpsc;
use xilem_core::MessageResult;

use crate::widget::{BoxConstraints, ChangeFlags, Event};
use crate::widget::{CxState, LayoutCx, PaintCx, Pod, Widget, WidgetState};
use crate::{AnyView, App, Cx, View, ViewMarker};

/// Render a view and return the terminal to check the generated output
///
/// * `buffer_size` - The terminal output buffer is set to that size.
/// * `sut` - A closure that returns the widget to test when called.
///   This expects a function so it can create it inside the thread of the [`App`].
/// * `state` - Is the state for the [`App`]
pub fn render_view<T: Send + 'static>(
    buffer_size: Size,
    sut: Box<dyn Fn() -> Box<dyn AnyView<T>> + Send + Sync>,
    state: T,
) -> Buffer {
    const CHANNEL_SIZE: usize = 3;
    let (message_tx, mut message_rx) = mpsc::channel::<Buffer>(CHANNEL_SIZE);
    let (event_tx, mut event_rx) = mpsc::channel(CHANNEL_SIZE);

    let event_tx_clone = event_tx.clone();

    let join_handle = std::thread::spawn(move || {
        let mut app = App::new(state, move |_state| {
            let w = sut();
            debug_view(w, message_tx.clone())
        });
        event_tx_clone.blocking_send(app.get_event_tx()).unwrap();

        app.get_terminal()
            .backend_mut()
            .resize(buffer_size.width, buffer_size.height);

        app.run()
    });

    let event_tx = event_rx.blocking_recv().unwrap();

    let buffer = message_rx.blocking_recv().unwrap();

    event_tx.blocking_send(Event::Quit).unwrap();
    let _ = join_handle.join().unwrap();
    buffer
}

/// Render a widget and return the terminal to check the generated output
///
/// * `buffer_size` - The terminal output buffer is set to that size.
/// * `sut` - (system under test) The widget to render.
pub fn render_widget(buffer_size: Size, sut: &mut impl Widget) -> Terminal<TestBackend> {
    let mut messages = vec![];
    let mut cx_state = CxState::new(&mut messages);
    let mut widget_state = WidgetState::new();

    let mut layout_cx = LayoutCx {
        cx_state: &mut cx_state,
        widget_state: &mut widget_state,
    };
    let backend = TestBackend::new(buffer_size.width, buffer_size.height);

    let mut terminal = Terminal::new(backend).unwrap();
    let _size = sut.layout(
        &mut layout_cx,
        &BoxConstraints::new(
            kurbo::Size::ZERO,
            kurbo::Size {
                width: buffer_size.width.into(),
                height: buffer_size.height.into(),
            },
        ),
    );

    let mut paint_cx = PaintCx {
        cx_state: &mut cx_state,
        widget_state: &mut widget_state,
        terminal: &mut terminal,
        override_style: Style::default(),
    };

    sut.paint(&mut paint_cx);
    terminal.flush().unwrap();

    terminal
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

impl<T, A, V> View<T, A> for DebugView<V, T, A>
where
    V: View<T, A>,
    V::Element: 'static,
{
    type State = (V::State, xilem_core::Id);

    type Element = DebugWidget;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, state, element) = self.content.build(cx);
        (id, state, DebugWidget::new(element, self.debug_chan_tx.clone()))
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        (state, child_id): &mut Self::State,
        element: &mut Self::Element,
    ) -> crate::widget::ChangeFlags {
        let changeflags = ChangeFlags::empty();

        let element = element
            .content
            .downcast_mut()
            .expect("The DebugView content widget changed its type, this should never happen!");

        changeflags
            | cx.with_id(*id, |cx| {
                self.content
                    .rebuild(cx, &prev.content, child_id, state, element)
            })
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        (state, child_id): &mut Self::State,
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
