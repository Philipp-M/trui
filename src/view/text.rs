use std::borrow::Cow;

use super::{common::Styleable, Cx, View, ViewMarker};
use crate::widget::{self, ChangeFlags};
use ratatui::style::{Color, Modifier, Style};
use unicode_segmentation::UnicodeSegmentation;

impl From<&'static str> for Text {
    fn from(text: &'static str) -> Self {
        Text {
            text: text.into(),
            style: Style::default(),
        }
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Text {
            text: text.into(),
            style: Style::default(),
        }
    }
}

impl From<Cow<'static, str>> for Text {
    fn from(text: Cow<'static, str>) -> Self {
        Text {
            text,
            style: Style::default(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Text {
    text: Cow<'static, str>,
    style: Style,
}

impl<T: Into<Text>> ViewMarker for T {}

impl<T, A, S: Into<Text> + Clone + Send + Sync + Eq> View<T, A> for S {
    type State = ();

    type Element = widget::Text;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let text = self.clone().into();
        let (id, element) = cx.with_new_id(|_| widget::Text {
            text: text.text,
            style: text.style,
        });
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut xilem_core::Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        if self != prev {
            let text = self.clone().into();
            changeflags |= element.set_text(text.text.clone());
            changeflags |= element.set_style(text.style);
        }
        changeflags
    }

    fn message(
        &self,
        _id_path: &[xilem_core::Id],
        _state: &mut Self::State,
        _message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        xilem_core::MessageResult::Nop
    }
}

#[macro_export]
macro_rules! impl_styleable_text_views {
    ($($ty:tt)*) => {
        impl Styleable for $($ty)* {
            type Output = Text;

            fn fg(self, color: Color) -> Self::Output {
                Text::from(self).fg(color)
            }

            fn bg(self, color: Color) -> Self::Output {
                Text::from(self).bg(color)
            }

            fn modifier(self, modifier: Modifier) -> Self::Output {
                Text::from(self).modifier(modifier)
            }

            fn style(self, style: Style) -> Self::Output {
                Text::from(self).style(style)
            }

            fn current_style(&self) -> Style {
                Style::default()
            }
        }
    };
}

impl_styleable_text_views!(&'static str);
impl_styleable_text_views!(String);
impl_styleable_text_views!(Cow<'static, str>);

impl Styleable for Text {
    type Output = Self;

    fn fg(mut self, color: Color) -> Self::Output {
        self.style.fg = Some(color);
        self
    }

    fn bg(mut self, color: Color) -> Self::Output {
        self.style.bg = Some(color);
        self
    }

    fn modifier(mut self, modifier: Modifier) -> Self::Output {
        self.style = self.style.add_modifier(modifier);
        self
    }

    fn style(mut self, style: Style) -> Self::Output {
        self.style = style;
        self
    }

    fn current_style(&self) -> Style {
        self.style
    }
}

pub struct WrappedText {
    words: Vec<(String, Style)>,
}

pub trait ToWrappedText {
    fn wrapped(self) -> WrappedText;
}

impl<T: Into<Text>> ToWrappedText for T {
    fn wrapped(self) -> WrappedText {
        let text = self.into();
        WrappedText {
            words: text
                .text
                .split_word_bounds()
                .map(|s| (s.into(), text.style))
                .collect(),
        }
        // WrappedText { text: vec![self.into()] }
    }
}

// TODO maybe extend this for bigger tuples as well with a macro...
impl<T1: Into<Text>, T2: Into<Text>> ToWrappedText for (T1, T2) {
    fn wrapped(self) -> WrappedText {
        let mut words = Vec::new();
        let text = self.0.into();
        for w in text
            .text
            .split_word_bounds()
            .map(|s| (s.into(), text.style))
        {
            words.push(w);
        }
        let text = self.1.into();
        for w in text
            .text
            .split_word_bounds()
            .map(|s| (s.into(), text.style))
        {
            words.push(w);
        }
        WrappedText { words }
    }
}

impl ViewMarker for WrappedText {}

impl<T, A> View<T, A> for WrappedText {
    type State = ();

    type Element = widget::WrappedText;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| widget::WrappedText::new(self.words.clone()));
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut xilem_core::Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        element.set_words(&self.words)
    }

    fn message(
        &self,
        _id_path: &[xilem_core::Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        xilem_core::MessageResult::Stale(message)
    }
}
