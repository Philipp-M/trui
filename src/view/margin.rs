use std::marker::PhantomData;

use xilem_core::MessageResult;

use crate::{
    widget::{self, ChangeFlags},
    Cx, Position, View, ViewMarker,
};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Margin<V, T, A> {
    pub(crate) content: V,
    pub(crate) amount: u16,
    pub(crate) position: Position,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, V> ViewMarker for Margin<V, T, A> {}

impl<T, A, V: View<T, A>> View<T, A> for Margin<V, T, A> {
    type State = V::State;

    type Element = widget::Margin;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, state, element) = self.content.build(cx);
        let element = widget::Margin::new(element, self.position, self.amount);
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> crate::widget::ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        changeflags |= element.set_amount(self.amount);
        changeflags |= element.set_position(self.position);

        let content_el = element
            .content
            .downcast_mut()
            .expect("The margin widget changed its type, this should never happen!");

        let content_changeflags = self
            .content
            .rebuild(cx, &prev.content, id, state, content_el);
        changeflags | element.content.mark(content_changeflags)
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.content.message(id_path, state, message, app_state)
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MarginStyle {
    pub amount: u16,
    pub position: Position,
}

impl From<u16> for MarginStyle {
    fn from(amount: u16) -> Self {
        Self {
            amount,
            position: Position::ALL,
        }
    }
}

impl From<(u16, Position)> for MarginStyle {
    fn from((amount, position): (u16, Position)) -> Self {
        Self { amount, position }
    }
}

impl From<(Position, u16)> for MarginStyle {
    fn from((position, amount): (Position, u16)) -> Self {
        Self { amount, position }
    }
}
