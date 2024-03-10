use std::marker::PhantomData;

use xilem_core::{Id, MessageResult};

use crate::{
    widget::{self, ChangeFlags},
    Animatable, Cx, Fill, View, ViewMarker,
};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FillMaxSize<V, P, T, A> {
    pub(crate) content: V,
    // TODO making this animatable would be great too
    pub(crate) fill: Fill,
    pub(crate) percent: P,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, P, V> ViewMarker for FillMaxSize<V, P, T, A> {}

pub struct FillMaxSizeState<CS, PS> {
    content_state: CS,
    content_id: Id,
    percent_state: PS,
    percent_id: Id,
    // percent: f64,
}

impl<T, A, P: Animatable<f64>, V: View<T, A>> View<T, A> for FillMaxSize<V, P, T, A> {
    type State = FillMaxSizeState<V::State, P::State>;

    type Element = widget::FillMaxSize<P::Element>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (content_id, content_state, element) = self.content.build(cx);
            let (percent_id, percent_state, percent_element) = self.percent.build(cx);
            let element = widget::FillMaxSize::new(element, self.fill, percent_element);
            (
                FillMaxSizeState {
                    content_state,
                    content_id,
                    percent_state,
                    percent_id,
                },
                element,
            )
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> crate::widget::ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        changeflags |= element.set_fill(self.fill);
        cx.with_id(*id, |cx| {
            changeflags |= self.percent.rebuild(
                cx,
                &prev.percent,
                &mut state.percent_id,
                &mut state.percent_state,
                &mut element.percent,
            );

            let content_el = element
                .content
                .downcast_mut()
                .expect("The margin widget changed its type, this should never happen!");

            let content_changeflags = self.content.rebuild(
                cx,
                &prev.content,
                &mut state.content_id,
                &mut state.content_state,
                content_el,
            );

            changeflags | element.content.mark(content_changeflags)
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        match id_path {
            [id, rest_path @ ..] if *id == state.content_id => {
                self.content
                    .message(rest_path, &mut state.content_state, message, app_state)
            }
            [id, rest_path @ ..] if *id == state.percent_id => {
                match self
                    .percent
                    .message(rest_path, &mut state.percent_state, message)
                {
                    MessageResult::Action(_) | MessageResult::RequestRebuild => {
                        MessageResult::RequestRebuild
                    }
                    MessageResult::Nop => MessageResult::Nop,
                    MessageResult::Stale(message) => MessageResult::Stale(message),
                }
            }
            [..] => xilem_core::MessageResult::Stale(message),
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FillMaxSizeStyle<P> {
    pub fill: Fill,
    pub percent: P,
}

pub trait IntoFillMaxSizeStyle<An>: Sized
where
    An: Animatable<f64>,
{
    fn into(self) -> FillMaxSizeStyle<An>;
}

impl<P: Animatable<f64>> IntoFillMaxSizeStyle<P> for FillMaxSizeStyle<P> {
    fn into(self) -> FillMaxSizeStyle<P> {
        self
    }
}

impl<P: Animatable<f64>> IntoFillMaxSizeStyle<P> for P {
    fn into(self) -> FillMaxSizeStyle<P> {
        FillMaxSizeStyle {
            fill: Fill::ALL,
            percent: self,
        }
    }
}

impl IntoFillMaxSizeStyle<f64> for Fill {
    fn into(self) -> FillMaxSizeStyle<f64> {
        FillMaxSizeStyle {
            fill: self,
            percent: 1.0,
        }
    }
}

// TODO just use Default::default()?
impl IntoFillMaxSizeStyle<f64> for () {
    fn into(self) -> FillMaxSizeStyle<f64> {
        FillMaxSizeStyle {
            fill: Fill::ALL,
            percent: 1.0,
        }
    }
}
