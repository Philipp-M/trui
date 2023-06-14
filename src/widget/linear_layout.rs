use ratatui::layout::Rect;
use taffy::{
    prelude::NodeId,
    style::{FlexDirection, Style},
};

use super::{
    core::{EventCx, PaintCx},
    LayoutCx, Pod, StyleCx, Widget,
};

pub struct LinearLayout {
    pub children: Vec<Pod>,
    pub direction: FlexDirection,
}

impl Widget for LinearLayout {
    fn paint(&mut self, cx: &mut PaintCx, _rect: Rect) {
        for child in self.children.iter_mut() {
            child.paint(cx);
        }
    }

    fn style(&mut self, cx: &mut StyleCx, _prev: NodeId) -> NodeId {
        let style = Style {
            size: taffy::prelude::Size {
                width: taffy::style::Dimension::Percent(1.0),
                height: taffy::style::Dimension::Percent(1.0),
            },
            flex_direction: self.direction,
            ..Default::default()
        };
        let children: Vec<_> = self
            .children
            .iter_mut()
            .map(|child| child.style(cx))
            .collect();
        cx.taffy.new_with_children(style, &children).unwrap()
    }

    fn event(&mut self, cx: &mut EventCx, event: &super::Event) {
        for child in &mut self.children {
            child.event(cx, event);
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, rect: Rect) {
        for child in &mut self.children {
            child.layout(cx, rect);
        }
    }
}
