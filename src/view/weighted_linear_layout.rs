use super::{Cx, View, ViewMarker, ViewSequence};
use crate::{
    geometry::Axis,
    widget::{self, ChangeFlags},
    Animatable,
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

pub struct WeightedLayoutElement<V, W, T, A> {
    pub(crate) content: V,
    pub(crate) weight: W,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, V, W> ViewMarker for WeightedLayoutElement<V, W, T, A> {}

pub struct WeightedLayoutElementState<CS, WS> {
    content_state: CS,
    weight_state: WS,
    content_id: Id,
    weight_id: Id,
    weight: f64,
}

impl<T, A, V: View<T, A>, W: Animatable<f64>> View<T, A> for WeightedLayoutElement<V, W, T, A> {
    type State = WeightedLayoutElementState<V::State, W::State>;

    type Element = widget::WeightedLayoutElement;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (content_id, content_state, element) = self.content.build(cx);
            let (weight_id, weight_state, weight) = self.weight.build(cx);
            let element = widget::WeightedLayoutElement::new(element, weight);
            let state = WeightedLayoutElementState {
                content_state,
                weight_state,
                content_id,
                weight_id,
                weight,
            };
            (state, element)
        });
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
        cx.with_id(*id, |cx| {
            let mut changeflags = ChangeFlags::empty();
            let weight_updated = self
                .weight
                .rebuild(
                    cx,
                    &prev.weight,
                    &mut state.weight_id,
                    &mut state.weight_state,
                    &mut state.weight,
                )
                .contains(ChangeFlags::UPDATE);

            if weight_updated {
                changeflags |= element.set_weight(state.weight);
            }

            let element = element
                .content
                .downcast_mut()
                .expect("The weighted widget changed its type, this should never happen!");

            changeflags
                | self.content.rebuild(
                    cx,
                    &prev.content,
                    &mut state.content_id,
                    &mut state.content_state,
                    element,
                )
        })
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        match id_path {
            [id, rest_path @ ..] if *id == state.content_id => {
                self.content
                    .message(rest_path, &mut state.content_state, message, app_state)
            }
            [id, rest_path @ ..] if *id == state.weight_id => {
                match self
                    .weight
                    .message(rest_path, &mut state.weight_state, message)
                {
                    MessageResult::Action(_) | MessageResult::RequestRebuild => {
                        MessageResult::RequestRebuild
                    }
                    MessageResult::Nop => MessageResult::Nop,
                    MessageResult::Stale(message) => MessageResult::Stale(message),
                }
            }
            [..] => MessageResult::Stale(message),
        }
        // self.content.message(id_path, state, message, app_state)
    }
}

pub fn weighted<T, A, V: View<T, A>, W: Animatable<f64>>(
    weight: W,
    content: V,
) -> WeightedLayoutElement<V, W, T, A> {
    WeightedLayoutElement {
        content,
        weight,
        phantom: PhantomData,
    }
}
