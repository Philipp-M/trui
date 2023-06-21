use taffy::{
    prelude::NodeId,
    style::{FlexDirection, Style},
};

use super::{
    core::{update_layout_node, EventCx, PaintCx},
    LayoutCx, Pod, Widget,
};

pub struct LinearLayout {
    pub children: Vec<Pod>,
    pub direction: FlexDirection,
    style: Style,
}

impl LinearLayout {
    pub(crate) fn new(children: Vec<Pod>, direction: FlexDirection) -> Self {
        LinearLayout {
            children,
            direction,
            style: Style {
                size: taffy::prelude::Size {
                    width: taffy::style::Dimension::Percent(1.0),
                    height: taffy::style::Dimension::Percent(1.0),
                },
                flex_direction: direction,
                ..Default::default()
            },
        }
    }
}

impl Widget for LinearLayout {
    fn paint(&mut self, cx: &mut PaintCx) {
        for child in self.children.iter_mut() {
            child.paint(cx, cx.rect());
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
        let children: Vec<_> = self
            .children
            .iter_mut()
            .map(|child| child.layout(cx))
            .collect();
        if !prev.is_null() {
            update_layout_node(prev, cx.taffy, &children, &self.style);
            prev
        } else {
            cx.taffy
                .new_with_children(self.style.clone(), &children)
                .unwrap()
        }
    }

    fn event(&mut self, cx: &mut EventCx, event: &super::Event) {
        for child in &mut self.children {
            child.event(cx, event);
        }
    }
}
