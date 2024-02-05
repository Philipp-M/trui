use crate::{geometry::Size, Fill};

use super::{
    core::{EventCx, LifeCycleCx, PaintCx},
    BoxConstraints, ChangeFlags, Event, LayoutCx, LifeCycle, Pod, Widget,
};

pub struct FillMaxSize {
    pub(crate) content: Pod,
    fill: Fill,
    percent: f64,
}

impl FillMaxSize {
    pub(crate) fn new(content: impl Widget, fill: Fill, percent: f64) -> Self {
        FillMaxSize {
            content: Pod::new(content),
            fill,
            percent,
        }
    }

    pub(crate) fn set_fill(&mut self, fill: Fill) -> ChangeFlags {
        if self.fill != fill {
            self.fill = fill;
            ChangeFlags::LAYOUT
        } else {
            ChangeFlags::empty()
        }
    }

    pub(crate) fn set_percent(&mut self, percent: f64) -> ChangeFlags {
        let percent = percent.clamp(0.0, 1.0);
        if self.percent != percent {
            self.percent = percent;
            ChangeFlags::LAYOUT
        } else {
            ChangeFlags::empty()
        }
    }
}

impl Widget for FillMaxSize {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.content.paint(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let mut bc = *bc;
        if self.fill.contains(Fill::WIDTH) && bc.is_width_bounded() {
            bc = bc.constrain_width_to(bc.max().width * self.percent);
        }
        if self.fill.contains(Fill::HEIGHT) && bc.is_height_bounded() {
            bc = bc.constrain_height_to(bc.max().height * self.percent);
        }
        self.content.layout(cx, &bc)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.content.event(cx, event)
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.content.lifecycle(cx, event)
    }
}
