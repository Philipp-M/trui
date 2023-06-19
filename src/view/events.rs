use super::{Cx, Styleable, View, ViewMarker};
use crate::widget::{self, ChangeFlags, StyleableWidget};
use ratatui::style::Style;
use std::marker::PhantomData;
use xilem_core::Id;

pub trait Hoverable<T, A, V, C: Fn(&mut T) -> A + Send> {
    fn on_hover(self, callback: C) -> OnHover<T, A, V, C>;
    fn on_blur_hover(self, callback: C) -> OnHoverLost<T, A, V, C>;
}

impl<T, A, V, C> Hoverable<T, A, V, C> for V
where
    V: View<T, A>,
    C: Fn(&mut T) -> A + Send,
{
    fn on_hover(self, callback: C) -> OnHover<T, A, V, C> {
        OnHover {
            view: self,
            callback,
            phantom: PhantomData,
        }
    }

    fn on_blur_hover(self, callback: C) -> OnHoverLost<T, A, V, C> {
        OnHoverLost {
            view: self,
            callback,
            phantom: PhantomData,
        }
    }
}

pub trait Clickable<T, A, V, C: Fn(&mut T) -> A + Send> {
    fn on_click(self, callback: C) -> OnClick<T, A, V, C>;
}

impl<T, A, V, C> Clickable<T, A, V, C> for V
where
    V: View<T, A>,
    C: Fn(&mut T) -> A + Send,
{
    fn on_click(self, callback: C) -> OnClick<T, A, V, C> {
        OnClick {
            view: self,
            callback,
            phantom: PhantomData,
        }
    }
}

pub struct StyleOnHover<T, A, V> {
    view: V,
    style: Style,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub trait HoverStyleable<T, A, V: View<T, A>> {
    fn on_hover_style(self, style: Style) -> StyleOnHover<T, A, V>;
}

impl<T, A, VS, V> HoverStyleable<T, A, V> for V
where
    VS: View<T, A>,
    V: View<T, A> + Styleable<T, A, Output = VS>,
{
    fn on_hover_style(self, style: Style) -> StyleOnHover<T, A, V> {
        StyleOnHover {
            view: self,
            style,
            phantom: PhantomData,
        }
    }
}

impl<T, A, V> ViewMarker for StyleOnHover<T, A, V> {}

// TODO is "invisible" (i.e. without id) a good idea,
// it never should receive events (or other things) directly and is just a trait on top of any *actual* view?
impl<T, A, VS, V> View<T, A> for StyleOnHover<T, A, V>
where
    VS: View<T, A>,
    V::Element: StyleableWidget,
    V: View<T, A> + Styleable<T, A, Output = VS>,
{
    type State = V::State;

    type Element = widget::StyleOnHover<V::Element>;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, state, element) = self.view.build(cx);
        let element = widget::StyleOnHover::new(element, self.style);
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        self.view
            .rebuild(cx, &prev.view, id, state, &mut element.element)
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        self.view.message(id_path, state, message, app_state)
    }
}

// TODO I'm not sure if it should be further possible to style this (compile times, ambiguous behavior etc.)
impl<T, A, V> Styleable<T, A> for StyleOnHover<T, A, V>
where
    V: View<T, A> + Styleable<T, A>,
    V::Output: Styleable<T, A>,
    // <V as Styleable<T, A>>::Output: Styleable<T, A>,
    <<V as Styleable<T, A>>::Output as View<T, A>>::Element: StyleableWidget,
{
    type Output = StyleOnHover<T, A, <V as Styleable<T, A>>::Output>;

    fn fg(self, color: ratatui::style::Color) -> Self::Output {
        StyleOnHover {
            view: self.view.fg(color),
            style: self.style,
            phantom: PhantomData,
        }
    }

    fn bg(self, color: ratatui::style::Color) -> Self::Output {
        StyleOnHover {
            view: self.view.bg(color),
            style: self.style,
            phantom: PhantomData,
        }
    }

    fn modifier(self, modifier: ratatui::style::Modifier) -> Self::Output {
        StyleOnHover {
            view: self.view.modifier(modifier),
            style: self.style,
            phantom: PhantomData,
        }
    }

    fn style(self, style: ratatui::style::Style) -> Self::Output {
        StyleOnHover {
            view: self.view.style(style),
            style: self.style,
            phantom: PhantomData,
        }
    }

    fn current_style(&self) -> Style {
        self.view.current_style()
    }
}

macro_rules! event_views {
    ($($name:ident),*) => {
        $(
        pub struct $name<T, A, V, C> {
            view: V,
            callback: C,
            phantom: PhantomData<fn() -> (T, A)>,
        }

        impl<T, A, V, C> ViewMarker for $name<T, A, V, C> {}

        impl<T, A, V, C> View<T, A> for $name<T, A, V, C>
        where
            V: View<T, A>,
            C: Fn(&mut T) -> A + Send,
        {
            type State = (V::State, Id);

            type Element = widget::$name<V::Element>;

            fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
                let (id, (state, element)) = cx.with_new_id(|cx| {
                    let (child_id, state, element) = self.view.build(cx);

                    ((state, child_id), widget::$name::new(element, cx.id_path()))
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
            ) -> ChangeFlags {
                cx.with_id(*id, |cx| {
                    self.view
                        .rebuild(cx, &prev.view, child_id, state, &mut element.element)
                })
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                (state, child_id): &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> xilem_core::MessageResult<A> {
                if let Some((first, rest_path)) = id_path.split_first() {
                    if first == child_id {
                        self.view.message(rest_path, state, message, app_state)
                    } else {
                        xilem_core::MessageResult::Stale(message)
                    }
                } else {
                    xilem_core::MessageResult::Action((self.callback)(app_state))
                }
            }
        }

        impl<T, A, V, C> Styleable<T, A> for $name<T, A, V, C>
        where
            V: View<T, A> + Styleable<T, A>,
            C: Fn(&mut T) -> A + Send,
        {
            type Output = $name<T, A, <V as Styleable<T, A>>::Output, C>;

            fn fg(self, color: ratatui::style::Color) -> Self::Output {
                $name {
                    view: self.view.fg(color),
                    callback: self.callback,
                    phantom: PhantomData,
                }
            }

            fn bg(self, color: ratatui::style::Color) -> Self::Output {
                $name {
                    view: self.view.bg(color),
                    callback: self.callback,
                    phantom: PhantomData,
                }
            }

            fn modifier(self, modifier: ratatui::style::Modifier) -> Self::Output {
                $name {
                    view: self.view.modifier(modifier),
                    callback: self.callback,
                    phantom: PhantomData,
                }
            }

            fn style(self, style: ratatui::style::Style) -> Self::Output {
                $name {
                    view: self.view.style(style),
                    callback: self.callback,
                    phantom: PhantomData,
                }
            }

            fn current_style(&self) -> Style {
                self.view.current_style()
            }
        }
        )*
    };
}

event_views!(OnClick, OnHover, OnHoverLost);
