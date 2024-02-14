use std::{
    marker::PhantomData,
    ops::{Add, Mul, Sub},
    time::Duration,
};

use xilem_core::{AsyncWake, Id, MessageResult};

use crate::{widget::ChangeFlags, Cx};

// xilem_core::generate_view_trait!(AnimatableValue, Send, Cx, ChangeFlags; (Send + Sync), (Send));
// TODO animation specific ChangeFlags (currently only UPDATE is used, in case something has changed (and dependent AnimatableValues have to be updated as well)
// This is basically a View trait without <T, A> but this may be subject to change, to allow animations based on the AppState (via events)
pub trait Animatable: Send + Sync {
    /// Associated state for the view.
    type State: Send;

    type Value;

    /// Build the associated widget and initialize state.
    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Value);

    /// Update the associated value.
    ///
    /// Returns an indication of what, if anything, has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        value: &mut Self::Value,
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

pub struct Lerp<
    V,
    AnS: Animatable<Value = V>,
    AnE: Animatable<Value = V>,
    AnR: Animatable<Value = f64>,
> {
    start: AnS,
    end: AnE,
    ratio: AnR,
    #[allow(clippy::complexity)]
    phantom: PhantomData<fn() -> V>,
}

pub trait Lens: Animatable {
    fn interpolate(&self, state: &mut Self::State, target: &mut Self::Value, ratio: f64);
}

pub fn lerp<
    V: Copy + Send + Sub<Output = V> + Mul<f64, Output = V> + Add<Output = V> + std::fmt::Display,
    AnS: Animatable<Value = V>,
    AnE: Animatable<Value = V>,
    AnR: Animatable<Value = f64>,
>(
    start: AnS,
    end: AnE,
    ratio: AnR,
) -> Lerp<V, AnS, AnE, AnR> {
    Lerp {
        start,
        end,
        ratio,
        phantom: PhantomData,
    }
}

pub struct LerpState<V, SS, ES, RS> {
    start: V,
    end: V,
    ratio: f64,
    start_id: Id,
    end_id: Id,
    ratio_id: Id,
    start_state: SS,
    end_state: ES,
    ratio_state: RS,
}

impl<
        // TODO multiple concrete impls instead of blanket impl?
        V: Copy + Send + Sub<Output = V> + Mul<f64, Output = V> + Add<Output = V> + std::fmt::Display,
        AnS: Animatable<Value = V>,
        AnE: Animatable<Value = V>,
        AnR: Animatable<Value = f64>,
    > Animatable for Lerp<V, AnS, AnE, AnR>
{
    type State = LerpState<V, AnS::State, AnE::State, AnR::State>;

    type Value = V;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, V) {
        // tracing::info!("lolaorstuyolasrtoyuin");
        let (id, (state, el)) = cx.with_new_id(|cx| {
            let (start_id, start_state, start) = self.start.build(cx);
            let (end_id, end_state, end) = self.end.build(cx);
            let (ratio_id, ratio_state, ratio) = self.ratio.build(cx);

            let interpolated = start + (end - start) * ratio.clamp(0.0, 1.0);
            // tracing::info!("interpolated: {interpolated}");
            let state = LerpState {
                start,
                end,
                ratio,
                start_id,
                end_id,
                ratio_id,
                start_state,
                end_state,
                ratio_state,
            };
            (state, interpolated)
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
            changeflags |= self.start.rebuild(
                cx,
                &prev.start,
                &mut state.start_id,
                &mut state.start_state,
                &mut state.start,
            );
            changeflags |= self.end.rebuild(
                cx,
                &prev.end,
                &mut state.end_id,
                &mut state.end_state,
                &mut state.end,
            );
            changeflags |= self.ratio.rebuild(
                cx,
                &prev.ratio,
                &mut state.ratio_id,
                &mut state.ratio_state,
                &mut state.ratio,
            );
        });
        if changeflags.contains(ChangeFlags::UPDATE) {
            *value = state.start + (state.end - state.start) * state.ratio.clamp(0.0, 1.0);
        }
        changeflags
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
            [id, rest_path @ ..] if *id == state.ratio_id => {
                self.ratio
                    .message(rest_path, &mut state.ratio_state, message)
            }
            [..] => MessageResult::Stale(message),
        }
    }
}

pub struct LowPassIIR<AT> {
    decay: f64,
    target: AT,
}

impl<AT: Animatable<Value = f64>> Animatable for LowPassIIR<AT> {
    type State = (AT::State, f64);
    type Value = f64;

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
            // TODO set exact value when this threshold is reached?
            if (*target_value - *value).abs() > 0.0001 {
                let delta_time = cx
                    .time_since_last_rebuild()
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

pub fn low_pass<AT: Animatable<Value = f64>>(decay: f64, target: AT) -> LowPassIIR<AT> {
    LowPassIIR { decay, target }
}

// TODO could be a macro for primitive non-animating values like the following
impl Animatable for u32 {
    type State = ();
    type Value = u32;

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

impl Animatable for f64 {
    type State = ();
    type Value = f64;

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
