use ratatui::backend::TestBackend;
use ratatui::layout::Size;
use ratatui::style::Style;
use ratatui::Terminal;
// use taffy::style_helpers::{length, TaffyMaxContent};
// use taffy::{FlexDirection, NodeId, TaffyTree};

use crate::widget::{CxState, LayoutCx, PaintCx, Widget, WidgetState};

pub(crate) fn render_widget(buffer_size: Size, sut: &mut impl Widget) -> Terminal<TestBackend> {
    let mut messages = vec![];
    let mut cx_state = CxState::new(&mut messages);
    let mut widget_state = WidgetState::new();

    // let mut taffy = TaffyTree::default();
    let mut layout_cx = LayoutCx {
        cx_state: &mut cx_state,
        widget_state: &mut widget_state,
        // taffy: &mut taffy,
    };
    // let node_id = sut.layout(&mut layout_cx, NodeId::null());
    // let root_node = taffy
    //     .new_with_children(
    //         taffy::Style {
    //             flex_direction: FlexDirection::Column,
    //             size: taffy::Size {
    //                 width: length(buffer_size.width),
    //                 height: length(buffer_size.height),
    //             },
    //             ..Default::default()
    //         },
    //         &[node_id],
    //     )
    //     .unwrap();
    // taffy
    //     .compute_layout(root_node, taffy::Size::MAX_CONTENT)
    //     .unwrap();

    let backend = TestBackend::new(buffer_size.width, buffer_size.height);

    let mut terminal = Terminal::new(backend).unwrap();

    let mut paint_cx = PaintCx {
        cx_state: &mut cx_state,
        widget_state: &mut widget_state,
        terminal: &mut terminal,
        // taffy: &mut taffy,
        override_style: Style::default(),
    };

    sut.paint(&mut paint_cx);
    terminal.flush().unwrap();

    terminal
}
