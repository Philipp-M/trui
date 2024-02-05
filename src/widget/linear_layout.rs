use crate::geometry::{Axis, Size};

use super::{
    core::{EventCx, PaintCx},
    BoxConstraints, LayoutCx, Pod, Widget,
};

pub struct LinearLayout {
    pub children: Vec<Pod>,
    pub spacing: f64,
    pub axis: Axis,
}

impl LinearLayout {
    pub(crate) fn new(children: Vec<Pod>, spacing: f64, axis: Axis) -> Self {
        LinearLayout {
            children,
            axis,
            spacing,
        }
    }
}

impl Widget for LinearLayout {
    fn paint(&mut self, cx: &mut PaintCx) {
        for child in self.children.iter_mut() {
            child.paint(cx);
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let major_max = self.axis.major(*bc).end;
        let mut child_bc = self.axis.with_major(bc.loosen(), 0.0..major_max);
        let child_count = self.children.len();

        let mut major_used: f64 = 0.0;
        let mut max_minor: f64 = 0.0;

        for (index, child) in self.children.iter_mut().enumerate() {
            let size = child.layout(cx, &child_bc);
            child.set_origin(cx, self.axis.pack(major_used, 0.0));
            major_used += self.axis.major(size);
            if index < child_count - 1 {
                major_used += self.spacing;
            }
            child_bc = child_bc.shrink_max_to(self.axis, major_max - major_used);
            max_minor = max_minor.max(self.axis.minor(size));
        }

        bc.constrain(self.axis.pack::<Size>(major_used, max_minor))
    }

    fn event(&mut self, cx: &mut EventCx, event: &super::Event) {
        for child in &mut self.children {
            child.event(cx, event);
        }
    }

    fn lifecycle(&mut self, cx: &mut super::core::LifeCycleCx, event: &super::LifeCycle) {
        for child in &mut self.children {
            child.lifecycle(cx, event);
        }
    }
}
