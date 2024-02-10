use super::{
    core::LayoutCx, core::PaintCx, BoxConstraints, ChangeFlags, Event, EventCx, Pod, Widget,
};
use crate::{
    geometry::{to_ratatui_rect, Point, Size},
    view::Borders,
    BorderKind,
};
use ratatui::{style::Style, symbols};

pub struct Border {
    pub(crate) content: Pod,
    borders: Borders,
    kind: BorderKind,
    style: Style,
}

impl Border {
    pub(crate) fn new(
        content: impl Widget,
        borders: Borders,
        style: Style,
        kind: BorderKind,
    ) -> Self {
        Border {
            content: Pod::new(content),
            borders,
            kind,
            style,
        }
    }

    pub(crate) fn set_borders(&mut self, borders: Borders) -> ChangeFlags {
        if self.borders != borders {
            self.borders = borders;
            // TODO more sophisticated check for needed ChangeFlags (specifically layout)
            ChangeFlags::LAYOUT | ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }

    pub(crate) fn set_kind(&mut self, kind: BorderKind) -> ChangeFlags {
        if self.kind != kind {
            self.kind = kind;
            // TODO more sophisticated check for needed ChangeFlags (specifically layout)
            ChangeFlags::LAYOUT | ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }

    pub(crate) fn set_style(&mut self, style: Style) -> ChangeFlags {
        if style != self.style {
            self.style = style;
            ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }

    fn render_border(&self, cx: &mut PaintCx) {
        let style = self.style.patch(cx.override_style);
        cx.override_style = Style::default();
        let r = to_ratatui_rect(cx.rect());

        use Borders as B; // unfortunately not possible to wildcard import since it's not an enum...
        if r.width == 0 || r.height == 0 {
            return;
        }

        let buf = cx.terminal.current_buffer_mut();

        let mut draw = |x, y, symbol, style| {
            if buf.area.x + x < buf.area.width && buf.area.y + y < buf.area.height {
                buf.get_mut(x, y).set_symbol(symbol).set_style(style);
            }
        };

        // Voluntary extra task, find cases where a dot makes sense as well (like `TOP | LEFT`)...
        if r.width == 1 && r.height == 1 && self.borders.intersects(B::ALL_CORNERS) {
            draw(r.x, r.y, symbols::DOT, self.style);
            return;
        }

        // borders
        if self.borders.intersects(B::HORIZONTAL) {
            let start = if self.borders.intersects(B::LEFT_WITH_CORNERS) {
                r.x + 1
            } else {
                r.x
            };
            let end = if self.borders.intersects(B::RIGHT_WITH_CORNERS) {
                r.x + r.width - 1
            } else {
                r.x + r.width
            };
            if self.borders.contains(B::TOP) {
                for x in start..end {
                    draw(x, r.y, self.kind.symbols().horizontal, style);
                }
            }
            if self.borders.contains(B::BOTTOM) {
                for x in start..end {
                    draw(x, r.y + r.height - 1, self.kind.symbols().horizontal, style);
                }
            }
        }
        if self.borders.intersects(B::VERTICAL) {
            let start = if self.borders.intersects(B::TOP_WITH_CORNERS) {
                r.y + 1
            } else {
                r.y
            };
            let end = if self.borders.intersects(B::BOTTOM_WITH_CORNERS) {
                r.y + r.height - 1
            } else {
                r.y + r.height
            };
            if self.borders.contains(B::LEFT) {
                for y in start..end {
                    draw(r.x, y, self.kind.symbols().vertical, style);
                }
            }
            if self.borders.contains(B::RIGHT) {
                for y in start..end {
                    draw(r.x + r.width - 1, y, self.kind.symbols().vertical, style);
                }
            }
        }

        // corners
        if self.borders.contains(B::TOP_LEFT_CORNER) {
            draw(r.x, r.y, self.kind.symbols().top_left, style);
        }
        if self.borders.contains(B::BOTTOM_LEFT_CORNER) {
            let symbol = self.kind.symbols().bottom_left;
            draw(r.x, r.y + r.height - 1, symbol, style);
        }
        if self.borders.contains(B::BOTTOM_RIGHT_CORNER) {
            let symbol = self.kind.symbols().bottom_right;
            draw(r.x + r.width - 1, r.y + r.height - 1, symbol, style);
        }
        if self.borders.contains(B::TOP_RIGHT_CORNER) {
            let symbol = self.kind.symbols().top_right;
            draw(r.x + r.width - 1, r.y, symbol, style);
        }
    }
}

impl Widget for Border {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.render_border(cx);
        self.content.paint(cx)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.content.event(cx, event)
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let pad = |borders| {
            if self.borders.intersects(borders) {
                1.0
            } else {
                0.0
            }
        };
        let pad_left = pad(Borders::LEFT_WITH_CORNERS);
        let pad_right = pad(Borders::RIGHT_WITH_CORNERS);
        let pad_top = pad(Borders::TOP_WITH_CORNERS);
        let pad_bottom = pad(Borders::BOTTOM_WITH_CORNERS);
        let border_padding = Size::new(pad_left + pad_right, pad_top + pad_bottom);
        let content_size = self.content.layout(cx, &bc.shrink(border_padding));

        self.content.set_origin(cx, Point::new(pad_left, pad_top));
        bc.constrain(content_size + border_padding)
    }

    fn lifecycle(&mut self, cx: &mut super::core::LifeCycleCx, event: &super::LifeCycle) {
        self.content.lifecycle(cx, event);
    }
}
