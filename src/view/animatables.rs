use std::{ops::Range, time::Duration};

use xilem_core::{Id, MessageResult};

use crate::{
    widget::{self, animatables::AnimatableElement, ChangeFlags},
    Cx,
};

// This is basically a View trait without <T, A> (but this may be subject to change, to allow animations based on the AppState (via e.g. event callbacks))
pub trait Animatable<V>: Send + Sync {
    /// Associated state for the animatable.
    type State: Send;

    /// Associated state for the animatable.
    type Element: AnimatableElement<V>;

    /// Build the associated widget and initialize state.
    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element);

    /// Update the associated value.
    ///
    /// Returns an indication of what, if anything, has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags;

    /// Propagate a message.
    ///
    /// Handle a message, propagating to children if needed. Here, `id_path` is a slice
    /// of ids beginning at a child of this animatable.
    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()>; // TODO different type (AnimationMessage?)
}

#[derive(Clone, Debug)]
pub struct Lerp<T, R> {
    tweenable: T,
    ratio: R,
}

pub fn lerp<V, T: Tweenable<V>, R: Animatable<f64>>(tweenable: T, ratio: R) -> Lerp<T, R> {
    Lerp { tweenable, ratio }
}

impl<V, T: Tweenable<V>, R: Animatable<f64>> Animatable<V> for Lerp<T, R> {
    type State = (Id, T::State, Id, R::State);

    type Element = widget::animatables::Lerp<T::Element, R::Element>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (ratio_id, ratio_state, ratio_element) = self.ratio.build(cx);
            let (tweenable_id, tweenable_state, tweenable_element) = self.tweenable.build(cx);
            (
                (tweenable_id, tweenable_state, ratio_id, ratio_state),
                widget::animatables::Lerp::new(tweenable_element, ratio_element),
            )
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        (tweenable_id, tweenable_state, ratio_id, ratio_state): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.with_id(*id, |cx| {
            self.ratio
                .rebuild(cx, &prev.ratio, ratio_id, ratio_state, &mut element.ratio)
                | self.tweenable.rebuild(
                    cx,
                    &prev.tweenable,
                    tweenable_id,
                    tweenable_state,
                    &mut element.tweenable,
                )
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        (tweenable_id, tweenable_state, ratio_id, ratio_state): &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()> {
        match id_path {
            [id, rest_path @ ..] if *id == *ratio_id => {
                self.ratio.message(rest_path, ratio_state, message)
            }
            [id, rest_path @ ..] if *id == *tweenable_id => {
                self.tweenable.message(rest_path, tweenable_state, message)
            }
            [..] => MessageResult::Stale(message),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LowPassIIR<AT> {
    decay: f64,
    target: AT,
}

impl<AT: Animatable<f64>> Animatable<f64> for LowPassIIR<AT> {
    type State = AT::State;

    type Element = widget::animatables::LowPassIIR<AT::Element, f64>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, state, target) = self.target.build(cx);
        (
            id,
            state,
            widget::animatables::LowPassIIR::new(target, self.decay),
        )
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        self.target
            .rebuild(cx, &prev.target, id, state, &mut element.target)
            | element.set_decay(self.decay)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()> {
        self.target.message(id_path, state, message)
    }
}

pub fn low_pass<AT: Animatable<f64>>(decay: f64, target: AT) -> LowPassIIR<AT> {
    LowPassIIR { decay, target }
}

// TODO use a macro for primitive non-animating/"const" values like the following
impl Animatable<u32> for u32 {
    type State = ();

    type Element = u32;

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        (Id::next(), (), *self)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        value: &mut Self::Element,
    ) -> ChangeFlags {
        if self != prev {
            *value = *self;
            ChangeFlags::ANIMATION
        } else {
            ChangeFlags::empty()
        }
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()> {
        MessageResult::Stale(message)
    }
}

impl Animatable<f64> for f64 {
    type State = ();

    type Element = f64;

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        (Id::next(), (), *self)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        value: &mut Self::Element,
    ) -> ChangeFlags {
        if self != prev {
            *value = *self;
            ChangeFlags::ANIMATION
        } else {
            ChangeFlags::empty()
        }
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()> {
        MessageResult::Stale(message)
    }
}

pub trait Tweenable<V>: Send + Sync {
    /// Associated state for the tweenable.
    type State: Send;

    type Element: widget::animatables::TweenableElement<V>;

    /// Build the associated widget and initialize state.
    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element);

    /// Update the associated element.
    ///
    /// Returns an indication of what, if anything, has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags;

    /// Propagate a message.
    ///
    /// Handle a message, propagating to children if needed. Here, `id_path` is a slice
    /// of ids beginning at a child of this tweenable.
    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()>;

    fn tween<PS>(self, duration: Duration, play_speed: PS) -> Tween<PS, Self>
    where
        Self: Sized,
        PS: Animatable<f64>,
    {
        Tween {
            play_speed,
            tweenable: self,
            duration,
        }
    }

    fn lerp<R>(self, ratio: R) -> Lerp<Self, R>
    where
        Self: Sized,
        R: Animatable<f64>,
    {
        Lerp {
            tweenable: self,
            ratio,
        }
    }

    // Convenience modifier methods
    fn reverse(self) -> ease::Reverse<Self>
    where
        Self: Sized,
    {
        ease::Reverse(self)
    }

    fn quadratic_in_ease(self) -> ease::QuadraticIn<Self>
    where
        Self: Sized,
    {
        ease::QuadraticIn(self)
    }

    fn quadratic_out_ease(self) -> ease::QuadraticOut<Self>
    where
        Self: Sized,
    {
        ease::QuadraticOut(self)
    }

    fn quadratic_in_out_ease(self) -> ease::QuadraticInOut<Self>
    where
        Self: Sized,
    {
        ease::QuadraticInOut(self)
    }

    fn elastic_in_out_ease(self) -> ease::ElasticInOut<Self>
    where
        Self: Sized,
    {
        ease::ElasticInOut(self)
    }
}

/// returns just the start value, only really useful in combination with Tweenable
impl<A: Animatable<f64>> Tweenable<f64> for Range<A> {
    type State = (Id, A::State, Id, A::State);

    type Element = widget::animatables::TweenableRange<A::Element, f64>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (start_id, start_state, start_element) = self.start.build(cx);
            let (end_id, end_state, end_element) = self.end.build(cx);
            let element = widget::animatables::TweenableRange::new(start_element, end_element);
            ((start_id, start_state, end_id, end_state), element)
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        (start_id, start_state, end_id, end_state): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.with_id(*id, |cx| {
            let start_changeflags =
                self.start
                    .rebuild(cx, &prev.start, start_id, start_state, &mut element.start);
            let end_changeflags =
                self.end
                    .rebuild(cx, &prev.end, end_id, end_state, &mut element.end);

            start_changeflags | end_changeflags
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        (start_id, start_state, end_id, end_state): &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()> {
        match id_path {
            [id, rest_path @ ..] if *id == *start_id => {
                self.start.message(rest_path, start_state, message)
            }
            [id, rest_path @ ..] if *id == *end_id => {
                self.end.message(rest_path, end_state, message)
            }
            [..] => MessageResult::Stale(message),
        }
    }
}

// TODO Duration could also be animated, but I'm not sure it's worth the complexity (vs benefit)...
#[derive(Clone, Debug)]
pub struct Tween<PS, TW> {
    play_speed: PS,
    tweenable: TW,
    duration: Duration,
}

impl<V, PS, TW> Animatable<V> for Tween<PS, TW>
where
    V: 'static,
    PS: Animatable<f64>,
    TW: Tweenable<V>,
{
    type State = (Id, PS::State, Id, TW::State);

    type Element = widget::animatables::Tween<PS::Element, TW::Element>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (play_speed_id, play_speed_state, play_speed_element) = self.play_speed.build(cx);
            let (tweenable_id, tweenable_state, tweenable_element) = self.tweenable.build(cx);

            let element = widget::animatables::Tween::new(
                play_speed_element,
                tweenable_element,
                self.duration,
            );
            (
                (
                    play_speed_id,
                    play_speed_state,
                    tweenable_id,
                    tweenable_state,
                ),
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
        (play_speed_id, play_speed_state, tweenable_id, tweenable_state): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        cx.with_id(*id, |cx| {
            self.play_speed.rebuild(
                cx,
                &prev.play_speed,
                play_speed_id,
                play_speed_state,
                &mut element.play_speed,
            ) | self.tweenable.rebuild(
                cx,
                &prev.tweenable,
                tweenable_id,
                tweenable_state,
                &mut element.tweenable,
            )
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        (play_speed_id, play_speed_state, tweenable_id, tweenable_state): &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()> {
        match id_path {
            [id, rest_path @ ..] if *id == *play_speed_id => {
                self.play_speed
                    .message(rest_path, play_speed_state, message)
            }
            [id, rest_path @ ..] if *id == *tweenable_id => {
                self.tweenable.message(rest_path, tweenable_state, message)
            }
            [..] => MessageResult::Stale(message),
        }
    }
}

pub mod ease {
    use crate::{widget::animatables::ease as ease_widget, widget::ChangeFlags, Cx, Tweenable};
    use xilem_core::{Id, MessageResult};

    macro_rules! ease_fn {
        ($type: ident) => {
            #[derive(Clone, Debug)]
            pub struct $type<T>(pub(crate) T);

            impl<V, T: Tweenable<V>> Tweenable<V> for $type<T> {
                type State = T::State;
                type Element = ease_widget::$type<T::Element>;

                fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
                    // , ease_widget::$type::<()>::ease(ratio)
                    let (id, state, tweenable) = self.0.build(cx);
                    (id, state, ease_widget::$type(tweenable))
                }

                fn rebuild(
                    &self,
                    cx: &mut Cx,
                    prev: &Self,
                    id: &mut Id,
                    state: &mut Self::State,
                    element: &mut Self::Element,
                ) -> ChangeFlags {
                    self.0.rebuild(cx, &prev.0, id, state, &mut element.0)
                }

                fn message(
                    &self,
                    id_path: &[Id],
                    state: &mut Self::State,
                    message: Box<dyn std::any::Any>,
                ) -> MessageResult<()> {
                    self.0.message(id_path, state, message)
                }
            }
        };
    }

    ease_fn!(Reverse);
    ease_fn!(QuadraticIn);
    ease_fn!(QuadraticOut);
    ease_fn!(QuadraticInOut);
    ease_fn!(ElasticInOut);
}
