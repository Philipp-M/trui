use super::{core::PaintCx, ChangeFlags, Event, EventCx, LayoutCx, Pod, Widget};
use crate::view::Borders;
use ratatui::{layout::Rect, style::Style, symbols};
use taffy::tree::NodeId;

pub fn render_border(cx: &mut PaintCx, r: Rect, borders: Borders, style: Style) {
    if r.width == 0 || r.height == 0 {
        return;
    }
    let buf = cx.terminal.current_buffer_mut();

    let mut draw = |x, y, s| {
        if buf.area.x + x < buf.area.width && buf.area.y + y < buf.area.height {
            buf.get_mut(x, y).set_symbol(s).set_style(style);
        }
    };
    if r.width == 1 && r.height == 1 {
        draw(r.x, r.y, symbols::DOT);
        return;
    }
    if r.width > 1 && borders.intersects(Borders::HORIZONTAL) {
        for x in r.x..(r.x + r.width) {
            if borders.contains(Borders::TOP) {
                draw(x, r.y, symbols::line::HORIZONTAL);
            }
            if borders.contains(Borders::BOTTOM) {
                draw(x, r.y + r.height - 1, symbols::line::HORIZONTAL);
            }
        }
    }
    if r.height > 1 && borders.intersects(Borders::VERTICAL) {
        for y in r.y..(r.y + r.height) {
            if borders.contains(Borders::LEFT) {
                draw(r.x, y, symbols::line::VERTICAL);
            }
            if borders.contains(Borders::RIGHT) {
                draw(r.x + r.width - 1, y, symbols::line::VERTICAL);
            }
        }
    }
    if r.width > 1 && r.height > 1 {
        if borders.contains(Borders::LEFT | Borders::TOP) {
            draw(r.x, r.y, symbols::line::ROUNDED_TOP_LEFT);
        }
        if borders.contains(Borders::LEFT | Borders::BOTTOM) {
            draw(r.x, r.y + r.height - 1, symbols::line::ROUNDED_BOTTOM_LEFT);
        }
        if borders.contains(Borders::RIGHT | Borders::BOTTOM) {
            draw(
                r.x + r.width - 1,
                r.y + r.height - 1,
                symbols::line::ROUNDED_BOTTOM_RIGHT,
            );
        }
        if borders.contains(Borders::RIGHT | Borders::TOP) {
            draw(r.x + r.width - 1, r.y, symbols::line::ROUNDED_TOP_RIGHT);
        }
    }
}

pub struct Border {
    pub(crate) content: Pod,
    borders: Borders,
    border_style: Style,
}

impl Border {
    pub fn new(content: impl Widget + 'static, borders: Borders, border_style: Style) -> Self {
        Border {
            content: Pod::new(content),
            borders,
            border_style,
        }
    }

    pub fn set_borders(&mut self, borders: Borders) -> ChangeFlags {
        self.borders = borders;
        ChangeFlags::LAYOUT | ChangeFlags::PAINT
    }
    pub fn set_border_style(&mut self, style: Style) -> ChangeFlags {
        self.border_style = style;
        ChangeFlags::PAINT
    }
}

impl Widget for Border {
    fn paint(&mut self, cx: &mut PaintCx, rect: Rect) {
        render_border(cx, rect, self.borders, self.border_style);
        self.content.paint(cx)
    }

    fn style(&mut self, cx: &mut super::StyleCx, _prev: NodeId) -> NodeId {
        let pad = |b| {
            taffy::style::LengthPercentage::Length(if self.borders.contains(b) { 1.0 } else { 0.0 })
        };

        let border_style = taffy::style::Style {
            padding: taffy::prelude::Rect {
                left: pad(Borders::LEFT),
                right: pad(Borders::RIGHT),
                top: pad(Borders::TOP),
                bottom: pad(Borders::BOTTOM),
            },
            size: taffy::prelude::Size {
                width: taffy::style::Dimension::Percent(1.0),
                height: taffy::style::Dimension::Percent(1.0),
            },
            ..Default::default()
        };

        // TODO diff children...
        let content = self.content.style(cx);
        cx.taffy
            .new_with_children(border_style, &[content])
            .unwrap()
    }

    fn layout(&mut self, cx: &mut LayoutCx, rect: Rect) {
        self.content.layout(cx, rect)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.content.event(cx, event)
    }
}
