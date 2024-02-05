use std::marker::PhantomData;

use xilem_core::MessageResult;

use crate::{
    widget::{self, ChangeFlags},
    Cx, Fill, View, ViewMarker,
};

pub struct FillMaxSize<V, T, A> {
    pub(crate) content: V,
    pub(crate) fill: Fill,
    pub(crate) percent: f64,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, V> ViewMarker for FillMaxSize<V, T, A> {}

impl<T, A, V: View<T, A>> View<T, A> for FillMaxSize<V, T, A> {
    type State = V::State;

    type Element = widget::FillMaxSize;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, state, element) = self.content.build(cx);
        let element = widget::FillMaxSize::new(element, self.fill, self.percent);
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
        changeflags |= element.set_fill(self.fill);
        changeflags |= element.set_percent(self.percent);

        let element = element
            .content
            .downcast_mut()
            .expect("The margin widget changed its type, this should never happen!");

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

pub struct FillMaxSizeStyle {
    pub fill: Fill,
    pub percent: f64,
}

impl From<()> for FillMaxSizeStyle {
    fn from((): ()) -> Self {
        Self {
            fill: Fill::ALL,
            percent: 1.0,
        }
    }
}

impl From<Fill> for FillMaxSizeStyle {
    fn from(fill: Fill) -> Self {
        Self { fill, percent: 1.0 }
    }
}

impl From<f64> for FillMaxSizeStyle {
    fn from(percent: f64) -> Self {
        Self {
            fill: Fill::ALL,
            percent,
        }
    }
}

impl From<(Fill, f64)> for FillMaxSizeStyle {
    fn from((fill, percent): (Fill, f64)) -> Self {
        Self { fill, percent }
    }
}

impl From<(f64, Fill)> for FillMaxSizeStyle {
    fn from((percent, fill): (f64, Fill)) -> Self {
        Self { fill, percent }
    }
}
