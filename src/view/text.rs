use super::{common::Styleable, Cx, View, ViewMarker};
use crate::widget::{self, ChangeFlags, StyleableWidget};
use ratatui::style::{Color, Modifier, Style};
use std::marker::PhantomData;
use unicode_segmentation::UnicodeSegmentation;

impl ViewMarker for &str {}

impl<T, A> View<T, A> for &str {
    type State = ();

    type Element = widget::Text;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| widget::Text {
            text: String::from(*self),
            style: Style::default(),
        });
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
        element.set_text(self)
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

impl ViewMarker for String {}

impl<T, A> View<T, A> for String {
    type State = ();

    type Element = widget::Text;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        <&str as View<T>>::build(&self.as_str(), cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        <&str as View<T>>::rebuild(&self.as_str(), cx, &prev.as_str(), id, state, element)
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        <&str as View<T, A>>::message(&self.as_str(), id_path, state, message, app_state)
    }
}

impl<T, A> From<&str> for Text<T, A> {
    fn from(text: &str) -> Self {
        Text {
            text: text.into(),
            style: Style::default(),
            phantom: PhantomData,
        }
    }
}

impl<T, A> From<String> for Text<T, A> {
    fn from(text: String) -> Self {
        Text {
            text,
            style: Style::default(),
            phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Text<T = (), A = ()> {
    text: String,
    style: Style,
    // necessary for inference...
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A> ViewMarker for Text<T, A> {}

impl<T, A> View<T, A> for Text<T, A> {
    type State = ();

    type Element = widget::Text;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| widget::Text {
            text: self.text.clone(),
            style: self.style,
        });
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
        let mut changeflags = element.set_text(&self.text);
        if element.set_style(self.style) {
            changeflags |= ChangeFlags::PAINT;
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

impl<T, A, S: Into<String>> Styleable<T, A> for S {
    type Output = Text<T, A>;

    fn fg(self, color: Color) -> Self::Output {
        <Text<T, A> as Styleable<T, A>>::fg(Text::from(self.into()), color)
    }

    fn bg(self, color: Color) -> Self::Output {
        <Text<T, A> as Styleable<T, A>>::bg(Text::from(self.into()), color)
    }

    fn modifier(self, modifier: Modifier) -> Self::Output {
        <Text<T, A> as Styleable<T, A>>::modifier(Text::from(self.into()), modifier)
    }

    fn style(self, style: Style) -> Self::Output {
        <Text<T, A> as Styleable<T, A>>::style(Text::from(self.into()), style)
    }

    fn current_style(&self) -> Style {
        Style::default()
    }
}

impl<T, A> Styleable<T, A> for Text<T, A> {
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

pub struct WrappedText<T, A> {
    words: Vec<(String, Style)>,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub trait ToWrappedText<T, A> {
    fn wrapped(self) -> WrappedText<T, A>;
}

impl<S, A, T: Into<Text<S, A>>> ToWrappedText<S, A> for T {
    fn wrapped(self) -> WrappedText<S, A> {
        let text = self.into();
        WrappedText {
            words: text
                .text
                .split_word_bounds()
                .map(|s| (s.into(), text.style))
                .collect(),
            phantom: PhantomData,
        }
        // WrappedText { text: vec![self.into()] }
    }
}

// TODO maybe extend this for bigger tuples as well with a macro...
impl<S, A, T1: Into<Text<S, A>>, T2: Into<Text<S, A>>> ToWrappedText<S, A> for (T1, T2) {
    fn wrapped(self) -> WrappedText<S, A> {
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
        WrappedText {
            words,
            phantom: PhantomData,
        }
    }
}

impl<T, A> ViewMarker for WrappedText<T, A> {}

impl<T, A> View<T, A> for WrappedText<T, A> {
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
