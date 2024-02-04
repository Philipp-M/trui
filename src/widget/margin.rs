use crate::{
    geometry::{Point, Size},
    Position,
};

use super::{
    core::{EventCx, LifeCycleCx, PaintCx},
    BoxConstraints, ChangeFlags, Event, LayoutCx, LifeCycle, Pod, Widget,
};

pub struct Margin {
    pub(crate) content: Pod,
    amount: u16,
    position: Position,
}

impl Margin {
    pub(crate) fn new(content: impl Widget, position: Position, amount: u16) -> Self {
        Margin {
            content: Pod::new(content),
            amount,
            position,
        }
    }

    pub(crate) fn set_amount(&mut self, amount: u16) -> ChangeFlags {
        if self.amount != amount {
            self.amount = amount;
            ChangeFlags::LAYOUT
        } else {
            ChangeFlags::empty()
        }
    }

    pub(crate) fn set_position(&mut self, position: Position) -> ChangeFlags {
        if self.position != position {
            self.position = position;
            ChangeFlags::LAYOUT
        } else {
            ChangeFlags::empty()
        }
    }
}

impl Widget for Margin {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.content.paint(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let margin = |position| {
            if self.position.contains(position) {
                self.amount as f64
            } else {
                0.0
            }
        };
        let margin_top = margin(Position::TOP);
        let margin_bottom = margin(Position::BOTTOM);
        let margin_left = margin(Position::LEFT);
        let margin_right = margin(Position::RIGHT);
        let margin = Size::new(margin_left + margin_right, margin_top + margin_bottom);
        let content_size = self.content.layout(cx, &bc.shrink(margin));

        self.content
            .set_origin(cx, Point::new(margin_left, margin_top));
        content_size + margin
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.content.event(cx, event)
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.content.lifecycle(cx, event)
    }
}
