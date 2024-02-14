use std::{
    marker::PhantomData,
    ops::{Add, Mul, Sub},
    sync::Arc,
    task::Waker,
    time::Duration,
};

use tokio::runtime::Runtime;
use xilem_core::{AsyncWake, Id, MessageResult};

use crate::{widget::ChangeFlags, Cx};

// xilem_core::generate_view_trait!(AnimatableValue, Send, Cx, ChangeFlags; (Send + Sync), (Send));
// TODO animation specific ChangeFlags (currently only UPDATE is used, in case something has changed (and dependent AnimatableValues have to be updated as well)
// This is basically a View trait without <T, A> but this may be subject to change, to allow animations based on the AppState (via events)
pub trait Animatable: Send + Sync {
    /// Associated state for the view.
    type State: Send;

    type Value; //: Send + Sync; // bounds not really necessary, but currently it reduces some boilerplate, because the `Value` is stored in views

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

enum LowPassIIRMessage {
    TargetValueChanged(f64),
    DecayChanged(f64),
}

pub struct LowPassIIRState<ATS> {
    target_state: ATS,
    target_id: Id,
    target_value: f64,
    value: f64,
    message_tx: Option<tokio::sync::mpsc::Sender<LowPassIIRMessage>>,
    new_value_rx: Option<tokio::sync::mpsc::Receiver<f64>>,
    task: Option<tokio::task::JoinHandle<()>>,
    waker: Waker,
    runtime: Arc<Runtime>,
}

impl<ATS> LowPassIIRState<ATS> {
    fn start_or_wake(&mut self, mut decay: f64) {
        if self.task_finished() {
            let (message_tx, mut message_rx) = tokio::sync::mpsc::channel(100);
            let (new_value_tx, new_value_rx) = tokio::sync::mpsc::channel(100);
            self.message_tx = Some(message_tx);
            self.new_value_rx = Some(new_value_rx);

            let mut target_value = self.target_value;
            let mut value = self.value;
            let mut finished = false;
            let waker = self.waker.clone();

            self.task = Some(self.runtime.spawn(async move {
                // ideally the screen refresh rate, but this should do it for now...
                let mut interval = tokio::time::interval(Duration::from_secs_f64(1.0 / 60.0));

                while !finished {
                    tokio::select! {
                        _ = interval.tick() => {
                            let needs_update = (target_value.abs() - value.abs()).abs() > 0.001;
                            // tracing::info!("needs_update: {needs_update}");
                            value += decay.clamp(0.0, 1.0) * (target_value - value);

                            if !needs_update || new_value_tx.send(value).await.is_err() {
                                finished = true;
                            } else {
                                waker.wake_by_ref();
                            }
                        }
                        message = message_rx.recv() => {
                            match message {
                                Some(LowPassIIRMessage::TargetValueChanged(new_value)) => target_value = new_value,
                                Some(LowPassIIRMessage::DecayChanged(new_decay)) => decay = new_decay,
                                None => finished = true
                            }
                        }
                    };
                }
                tracing::info!("Low pass Finished!");
            }));
        } else if let Some(message_tx) = self.message_tx.as_ref() {
            let _ = message_tx.blocking_send(LowPassIIRMessage::DecayChanged(decay));
            let _ =
                message_tx.blocking_send(LowPassIIRMessage::TargetValueChanged(self.target_value));
        }
    }

    fn task_finished(&self) -> bool {
        self.task.as_ref().map(|t| t.is_finished()).unwrap_or(true)
    }

    fn poll_value(&mut self) -> Option<f64> {
        self.new_value_rx.as_mut().and_then(|rx| rx.try_recv().ok())
    }
}

impl<AT: Animatable<Value = f64>> Animatable for LowPassIIR<AT> {
    type State = LowPassIIRState<AT::State>;
    type Value = f64;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, f64) {
        let (id, state) = cx.with_new_id(|cx| {
            let (target_id, target_state, target_value) = self.target.build(cx);
            LowPassIIRState {
                target_state,
                target_id,
                target_value,
                task: None,
                runtime: cx.rt.clone(),
                value: target_value,
                new_value_rx: None,
                message_tx: None,
                waker: cx.waker(),
            }
        });
        let start_value = state.target_value;
        (id, state, start_value)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        value: &mut f64,
    ) -> ChangeFlags {
        cx.with_id(*id, |cx| {
            let mut changeflags = self.target.rebuild(
                cx,
                &prev.target,
                &mut state.target_id,
                &mut state.target_state,
                &mut state.target_value,
            );
            if changeflags.contains(ChangeFlags::UPDATE)
                && (state.target_value.abs() - value.abs()).abs() > 0.001
            {
                state.start_or_wake(self.decay);
            }
            if !state.task_finished() {
                cx.add_pending_async(*id)
            }
            if *value != state.value {
                *value = state.value;
                changeflags |= ChangeFlags::UPDATE;
            }
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
            [id, rest_path @ ..] if *id == state.target_id => {
                self.target
                    .message(rest_path, &mut state.target_state, message)
            }
            [] if message.downcast_ref::<AsyncWake>().is_some() => {
                let mut message_result = MessageResult::Nop;
                while let Some(value) = state.poll_value() {
                    // tracing::info!("new_value: {value}");
                    state.value = value;
                    message_result = MessageResult::RequestRebuild;
                }
                message_result
            }
            [..] => MessageResult::Stale(message),
        }
    }
}

pub fn low_pass<AT: Animatable<Value = f64>>(decay: f64, target: AT) -> LowPassIIR<AT> {
    LowPassIIR { decay, target }
}

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
