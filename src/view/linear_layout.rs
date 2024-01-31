use super::{Cx, View, ViewMarker, ViewSequence};
use crate::widget::{self, ChangeFlags};
use std::{any::Any, marker::PhantomData};
use taffy::style::FlexDirection;
use xilem_core::{Id, VecSplice};

pub struct LinearLayout<T, A, VT> {
    children: VT,
    direction: FlexDirection,
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, VT> ViewMarker for LinearLayout<T, A, VT> {}

impl<T, A, VT: ViewSequence<T, A>> View<T, A> for LinearLayout<T, A, VT> {
    type State = VT::State;

    type Element = widget::LinearLayout;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut elements = vec![];
        let (id, state) = cx.with_new_id(|cx| self.children.build(cx, &mut elements));
        let column = widget::LinearLayout::new(elements, self.direction);
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

pub fn h_stack<T, A, VT: ViewSequence<T, A>>(children: VT) -> LinearLayout<T, A, VT> {
    LinearLayout {
        children,
        direction: FlexDirection::Row,
        phantom: PhantomData,
    }
}

pub fn v_stack<T, A, VT: ViewSequence<T, A>>(children: VT) -> LinearLayout<T, A, VT> {
    LinearLayout {
        children,
        direction: FlexDirection::Column,
        phantom: PhantomData,
    }
}
