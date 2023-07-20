use super::{Cx, Styleable, View, ViewMarker};
use crate::widget::{self, ChangeFlags, StyleableWidget, Widget};
use ratatui::style::{Color, Style};
use std::marker::PhantomData;
use xilem_core::Id;

pub trait Hoverable<T, C, A, V, CB: Fn(&mut T) -> A + Send> {
    fn on_hover(self, callback: CB) -> OnHover<T, C, A, V, CB>;
    fn on_blur_hover(self, callback: CB) -> OnHoverLost<T, C, A, V, CB>;
}

impl<T, C, A, V, CB> Hoverable<T, C, A, V, CB> for V
where
    V: View<T, C, A>,
    CB: Fn(&mut T) -> A + Send,
{
    fn on_hover(self, callback: CB) -> OnHover<T, C, A, V, CB> {
        OnHover {
            view: self,
            callback,
            phantom: PhantomData,
        }
    }

    fn on_blur_hover(self, callback: CB) -> OnHoverLost<T, C, A, V, CB> {
        OnHoverLost {
            view: self,
            callback,
            phantom: PhantomData,
        }
    }
}

pub trait Clickable<T, C, A, V, CB: Fn(&mut T) -> A + Send> {
    fn on_click(self, callback: CB) -> OnClick<T, C, A, V, CB>;
}

impl<T, C, A, V, CB> Clickable<T, C, A, V, CB> for V
where
    V: View<T, C, A>,
    CB: Fn(&mut T) -> A + Send,
{
    fn on_click(self, callback: CB) -> OnClick<T, C, A, V, CB> {
        OnClick {
            view: self,
            callback,
            phantom: PhantomData,
        }
    }
}

macro_rules! styled_event_views {
    ($($name:ident),*) => {
        $(
        pub struct $name<T, C, A, V> {
            view: V,
            style: Style,
            phantom: PhantomData<fn() -> (T, C, A)>,
        }

        impl<T, C, A, V> ViewMarker for $name<T, C, A, V> {}

        impl<T, C, A, V> Styleable<T, C, A> for $name<T, C, A, V>
        where
            V: View<T, C, A> + Styleable<T, C, A>,
            V::Output: Styleable<T, C, A>,
            <<V as Styleable<T, C, A>>::Output as View<T, C, A>>::Element: StyleableWidget + 'static,
        {
            type Output = $name<T, C, A, <V as Styleable<T, C, A>>::Output>;

            fn fg(self, color: ratatui::style::Color) -> Self::Output {
                $name {
                    view: self.view.fg(color),
                    style: self.style,
                    phantom: PhantomData,
                }
            }

            fn bg(self, color: ratatui::style::Color) -> Self::Output {
                $name {
                    view: self.view.bg(color),
                    style: self.style,
                    phantom: PhantomData,
                }
            }

            fn modifier(self, modifier: ratatui::style::Modifier) -> Self::Output {
                $name {
                    view: self.view.modifier(modifier),
                    style: self.style,
                    phantom: PhantomData,
                }
            }

            fn style(self, style: ratatui::style::Style) -> Self::Output {
                $name {
                    view: self.view.style(style),
                    style: self.style,
                    phantom: PhantomData,
                }
            }

            fn current_style(&self) -> Style {
                self.view.current_style()
            }
        }
        )*
    }
}

// TODO is "invisible" (i.e. without id) a good idea?
// it never should receive events (or other things) directly and is just a trait on top of any *actual* view?
impl<T, A, C, VS, V> View<T, C, A> for StyleOnHover<T, C, A, V>
where
    VS: View<T, C, A>,
    V::Element: StyleableWidget,
    V: View<T, C, A> + Styleable<T, C, A, Output = VS>,
{
    type State = V::State;

    type Element = widget::StyleOnHover<V::Element>;

    fn build(&self, cx: &mut Cx<C>) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, state, element) = self.view.build(cx);

        (id, state, widget::StyleOnHover::new(element, self.style))
    }

    fn rebuild(
        &self,
        cx: &mut Cx<C>,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        if element.style != self.style {
            element.style = self.style;
            changeflags |= ChangeFlags::PAINT;
        }
        changeflags |= self
            .view
            .rebuild(cx, &prev.view, id, state, &mut element.element);
        changeflags
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

impl<T, A, C, VS, V> View<T, C, A> for StyleOnPressed<T, C, A, V>
where
    VS: View<T, C, A>,
    V::Element: StyleableWidget + Widget + 'static,
    V: View<T, C, A> + Styleable<T, C, A, Output = VS>,
{
    type State = (V::State, Id);

    type Element = widget::StyleOnPressed<V::Element>;

    fn build(&self, cx: &mut Cx<C>) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (child_id, state, element) = self.view.build(cx);

            (
                (state, child_id),
                widget::StyleOnPressed::new(element, self.style),
            )
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx<C>,
        prev: &Self,
        id: &mut xilem_core::Id,
        (state, child_id): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        if element.style != self.style {
            element.style = self.style;
            changeflags |= ChangeFlags::PAINT;
        }
        changeflags |= cx.with_id(*id, |cx| {
            self.view.rebuild(
                cx,
                &prev.view,
                child_id,
                state,
                element.element.downcast_mut().expect(
                    "The style on pressed content widget changed its type, this should never happen!",
                ),
            )
        });
        changeflags
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        (state, child_id): &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        match id_path.split_first() {
            Some((first, rest_path)) if first == child_id => {
                self.view.message(rest_path, state, message, app_state)
            }
            _ => xilem_core::MessageResult::Stale(message),
        }
    }
}

styled_event_views!(StyleOnHover, StyleOnPressed);

pub trait HoverStyleable<T, C, A, V: View<T, C, A>> {
    fn on_hover_style(self, style: Style) -> StyleOnHover<T, C, A, V>;

    fn on_hover_fg(self, color: Color) -> StyleOnHover<T, C, A, V>
    where
        Self: Sized,
    {
        self.on_hover_style(Style::default().fg(color))
    }

    fn on_hover_bg(self, color: Color) -> StyleOnHover<T, C, A, V>
    where
        Self: Sized,
    {
        self.on_hover_style(Style::default().bg(color))
    }
}

impl<T, C, A, VS, V> HoverStyleable<T, C, A, V> for V
where
    VS: View<T, C, A>,
    V: View<T, C, A> + Styleable<T, C, A, Output = VS>,
{
    fn on_hover_style(self, style: Style) -> StyleOnHover<T, C, A, V> {
        StyleOnHover {
            view: self,
            style,
            phantom: PhantomData,
        }
    }
}

pub trait PressedStyleable<T, C, A, V: View<T, C, A>> {
    fn on_pressed_style(self, style: Style) -> StyleOnPressed<T, C, A, V>;

    fn on_pressed_fg(self, color: Color) -> StyleOnPressed<T, C, A, V>
    where
        Self: Sized,
    {
        self.on_pressed_style(Style::default().fg(color))
    }

    fn on_pressed_bg(self, color: Color) -> StyleOnPressed<T, C, A, V>
    where
        Self: Sized,
    {
        self.on_pressed_style(Style::default().bg(color))
    }
}

impl<T, C, A, VS, V> PressedStyleable<T, C, A, V> for V
where
    VS: View<T, C, A>,
    V: View<T, C, A> + Styleable<T, C, A, Output = VS>,
{
    fn on_pressed_style(self, style: Style) -> StyleOnPressed<T, C, A, V> {
        StyleOnPressed {
            view: self,
            style,
            phantom: PhantomData,
        }
    }
}

// TODO own state (id_path etc.)
macro_rules! event_views {
    ($($name:ident),*) => {
        $(
        pub struct $name<T, C, A, V, CB> {
            view: V,
            callback: CB,
            phantom: PhantomData<fn() -> (T, C, A)>,
        }

        impl<T, C, A, V, CB> ViewMarker for $name<T, C, A, V, CB> {}

        impl<T, C, A, V, CB> View<T, C, A> for $name<T, C, A, V, CB>
        where
            V: View<T, C, A>,
            CB: Fn(&mut T) -> A + Send,
        {
            type State = (V::State, Id);

            type Element = widget::$name<V::Element>;

            fn build(&self, cx: &mut Cx<C>) -> (xilem_core::Id, Self::State, Self::Element) {
                let (id, (state, element)) = cx.with_new_id(|cx| {
                    let (child_id, state, element) = self.view.build(cx);

                    ((state, child_id), widget::$name::new(element, cx.id_path()))
                });
                (id, state, element)
            }

            fn rebuild(
                &self,
                cx: &mut Cx<C>,
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
                match id_path.split_first() {
                    Some((first, rest_path)) if first == child_id => {
                        self.view.message(rest_path, state, message, app_state)
                    }
                    Some(_) => xilem_core::MessageResult::Stale(message),
                    None => xilem_core::MessageResult::Action((self.callback)(app_state))
                }
            }
        }

        impl<T, C, A, V, CB> Styleable<T, C, A> for $name<T, C, A, V, CB>
        where
            V: View<T, C, A> + Styleable<T, C, A>,
            CB: Fn(&mut T) -> A + Send,
        {
            type Output = $name<T, C, A, <V as Styleable<T, C, A>>::Output, CB>;

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

event_views!(OnHover, OnHoverLost);

pub struct OnClick<T, C, A, V, CB> {
    view: V,
    callback: CB,
    phantom: PhantomData<fn() -> (T, C, A)>,
}

impl<T, C, A, V, CB> ViewMarker for OnClick<T, C, A, V, CB> {}

impl<T, C, A, V, CB> View<T, C, A> for OnClick<T, C, A, V, CB>
where
    V: View<T, C, A>,
    <V as View<T, C, A>>::Element: 'static,
    CB: Fn(&mut T) -> A + Send,
{
    type State = (V::State, Id);

    type Element = widget::OnClick<V::Element>;

    fn build(&self, cx: &mut Cx<C>) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (child_id, state, element) = self.view.build(cx);

            (
                (state, child_id),
                widget::OnClick::new(element, cx.id_path()),
            )
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx<C>,
        prev: &Self,
        id: &mut xilem_core::Id,
        (state, child_id): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.with_id(*id, |cx| {
            self.view.rebuild(
                cx,
                &prev.view,
                child_id,
                state,
                element.element.downcast_mut().expect(
                    "The style on pressed content widget changed its type,\
                     this should never happen!",
                ),
            )
        })
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        (state, child_id): &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        match id_path.split_first() {
            Some((first, rest_path)) if first == child_id => {
                self.view.message(rest_path, state, message, app_state)
            }
            Some(_) => xilem_core::MessageResult::Stale(message),
            None => xilem_core::MessageResult::Action((self.callback)(app_state)),
        }
    }
}

impl<T, C, A, V, CB> Styleable<T, C, A> for OnClick<T, C, A, V, CB>
where
    V: View<T, C, A> + Styleable<T, C, A>,
    // <V as Styleable<T, A>>::Output: 'static,
    <<V as Styleable<T, C, A>>::Output as View<T, C, A>>::Element: 'static,
    CB: Fn(&mut T) -> A + Send,
{
    type Output = OnClick<T, C, A, <V as Styleable<T, C, A>>::Output, CB>;

    fn fg(self, color: ratatui::style::Color) -> Self::Output {
        OnClick {
            view: self.view.fg(color),
            callback: self.callback,
            phantom: PhantomData,
        }
    }

    fn bg(self, color: ratatui::style::Color) -> Self::Output {
        OnClick {
            view: self.view.bg(color),
            callback: self.callback,
            phantom: PhantomData,
        }
    }

    fn modifier(self, modifier: ratatui::style::Modifier) -> Self::Output {
        OnClick {
            view: self.view.modifier(modifier),
            callback: self.callback,
            phantom: PhantomData,
        }
    }

    fn style(self, style: ratatui::style::Style) -> Self::Output {
        OnClick {
            view: self.view.style(style),
            callback: self.callback,
            phantom: PhantomData,
        }
    }

    fn current_style(&self) -> Style {
        self.view.current_style()
    }
}
