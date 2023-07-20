use super::{common::Styleable, Cx, View, ViewMarker};
use crate::widget::{self, ChangeFlags, StyleableWidget};
use ratatui::style::{Color, Modifier, Style};
use std::marker::PhantomData;
use unicode_segmentation::UnicodeSegmentation;

impl ViewMarker for &str {}

impl<T, C, A> View<T, C, A> for &str {
    type State = ();

    type Element = widget::Text;

    fn build(&self, cx: &mut Cx<C>) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| widget::Text {
            text: String::from(*self),
            style: Style::default(),
        });
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx<C>,
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

impl<T, C, A> View<T, C, A> for String {
    type State = ();

    type Element = widget::Text;

    fn build(&self, cx: &mut Cx<C>) -> (xilem_core::Id, Self::State, Self::Element) {
        <&str as View<T, C, A>>::build(&self.as_str(), cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx<C>,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        <&str as View<T, C, A>>::rebuild(&self.as_str(), cx, &prev.as_str(), id, state, element)
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        <&str as View<T, C, A>>::message(&self.as_str(), id_path, state, message, app_state)
    }
}

impl<T, C, A> From<&str> for Text<T, C, A> {
    fn from(text: &str) -> Self {
        Text {
            text: text.into(),
            style: Style::default(),
            phantom: PhantomData,
        }
    }
}

impl<T, C, A> From<String> for Text<T, C, A> {
    fn from(text: String) -> Self {
        Text {
            text,
            style: Style::default(),
            phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Text<T, C, A = ()> {
    text: String,
    style: Style,
    // necessary for inference...
    phantom: PhantomData<fn() -> (T, C, A)>,
}

impl<T, C, A> ViewMarker for Text<T, C, A> {}

impl<T, C, A> View<T, C, A> for Text<T, C, A> {
    type State = ();

    type Element = widget::Text;

    fn build(&self, cx: &mut Cx<C>) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| widget::Text {
            text: self.text.clone(),
            style: self.style,
        });
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx<C>,
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

// TODO consider specialisation to avoid possibly unnecessary allocations (when e.g. using `Cow<str>` instead of `String`)
impl<T, C, A, S: Into<String>> Styleable<T, C, A> for S {
    type Output = Text<T, C, A>;

    fn fg(self, color: Color) -> Self::Output {
        Text::from(self.into()).fg(color)
    }

    fn bg(self, color: Color) -> Self::Output {
        Text::from(self.into()).bg(color)
    }

    fn modifier(self, modifier: Modifier) -> Self::Output {
        Text::from(self.into()).modifier(modifier)
    }

    fn style(self, style: Style) -> Self::Output {
        Text::from(self.into()).style(style)
    }

    fn current_style(&self) -> Style {
        Style::default()
    }
}

impl<T, C, A> Styleable<T, C, A> for Text<T, C, A> {
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

pub struct WrappedText<T, C, A> {
    words: Vec<(String, Style)>,
    phantom: PhantomData<fn() -> (T, C, A)>,
}

pub trait ToWrappedText<T, C, A> {
    fn wrapped(self) -> WrappedText<T, C, A>;
}

impl<S, C, A, T: Into<Text<S, C, A>>> ToWrappedText<S, C, A> for T {
    fn wrapped(self) -> WrappedText<S, C, A> {
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
impl<S, C, A, T1: Into<Text<S, C, A>>, T2: Into<Text<S, C, A>>> ToWrappedText<S, C, A>
    for (T1, T2)
{
    fn wrapped(self) -> WrappedText<S, C, A> {
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

impl<T, C, A> ViewMarker for WrappedText<T, C, A> {}

impl<T, C, A> View<T, C, A> for WrappedText<T, C, A> {
    type State = ();

    type Element = widget::WrappedText;

    fn build(&self, cx: &mut Cx<C>) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| widget::WrappedText::new(self.words.clone()));
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx<C>,
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
