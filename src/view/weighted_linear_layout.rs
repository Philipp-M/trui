use super::{Cx, View, ViewMarker, ViewSequence};
use crate::{
    geometry::Axis,
    widget::{self, ChangeFlags},
};
use std::{any::Any, marker::PhantomData};
use xilem_core::{Id, MessageResult, VecSplice};

pub struct WeightedLinearLayout<T, A, VT> {
    children: VT,
    axis: Axis,
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, VT> ViewMarker for WeightedLinearLayout<T, A, VT> {}

impl<T, A, VT: ViewSequence<T, A>> View<T, A> for WeightedLinearLayout<T, A, VT> {
    type State = VT::State;

    type Element = widget::WeightedLinearLayout;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut elements = vec![];
        let (id, state) = cx.with_new_id(|cx| self.children.build(cx, &mut elements));
        let column = widget::WeightedLinearLayout::new(elements, self.axis);
        (id, state, column)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut scratch = vec![];
        let mut splice = VecSplice::new(&mut element.children, &mut scratch);

        cx.with_id(*id, |cx| {
            self.children
                .rebuild(cx, &prev.children, state, &mut splice)
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        self.children.message(id_path, state, event, app_state)
    }
}

pub fn weighted_h_stack<T, A, VT: ViewSequence<T, A>>(
    children: VT,
) -> WeightedLinearLayout<T, A, VT> {
    WeightedLinearLayout {
        children,
        axis: Axis::Horizontal,
        phantom: PhantomData,
    }
}

pub fn weighted_v_stack<T, A, VT: ViewSequence<T, A>>(
    children: VT,
) -> WeightedLinearLayout<T, A, VT> {
    WeightedLinearLayout {
        children,
        axis: Axis::Vertical,
        phantom: PhantomData,
    }
}

pub struct WeightedLayoutElement<V, T, A> {
    pub(crate) content: V,
    pub(crate) weight: f64,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, V> ViewMarker for WeightedLayoutElement<V, T, A> {}

impl<T, A, V: View<T, A>> View<T, A> for WeightedLayoutElement<V, T, A> {
    type State = V::State;

    type Element = widget::WeightedLayoutElement;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, state, element) = self.content.build(cx);
        let element = widget::WeightedLayoutElement::new(element, self.weight);
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
        changeflags |= element.set_weight(self.weight);

        let element = element
            .content
            .downcast_mut()
            .expect("The weighted widget changed its type, this should never happen!");

        changeflags | self.content.rebuild(cx, &prev.content, id, state, element)
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

pub fn weighted<T, A, V: View<T, A>>(weight: f64, content: V) -> WeightedLayoutElement<V, T, A> {
    WeightedLayoutElement {
        content,
        weight,
        phantom: PhantomData,
    }
}
