use crate::{geometry::Size, Fill};

use super::{
    animatables::AnimatableElement,
    core::{EventCx, LifeCycleCx, PaintCx},
    BoxConstraints, ChangeFlags, Event, LayoutCx, LifeCycle, Pod, Widget,
};

pub struct FillMaxSize<P> {
    pub(crate) content: Pod,
    fill: Fill,
    pub(crate) percent: P,
    percent_value: f64,
}

impl<P> FillMaxSize<P> {
    pub(crate) fn new(content: impl Widget, fill: Fill, percent: P) -> Self {
        FillMaxSize {
            content: Pod::new(content),
            fill,
            percent,
            percent_value: 1.0,
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
}

impl<P: AnimatableElement<f64> + 'static> Widget for FillMaxSize<P> {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.content.paint(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let mut bc = *bc;
        if self.fill.contains(Fill::WIDTH) && bc.is_width_bounded() {
            bc = bc
                .constrain_width_to(bc.max().width * self.percent_value)
                .tighten_max_width();
        }
        if self.fill.contains(Fill::HEIGHT) && bc.is_height_bounded() {
            bc = bc
                .constrain_height_to(bc.max().height * self.percent_value)
                .tighten_max_height();
        }
        self.content.layout(cx, &bc)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.content.event(cx, event)
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        if let LifeCycle::Animate = event {
            let new_percent_value = *self.percent.animate(cx);
            if new_percent_value != self.percent_value {
                cx.request_layout();
                self.percent_value = new_percent_value;
            }
        }
        self.content.lifecycle(cx, event);
    }
}
