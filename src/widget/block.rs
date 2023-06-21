use super::{
    core::{update_layout_node, PaintCx},
    ChangeFlags, Event, EventCx, Pod, StyleableWidget, Widget,
};
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

fn fill_block(cx: &mut PaintCx, r: Rect, style: Style) {
    let buf = cx.terminal.current_buffer_mut();

    for x in r.x..(buf.area.width.min(r.width + r.x)) {
        for y in r.y..(buf.area.height.min(r.height + r.y)) {
            buf.get_mut(x, y).set_style(style);
        }
    }
}

pub struct Block {
    pub(crate) content: Pod,
    borders: Borders,
    border_style: Style,
    layout_style: taffy::style::Style,
    fill_with_bg: bool,
    inherit_style: bool,
}

impl Block {
    pub fn new(
        content: impl Widget + 'static,
        borders: Borders,
        border_style: Style,
        inherit_style: bool,
    ) -> Self {
        let pad =
            |b| taffy::style::LengthPercentage::Length(if borders.contains(b) { 1.0 } else { 0.0 });
        Block {
            content: Pod::new(content),
            borders,
            fill_with_bg: true,
            border_style,
            inherit_style,
            layout_style: taffy::style::Style {
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
            },
        }
    }

    pub fn set_borders(&mut self, borders: Borders) -> ChangeFlags {
        if self.borders != borders {
            self.borders = borders;
            ChangeFlags::LAYOUT | ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }

    pub fn set_inherit_style(&mut self, inherit: bool) -> ChangeFlags {
        if self.inherit_style != inherit {
            self.inherit_style = inherit;
            ChangeFlags::LAYOUT | ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }
}

impl StyleableWidget for Block {
    fn set_style(&mut self, style: Style) -> bool {
        let changed = style != self.border_style;
        if changed {
            self.border_style = style;
        }
        changed
    }
}

impl Widget for Block {
    fn paint(&mut self, cx: &mut PaintCx) {
        let style = self.border_style.patch(cx.override_style);
        cx.override_style = if self.inherit_style {
            style
        } else {
            Style::default()
        };

        if self.fill_with_bg {
            let fill_style = Style {
                bg: style.bg,
                ..Default::default()
            };
            fill_block(cx, cx.rect(), fill_style);
        }

        render_border(cx, cx.rect(), self.borders, style);

        self.content.paint(cx, cx.rect())
    }

    fn layout(&mut self, cx: &mut super::LayoutCx, prev: NodeId) -> NodeId {
        let content = self.content.layout(cx);
        if !prev.is_null() {
            update_layout_node(prev, cx.taffy, &[content], &self.layout_style);
            prev
        } else {
            cx.taffy
                .new_with_children(self.layout_style.clone(), &[content])
                .unwrap()
        }
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.content.event(cx, event)
    }
}
