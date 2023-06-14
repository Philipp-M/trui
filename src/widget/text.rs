use ratatui::{layout::Rect, style::Style};
use std::cmp::max;
use taffy::tree::NodeId;
use unicode_width::UnicodeWidthStr;

use super::{core::EventCx, ChangeFlags, Event, LayoutCx, PaintCx, StyleCx, Widget};

pub struct Text {
    pub(crate) text: String,
    pub(crate) rect: Rect,
    pub(crate) style: Style,
}

// TODO maybe a generic macro for stuff like below?
impl Text {
    pub fn set_text(&mut self, text: String) -> ChangeFlags {
        self.text = text;
        // TODO layout only, if width is different...
        ChangeFlags::LAYOUT | ChangeFlags::PAINT
    }
    pub fn set_style(&mut self, style: Style) -> ChangeFlags {
        self.style = style;
        ChangeFlags::PAINT
    }
}

impl Widget for Text {
    fn paint(&mut self, cx: &mut PaintCx, rect: Rect) {
        let (min_x, min_y) = self.text.lines().fold((0, 0), |(min_x, min_y), l| {
            (max(min_x, l.width()), min_y + 1)
        });

        let buf = cx.terminal.current_buffer_mut();
        // TODO multiline
        // TODO safe long strings...
        if rect.height >= 1
            && min_x <= rect.width as usize
            && min_y <= rect.height as usize
            && (rect.x + min_x as u16) < buf.area.width
            && (rect.y + min_y as u16) < buf.area.height
        {
            cx.terminal
                .current_buffer_mut()
                .set_string(rect.x, rect.y, &self.text, self.style)
        }
    }

    fn style(&mut self, cx: &mut StyleCx, prev: NodeId) -> NodeId {
        let (min_x, min_y) = self.text.lines().fold((0, 0), |(min_x, min_y), l| {
            (max(min_x, l.width()), min_y + 1)
        });
        let style = taffy::style::Style {
            min_size: taffy::prelude::Size {
                width: taffy::style::Dimension::Length(min_x as f32),
                height: taffy::style::Dimension::Length(min_y as f32),
            },
            size: taffy::prelude::Size {
                width: taffy::style::Dimension::Percent(1.0),
                height: taffy::style::Dimension::Percent(1.0),
            },
            ..Default::default()
        };
        if !cx.taffy.contains(prev) {
            cx.taffy.new_leaf(style).unwrap()
        } else {
            cx.taffy.set_style(prev, style).unwrap();
            prev
        }
    }

    fn event(&mut self, _cx: &mut EventCx, _event: &Event) {}

    fn layout(&mut self, _cx: &mut LayoutCx, rect: Rect) {
        self.rect = rect;
    }
}
