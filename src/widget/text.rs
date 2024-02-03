use std::borrow::Cow;

use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::geometry::{to_ratatui_rect, Size};

use super::{
    core::EventCx, BoxConstraints, ChangeFlags, Event, LayoutCx, PaintCx, StyleableWidget, Widget,
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
        let rect = to_ratatui_rect(cx.rect());

        let style = self.style.patch(cx.override_style);

        let term_size = cx.terminal.size().unwrap();

        let max_width = rect.width.min(term_size.width.saturating_sub(rect.x)) as usize;
        if rect.height > 0 && max_width > 0 && rect.y < term_size.height {
            // TODO cut the text off, when it is out of bounds (rect.height is not respected)
            // likely with a custom implementation to render the text, instead of `set_stringn`
            cx.terminal
                .current_buffer_mut()
                .set_stringn(rect.x, rect.y, &self.text, max_width, style);
        }
    }

    fn layout(&mut self, _cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let mut width = 0;
        let mut height = 0;

        for l in self.text.lines() {
            width = width.max(l.width());
            height += 1;
        }

        bc.constrain(Size {
            width: width as f64,
            height: height as f64,
        })
    }

    fn event(&mut self, _cx: &mut EventCx, _event: &Event) {}

    fn lifecycle(&mut self, _cx: &mut super::core::LifeCycleCx, _event: &super::LifeCycle) {}
}

// TODO relatively hacky naive implementation of wrapping text via flexbox
pub struct WrappedText {
    pub(crate) words: Vec<(String, Style)>,
    // pub(crate) words_layout: Vec<NodeId>,
    words_need_layout: bool, // TODO necessary?
}

impl WrappedText {
    pub(crate) fn new(words: Vec<(String, Style)>) -> Self {
        WrappedText {
            words,
            words_need_layout: true,
            // words_layout: Vec::new(),
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
    fn paint(&mut self, _cx: &mut PaintCx) {
        // let rect = cx.rect();
        // for ((word, style), node) in self.words.iter().zip(self.words_layout.iter()) {
        //     let layout = cx.taffy.layout(*node).unwrap();
        //     let x = rect.x + (layout.location.x as u16);
        //     let y = rect.y + (layout.location.y as u16);
        //     let term_size = cx.terminal.size().unwrap();

        //     let max_width = rect
        //         .width
        //         .saturating_sub(layout.location.x as u16)
        //         .min(term_size.width.saturating_sub(x)) as usize;
        //     if max_width > 0 && y < term_size.height {
        //         let style = style.patch(cx.override_style);
        //         cx.terminal
        //             .current_buffer_mut()
        //             .set_stringn(x, y, word, max_width, style);
        //     }
        // }
        todo!()
    }

    fn layout(&mut self, _cx: &mut LayoutCx, _bc: &BoxConstraints) -> Size {
        todo!()
    }

    fn event(&mut self, _cx: &mut EventCx, _event: &Event) {}

    fn lifecycle(&mut self, _cx: &mut super::core::LifeCycleCx, _event: &super::LifeCycle) {}
}
