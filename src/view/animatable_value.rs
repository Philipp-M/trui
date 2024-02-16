use std::{ops::Range, time::Duration};

use xilem_core::{Id, MessageResult};

use crate::{widget::ChangeFlags, Cx};

// TODO animation specific ChangeFlags (currently only UPDATE is used, in case something has changed (and dependent AnimatableValues have to be updated as well)
// This is basically a View trait without <T, A> (but this may be subject to change, to allow animations based on the AppState (via e.g. event callbacks))
// And the Element type being a generic type parameter instead of an associated type
// This could well be extended to be a View + Widget relation,
// which would possibly more efficient, as the "engine" (i.e. updating the animation) would be in each widget, so it could be more localized and doesn't require a full View::rebuild
pub trait Animatable<V>: Send + Sync {
    /// Associated state for the view.
    type State: Send;

    /// Build the associated widget and initialize state.
    fn build(&self, cx: &mut Cx) -> (Id, Self::State, V);

    /// Update the associated value.
    ///
    /// Returns an indication of what, if anything, has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        value: &mut V,
    ) -> ChangeFlags;

    /// Propagate a message.
    ///
    /// Handle a message, propagating to children if needed. Here, `id_path` is a slice
    /// of ids beginning at a child of this view.
    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()>; // TODO different type (AnimationMessage?)
}

pub struct Lerp<T, R> {
    tweenable: T,
    ratio: R,
}

pub fn lerp<V, T: Tweenable<V>, R: Animatable<f64>>(tweenable: T, ratio: R) -> Lerp<T, R> {
    Lerp { tweenable, ratio }
}

pub struct LerpState<TS, RS> {
    tweenable_id: Id,
    tweenable_state: TS,
    ratio_id: Id,
    ratio_state: RS,
    ratio: f64,
}

impl<V, T: Tweenable<V>, R: Animatable<f64>> Animatable<V> for Lerp<T, R> {
    type State = LerpState<T::State, R::State>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, V) {
        let (id, (state, el)) = cx.with_new_id(|cx| {
            let (ratio_id, ratio_state, ratio) = self.ratio.build(cx);
            let (tweenable_id, tweenable_state, value) = self.tweenable.build(cx, ratio);
            let state = LerpState {
                tweenable_id,
                tweenable_state,
                ratio,
                ratio_id,
                ratio_state,
            };
            (state, value)
        });
        (id, state, el)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        value: &mut V,
    ) -> ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        cx.with_id(*id, |cx| {
            changeflags |= self.ratio.rebuild(
                cx,
                &prev.ratio,
                &mut state.ratio_id,
                &mut state.ratio_state,
                &mut state.ratio,
            );
            changeflags |= self.tweenable.rebuild(
                cx,
                &prev.tweenable,
                &mut state.tweenable_id,
                &mut state.tweenable_state,
                state.ratio,
                value,
            );
        });
        changeflags
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()> {
        match id_path {
            [id, rest_path @ ..] if *id == state.ratio_id => {
                self.ratio
                    .message(rest_path, &mut state.ratio_state, message)
            }
            [id, rest_path @ ..] if *id == state.tweenable_id => {
                self.tweenable
                    .message(rest_path, &mut state.tweenable_state, message)
            }
            [..] => MessageResult::Stale(message),
        }
    }
}

pub struct LowPassIIR<AT> {
    decay: f64,
    target: AT,
}

impl<AT: Animatable<f64>> Animatable<f64> for LowPassIIR<AT> {
    type State = (AT::State, f64);

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, f64) {
        let (id, state, target_value) = self.target.build(cx);
        (id, (state, target_value), target_value)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        (state, target_value): &mut Self::State,
        value: &mut f64,
    ) -> ChangeFlags {
        cx.with_id(*id, |cx| {
            let _target_changeflags =
                self.target
                    .rebuild(cx, &prev.target, id, state, target_value);

            if (*target_value - *value).abs() > 0.0001 {
                let delta_time = cx
                    .time_since_last_render()
                    .unwrap_or(Duration::from_secs_f64(1.0 / 60.0))
                    .as_secs_f64()
                    * 100.0; // could be a different factor, and maybe more precisely a frequency based cutoff or something like that
                let time_adjusted_decay =
                    1.0 - ((1.0 - self.decay.clamp(0.0, 1.0)).powf(delta_time));
                *value += time_adjusted_decay * (*target_value - *value);
                cx.request_frame_update();
                return ChangeFlags::UPDATE;
            } else if *value != *target_value {
                *value = *target_value;
                return ChangeFlags::UPDATE;
            }
            ChangeFlags::empty()
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        (state, _): &mut Self::State,
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

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, u32) {
        (Id::next(), (), *self)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        value: &mut u32,
    ) -> ChangeFlags {
        if self != prev {
            *value = *self;
            ChangeFlags::UPDATE
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

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, f64) {
        (Id::next(), (), *self)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        value: &mut f64,
    ) -> ChangeFlags {
        if self != prev {
            *value = *self;
            ChangeFlags::UPDATE
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

    /// Build the associated widget and initialize state.
    fn build(&self, cx: &mut Cx, ratio: f64) -> (Id, Self::State, V);

    /// Update the associated value.
    ///
    /// Returns an indication of what, if anything, has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        ratio: f64,
        value: &mut V,
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

pub struct TweenableRangeState<RS, V> {
    start_id: Id,
    end_id: Id,
    start_state: RS,
    end_state: RS,
    start_value: V,
    end_value: V,
}

/// returns just the start value, only really useful in combination with Tweenable
impl<A: Animatable<f64>> Tweenable<f64> for Range<A> {
    type State = TweenableRangeState<A::State, f64>;

    fn build(&self, cx: &mut Cx, ratio: f64) -> (Id, Self::State, f64) {
        let (id, state) = cx.with_new_id(|cx| {
            let (start_id, start_state, start_value) = self.start.build(cx);
            let (end_id, end_state, end_value) = self.end.build(cx);
            TweenableRangeState {
                start_id,
                end_id,
                start_state,
                end_state,
                start_value,
                end_value,
            }
        });
        let value = state.start_value * (1.0 - ratio) + state.end_value * ratio;
        (id, state, value)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        ratio: f64,
        value: &mut f64,
    ) -> ChangeFlags {
        cx.with_id(*id, |cx| {
            let start_changeflags = self.start.rebuild(
                cx,
                &prev.start,
                &mut state.start_id,
                &mut state.start_state,
                &mut state.start_value,
            );
            let end_changeflags = self.end.rebuild(
                cx,
                &prev.end,
                &mut state.end_id,
                &mut state.end_state,
                &mut state.end_value,
            );
            *value = state.start_value * (1.0 - ratio) + state.end_value * ratio;

            start_changeflags | end_changeflags
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()> {
        match id_path {
            [id, rest_path @ ..] if *id == state.start_id => {
                self.start
                    .message(rest_path, &mut state.start_state, message)
            }
            [id, rest_path @ ..] if *id == state.end_id => {
                self.end.message(rest_path, &mut state.end_state, message)
            }
            [..] => MessageResult::Stale(message),
        }
    }
}

// TODO Duration could also be animated, but I'm not sure it's worth the complexity (vs benefit)...
pub struct Tween<PS, TW> {
    play_speed: PS,
    tweenable: TW,
    duration: Duration,
}

pub struct TweenState<PSS, TWS> {
    play_speed_state: PSS,
    tweenable_state: TWS,
    play_speed_id: Id,
    tweenable_id: Id,
    play_speed_value: f64,
    current_time: Duration,
}

impl<V, PS: Animatable<f64>, TW: Tweenable<V>> Animatable<V> for Tween<PS, TW> {
    type State = TweenState<PS::State, TW::State>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, V) {
        let (id, (state, value)) = cx.with_new_id(|cx| {
            let (tweenable_id, tweenable_state, tweenable_value) = self.tweenable.build(cx, 0.0);
            let (play_speed_id, play_speed_state, play_speed_value) = self.play_speed.build(cx);
            (
                TweenState {
                    current_time: Duration::from_secs(0),
                    tweenable_id,
                    play_speed_id,
                    tweenable_state,
                    play_speed_state,
                    // tweenable_value,
                    play_speed_value,
                },
                tweenable_value,
            )
        });
        // let start_value = state.tweenable_value.clone();
        // TODO Allow looping the tween (over boundaries?)?
        if !self.duration.is_zero() && state.play_speed_value >= 0.0 {
            cx.request_frame_update();
        }
        (id, state, value)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        value: &mut V,
    ) -> ChangeFlags {
        cx.with_id(*id, |cx| {
            let mut changeflags = self.play_speed.rebuild(
                cx,
                &prev.play_speed,
                &mut state.play_speed_id,
                &mut state.play_speed_state,
                &mut state.play_speed_value,
            );
            let duration_as_secs = self.duration.as_secs_f64();
            let current_time_as_secs = state.current_time.as_secs_f64();
            let new_time = (current_time_as_secs
                + state.play_speed_value * cx.time_since_last_render().unwrap().as_secs_f64())
            .clamp(0.0, duration_as_secs);
            if current_time_as_secs != new_time {
                state.current_time = Duration::from_secs_f64(new_time);
                changeflags |= ChangeFlags::UPDATE;
            }
            let ratio = if duration_as_secs == 0.0 {
                0.0
            } else {
                current_time_as_secs / duration_as_secs
            };

            if !self.duration.is_zero()
                && ((state.play_speed_value > 0.0 && new_time != duration_as_secs)
                    || (state.play_speed_value < 0.0 && new_time != 0.0))
            {
                cx.request_frame_update();
            }
            changeflags |= self.tweenable.rebuild(
                cx,
                &prev.tweenable,
                &mut state.tweenable_id,
                &mut state.tweenable_state,
                ratio,
                value,
            );

            changeflags
        })
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
    ) -> MessageResult<()> {
        match id_path {
            [id, rest_path @ ..] if *id == state.tweenable_id => {
                self.tweenable
                    .message(rest_path, &mut state.tweenable_state, message)
            }
            [id, rest_path @ ..] if *id == state.play_speed_id => {
                self.play_speed
                    .message(rest_path, &mut state.play_speed_state, message)
            }
            [..] => MessageResult::Stale(message),
        }
    }
}

pub mod ease {
    use crate::{widget::ChangeFlags, Cx, Tweenable};
    use xilem_core::{Id, MessageResult};

    macro_rules! impl_tweenable_for_ease_fn {
        ($type: ty, $ease_fn: expr) => {
            impl<V, T: Tweenable<V>> Tweenable<V> for $type {
                type State = T::State;

                fn build(&self, cx: &mut Cx, ratio: f64) -> (Id, Self::State, V) {
                    #[allow(clippy::redundant_closure_call)]
                    self.0.build(cx, $ease_fn(ratio))
                }

                fn rebuild(
                    &self,
                    cx: &mut Cx,
                    prev: &Self,
                    id: &mut Id,
                    state: &mut Self::State,
                    ratio: f64,
                    value: &mut V,
                ) -> ChangeFlags {
                    #[allow(clippy::redundant_closure_call)]
                    self.0
                        .rebuild(cx, &prev.0, id, state, $ease_fn(ratio), value)
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

    pub struct Reverse<T>(pub(crate) T);
    impl_tweenable_for_ease_fn!(Reverse<T>, |ratio| 1.0 - ratio);

    pub struct QuadraticIn<T>(pub(crate) T);
    impl_tweenable_for_ease_fn!(QuadraticIn<T>, |ratio| ratio * ratio);

    pub struct QuadraticOut<T>(pub(crate) T);
    impl_tweenable_for_ease_fn!(QuadraticOut<T>, |ratio: f64| -(ratio * (ratio - 2.0)));

    pub struct QuadraticInOut<T>(pub(crate) T);
    impl_tweenable_for_ease_fn!(QuadraticInOut<T>, |ratio: f64| {
        if ratio < 0.5 {
            2.0 * ratio * ratio
        } else {
            (-2.0 * ratio * ratio) + (4.0 * ratio) - 1.0
        }
    });

    pub struct ElasticInOut<EF>(pub(crate) EF);
    impl_tweenable_for_ease_fn!(ElasticInOut<T>, |ratio: f64| {
        use std::f64::consts::TAU;
        if ratio < 0.5 {
            0.5 * (13.0 * TAU * (2.0 * ratio)).sin() * 2.0_f64.powf(10.0 * ((2.0 * ratio) - 1.0))
        } else {
            0.5 * ((-13.0 * TAU * ((2.0 * ratio - 1.0) + 1.0)).sin()
                * 2.0_f64.powf(-10.0 * (2.0 * ratio - 1.0))
                + 2.0)
        }
    });
}

