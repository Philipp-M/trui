use std::borrow::Cow;

use ratatui::style::Style;
use taffy::tree::NodeId;
use unicode_width::UnicodeWidthStr;

use super::{
    core::{update_layout_node, EventCx},
    ChangeFlags, Event, LayoutCx, PaintCx, StyleableWidget, Widget,
};

pub struct Text {
    pub(crate) text: Cow<'static, str>,
    pub(crate) style: Style,
}

// TODO maybe a generic macro for stuff like below?
impl Text {
    pub fn set_text(&mut self, text: Cow<'static, str>) -> ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        if self.text != text {
            changeflags.set(ChangeFlags::LAYOUT, self.text.width() != text.width());
            changeflags |= ChangeFlags::PAINT;
            self.text = text;
        }
        changeflags
    }
}

impl StyleableWidget for Text {
    fn set_style(&mut self, style: Style) -> ChangeFlags {
        if style != self.style {
            self.style = style;
            ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }
}

impl Widget for Text {
    fn paint(&mut self, cx: &mut PaintCx) {
        let rect = cx.rect();

        let style = self.style.patch(cx.override_style);

        let term_size = cx.terminal.size().unwrap();

        let max_width = rect.width.min(term_size.width.saturating_sub(rect.x)) as usize;
        if rect.height > 0 && max_width > 0 && rect.y < term_size.height {
            cx.terminal
                .current_buffer_mut()
                .set_stringn(rect.x, rect.y, &self.text, max_width, style);
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
        let style = taffy::style::Style {
            min_size: taffy::prelude::Size {
                width: taffy::style::Dimension::Auto,
                height: taffy::style::Dimension::Length(1.0), // new lines seem to be ignored by the ratatui string functions
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

// TODO relatively hacky naive implementation of wrapping text via flexbox
pub struct WrappedText {
    pub(crate) words: Vec<(String, Style)>,
    pub(crate) words_layout: Vec<NodeId>,
    words_need_layout: bool, // TODO necessary?
    pub(crate) base_layout: taffy::style::Style,
}

impl WrappedText {
    pub(crate) fn new(words: Vec<(String, Style)>) -> Self {
        WrappedText {
            words,
            base_layout: taffy::style::Style {
                size: taffy::prelude::Size {
                    width: taffy::style::Dimension::Percent(1.0),
                    height: taffy::style::Dimension::Percent(1.0),
                },
                flex_wrap: taffy::style::FlexWrap::Wrap,
                align_content: Some(taffy::style::AlignContent::FlexStart),
                ..Default::default()
            },
            words_need_layout: true,
            words_layout: Vec::new(),
        }
    }

    pub fn set_words(&mut self, words: &Vec<(String, Style)>) -> ChangeFlags {
        if &self.words != words {
            self.words = words.clone();
            self.words_need_layout = true;
            ChangeFlags::PAINT | ChangeFlags::LAYOUT
        } else {
            ChangeFlags::empty()
        }
    }
}

impl Widget for WrappedText {
    fn paint(&mut self, cx: &mut PaintCx) {
        let rect = cx.rect();
        for ((word, style), node) in self.words.iter().zip(self.words_layout.iter()) {
            let layout = cx.taffy.layout(*node).unwrap();
            let x = rect.x + (layout.location.x as u16);
            let y = rect.y + (layout.location.y as u16);
            let term_size = cx.terminal.size().unwrap();

            let max_width = rect
                .width
                .saturating_sub(layout.location.x as u16)
                .min(term_size.width.saturating_sub(x)) as usize;
            if max_width > 0 && y < term_size.height {
                let style = style.patch(cx.override_style);
                cx.terminal
                    .current_buffer_mut()
                    .set_stringn(x, y, word, max_width, style);
            }
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
        if self.words_need_layout {
            self.words_need_layout = false;
            for n in &self.words_layout {
                cx.taffy.remove(*n).unwrap();
            }

            // TODO reuse memory?
            self.words_layout = self
                .words
                .iter()
                .map(|(word, _)| {
                    cx.taffy
                        .new_leaf(taffy::style::Style {
                            size: taffy::prelude::Size {
                                width: taffy::style::Dimension::Length(word.width() as f32),
                                height: taffy::style::Dimension::Length(1.0), // TODO multi line spacers?
                            },
                            ..Default::default()
                        })
                        .unwrap()
                })
                .collect();
        }
        if !prev.is_null() {
            update_layout_node(prev, cx.taffy, &self.words_layout, &self.base_layout);
            prev
        } else {
            cx.taffy
                .new_with_children(self.base_layout.clone(), &self.words_layout)
                .unwrap()
        }
    }

    fn event(&mut self, _cx: &mut EventCx, _event: &Event) {}
}
