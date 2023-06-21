use ratatui::style::Style;
use std::cmp::max;
use taffy::tree::NodeId;
use unicode_width::UnicodeWidthStr;

use super::{
    core::{update_layout_node, EventCx},
    ChangeFlags, Event, LayoutCx, PaintCx, StyleableWidget, Widget,
};

pub struct Text {
    pub(crate) text: String,
    pub(crate) style: Style,
}

// TODO maybe a generic macro for stuff like below?
impl Text {
    pub fn set_text(&mut self, text: &str) -> ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        if self.text != text {
            changeflags.set(ChangeFlags::LAYOUT, self.text.width() != text.width());
            changeflags |= ChangeFlags::PAINT;
            self.text = text.to_string();
        }
        changeflags
    }
}

impl StyleableWidget for Text {
    fn set_style(&mut self, style: Style) -> bool {
        let changed = style != self.style;
        if changed {
            self.style = style;
        }
        changed
    }
}

impl Widget for Text {
    fn paint(&mut self, cx: &mut PaintCx) {
        let (min_x, min_y) = self.text.lines().fold((0, 0), |(min_x, min_y), l| {
            (max(min_x, l.width()), min_y + 1)
        });
        let rect = cx.rect();

        let buf = cx.terminal.current_buffer_mut();
        // TODO multiline
        // TODO safe long strings...
        if rect.height >= 1
            && min_x <= rect.width as usize
            && min_y <= rect.height as usize
            && (rect.x + min_x as u16) < buf.area.width
            && (rect.y + min_y as u16) < buf.area.height
        {
            let style = self.style.patch(cx.override_style);

            cx.terminal
                .current_buffer_mut()
                .set_string(rect.x, rect.y, &self.text, style)
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
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
        if !prev.is_null() {
            update_layout_node(prev, cx.taffy, &[], &style);
            prev
        } else {
            cx.taffy.new_leaf(style).unwrap()
        }
    }

    fn event(&mut self, _cx: &mut EventCx, _event: &Event) {}
}
