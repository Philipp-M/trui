use crate::{
    widget::{BoxConstraints, Widget},
    ChangeFlags, Color, Cx, ElementsSplice, Modifier, Style, Styleable, View, ViewMarker,
    ViewSequence,
};

macro_rules! one_of_view {
    (
        #[doc = $first_doc_line:literal]
        $ident:ident { $( $vars:ident ),+ }
    ) => {
        #[doc = $first_doc_line]
        ///
        /// It is a statically-typed alternative to the type-erased `AnyView`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
        pub enum $ident<$($vars),+> {
            $($vars($vars),)+
        }

        impl<$($vars),+> ViewMarker for $ident<$($vars),+> {}

        impl<$($vars: Widget),+> Widget for $ident<$($vars),+> {
            fn paint(&mut self, cx: &mut crate::widget::PaintCx) {
                match self {
                    $($ident::$vars(el) => el.paint(cx),)+
                }
            }

            fn layout(&mut self, cx: &mut crate::widget::LayoutCx, bc: &BoxConstraints) -> kurbo::Size {
                match self {
                    $($ident::$vars(el) => el.layout(cx, bc),)+
                }
            }

            fn event(&mut self, cx: &mut crate::widget::EventCx, event: &crate::widget::Event) {
                match self {
                    $($ident::$vars(el) => el.event(cx, event),)+
                }
            }

            fn lifecycle(&mut self, cx: &mut crate::widget::LifeCycleCx, event: &crate::widget::LifeCycle) {
                match self {
                    $($ident::$vars(el) => el.lifecycle(cx, event),)+
                }
            }
        }

        impl<VT, VA, $($vars),+> View<VT, VA> for $ident<$($vars),+>
        where
            $($vars: View<VT, VA>,)+
        {
            type State = $ident<$($vars::State),+>;
            type Element = $ident<$($vars::Element),+>;

            fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
                match self {
                    $(
                        $ident::$vars(view) => {
                            let (id, state, el) = view.build(cx);
                            (id, $ident::$vars(state), $ident::$vars(el))
                        }
                    )+
                }
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                id: &mut xilem_core::Id,
                state: &mut Self::State,
                element: &mut Self::Element,
            ) -> ChangeFlags {
                match (prev, self) {
                    $(
                        // Variant is the same as before
                        ($ident::$vars(prev_view), $ident::$vars(view)) => {
                            let ($ident::$vars(state), $ident::$vars(element)) = (state, element)
                            else {
                                unreachable!(concat!("invalid state/view in ", stringify!($ident)));
                            };
                            view.rebuild(cx, prev_view, id, state, element)
                        }
                        // Variant has changed
                        (_, $ident::$vars(view)) => {
                            let (new_id, new_state, new_element) = view.build(cx);
                            *id = new_id;
                            *state = $ident::$vars(new_state);
                            *element = $ident::$vars(new_element);
                            ChangeFlags::tree_structure()
                        }
                    )+
                }
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut VT,
            ) -> xilem_core::MessageResult<VA> {
                match self {
                    $(
                        $ident::$vars(view) => {
                            let $ident::$vars(state) = state else {
                                unreachable!(concat!("invalid state/view in", stringify!($ident)));
                            };
                            view.message(id_path, state, message, app_state)
                        }
                    )+
                }
            }
        }

        impl<$($vars: Styleable),+> Styleable for $ident<$($vars),+> {
            type Output = $ident<$($vars::Output),+>;

            fn fg(self, color: Color) -> Self::Output {
                match self {
                    $($ident::$vars(el) => $ident::$vars(el.fg(color)),)+
                }
            }

            fn bg(self, color: Color) -> Self::Output {
                match self {
                    $($ident::$vars(el) => $ident::$vars(el.bg(color)),)+
                }
            }

            fn style(self, style: Style) -> Self::Output {
                match self {
                    $($ident::$vars(el) => $ident::$vars(el.style(style)),)+
                }
            }

            fn modifier(self, modifier: Modifier) -> Self::Output {
                match self {
                    $($ident::$vars(el) => $ident::$vars(el.modifier(modifier)),)+
                }
            }

            fn current_style(&self) -> Style {
                match self {
                    $($ident::$vars(el) => el.current_style(),)+
                }
            }
        }
    };
}

one_of_view! {
    /// This view container can switch between two views.
    OneOf2 { A, B }
}
one_of_view! {
    /// This view container can switch between three views.
    OneOf3 { A, B, C }
}

one_of_view! {
    /// This view container can switch between four views.
    OneOf4 { A, B, C, D }
}

one_of_view! {
    /// This view container can switch between five views.
    OneOf5 { A, B, C, D, E }
}

one_of_view! {
    /// This view container can switch between six views.
    OneOf6 { A, B, C, D, E, F }
}

one_of_view! {
    /// This view container can switch between seven views.
    OneOf7 { A, B, C, D, E, F, G }
}

one_of_view! {
    /// This view container can switch between eight views.
    OneOf8 { A, B, C, D, E, F, G, H }
}

macro_rules! one_of_sequence {
    (
        #[doc = $first_doc_line:literal]
        $ident:ident { $( $vars:ident ),+ }
    ) => {
        #[doc = $first_doc_line]
        ///
        /// It is a statically-typed alternative to the type-erased `AnyView`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
        pub enum $ident<$($vars),+> {
            $($vars($vars),)+
        }
        impl<VT, VA, $($vars),+> ViewSequence<VT, VA> for $ident<$($vars),+>
        where $(
            $vars: ViewSequence<VT, VA>,
        )+ {
            type State = $ident<$($vars::State),+>;

            fn build(&self, cx: &mut Cx, elements: &mut dyn ElementsSplice) -> Self::State {
                match self {
                    $(
                        $ident::$vars(view_sequence) => {
                            $ident::$vars(view_sequence.build(cx, elements))
                        }
                    )+
                }
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                state: &mut Self::State,
                elements: &mut dyn ElementsSplice,
            ) -> ChangeFlags {
                match (prev, self) {
                    $(
                        // Variant is the same as before
                        ($ident::$vars(prev_view), $ident::$vars(view_sequence)) => {
                            let $ident::$vars(state) = state else {
                                unreachable!(concat!("invalid state/view_sequence in ", stringify!($ident)));
                            };
                            view_sequence.rebuild(cx, prev_view, state, elements)
                        }
                        // Variant has changed
                        (_, $ident::$vars(view_sequence)) => {
                            let new_state = view_sequence.build(cx, elements);
                            *state = $ident::$vars(new_state);
                            ChangeFlags::tree_structure()
                        }
                    )+
                }
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut VT,
            ) -> xilem_core::MessageResult<VA> {
                match self {
                    $(
                        $ident::$vars(view_sequence) => {
                            let $ident::$vars(state) = state else {
                                unreachable!(concat!("invalid state/view_sequence in ", stringify!($ident)));
                            };
                            view_sequence.message(id_path, state, message, app_state)
                        }
                    )+
                }
            }

            fn count(&self, state: &Self::State) -> usize {
                match self {
                    $(
                        $ident::$vars(view_sequence) => {
                            let $ident::$vars(state) = state else {
                                unreachable!(concat!("invalid state/view_sequence in ", stringify!($ident)));
                            };
                            view_sequence.count(state)
                        }
                    )+
                }
            }
        }
    };
}

one_of_sequence! {
    /// This view sequence container can switch between two view sequences.
    OneSeqOf2 { A, B }
}
one_of_sequence! {
    /// This view sequence container can switch between three view sequences.
    OneSeqOf3 { A, B, C }
}

one_of_sequence! {
    /// This view sequence container can switch between four view sequences.
    OneSeqOf4 { A, B, C, D }
}

one_of_sequence! {
    /// This view sequence container can switch between five view sequences.
    OneSeqOf5 { A, B, C, D, E }
}

one_of_sequence! {
    /// This view sequence container can switch between six view sequences.
    OneSeqOf6 { A, B, C, D, E, F }
}

one_of_sequence! {
    /// This view sequence container can switch between seven view sequences.
    OneSeqOf7 { A, B, C, D, E, F, G }
}

one_of_sequence! {
    /// This view sequence container can switch between eight view sequences.
    OneSeqOf8 { A, B, C, D, E, F, G, H }
}
