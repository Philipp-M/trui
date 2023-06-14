use std::marker::PhantomData;

use crate::widget::{self, ChangeFlags};

use super::{Borders, Cx, Styleable, View, ViewMarker};
use ratatui::style::{Color, Style};
use xilem_core::MessageResult;

pub struct Border<T, A, V> {
    content: V,
    borders: Borders,
    phantom: PhantomData<fn() -> (T, A)>,
    border_style: Style,
}

impl<T, A, V> ViewMarker for Border<T, A, V> {}

impl<T, A, V> View<T, A> for Border<T, A, V>
where
    V: View<T, A>,
    V::Element: 'static,
{
    type State = (V::State, xilem_core::Id);

    type Element = widget::Border;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (child_id, state, element) = self.content.build(cx);
            (
                (state, child_id),
                widget::Border::new(element, self.borders, self.border_style),
            )
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        (state, child_id): &mut Self::State,
        element: &mut Self::Element,
    ) -> crate::widget::ChangeFlags {
        // could be a little bit less redundant...
        let mut changeflags = ChangeFlags::empty();
        if prev.borders != self.borders {
            changeflags |= element.set_borders(self.borders);
        }
        if prev.border_style != self.border_style {
            changeflags |= element.set_border_style(self.border_style);
        }
        let element = element
            .content
            .downcast_mut()
            .expect("The border content widget changed its type, this should never happen!");
        changeflags
            | cx.with_id(*id, |cx| {
                self.content
                    .rebuild(cx, &prev.content, child_id, state, element)
            })
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        (state, child_id): &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        if let Some((first, rest_path)) = id_path.split_first() {
            if first == child_id {
                self.content.message(rest_path, state, message, app_state)
            } else {
                xilem_core::MessageResult::Stale(message)
            }
        } else {
            xilem_core::MessageResult::Nop
        }
    }
}

impl<T, A, V> Styleable<T, A> for Border<T, A, V>
where
    V: View<T, A>,
    V::Element: 'static,
{
    type Output = Self;

    fn fg(mut self, color: Color) -> Self::Output {
        self.border_style.fg = Some(color);
        self
    }

    fn bg(mut self, color: Color) -> Self::Output {
        self.border_style.bg = Some(color);
        self
    }

    fn modifier(self, modifier: ratatui::style::Modifier) -> Self::Output {
        self.border_style.add_modifier(modifier);
        self
    }

    fn current_style(&self) -> Style {
        self.border_style
    }

    fn style(mut self, style: Style) -> Self::Output {
        self.border_style = style;
        self
    }
}

pub fn border<T, A, V>(content: V) -> Border<T, A, V> {
    Border {
        content,
        borders: Borders::ALL,
        phantom: PhantomData,
        border_style: Style::reset(),
    }
}
