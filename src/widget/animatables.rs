use std::{any::Any, ops::DerefMut, time::Duration};

use super::{ChangeFlags, LifeCycleCx};

pub trait AnimatableElement<V>: 'static + AnyAnimatableElement<V> {
    fn animate(&mut self, cx: &mut LifeCycleCx) -> &V;
}

pub trait AnyAnimatableElement<V> {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str;
}

impl<V, A: AnimatableElement<V>> AnyAnimatableElement<V> for A {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl<V: 'static> AnimatableElement<V> for Box<dyn AnimatableElement<V>> {
    fn animate(&mut self, cx: &mut LifeCycleCx) -> &V {
        self.deref_mut().animate(cx)
    }
}

macro_rules! impl_animatable_for_primitive {
    ($type: ty) => {
        impl AnimatableElement<$type> for $type {
            fn animate(&mut self, _cx: &mut LifeCycleCx) -> &$type {
                self
            }
        }
    };
}

// All builtin number types
impl_animatable_for_primitive!(i8);
impl_animatable_for_primitive!(u8);
impl_animatable_for_primitive!(i16);
impl_animatable_for_primitive!(u16);
impl_animatable_for_primitive!(i32);
impl_animatable_for_primitive!(u32);
impl_animatable_for_primitive!(i64);
impl_animatable_for_primitive!(u64);
impl_animatable_for_primitive!(i128);
impl_animatable_for_primitive!(u128);
impl_animatable_for_primitive!(isize);
impl_animatable_for_primitive!(usize);

impl_animatable_for_primitive!(f32);
impl_animatable_for_primitive!(f64);

#[derive(Clone, Debug)]
pub struct LowPassIIR<AT, V> {
    pub(crate) target: AT,
    decay: f64,
    value: Option<V>,
}

impl<AT> LowPassIIR<AT, f64> {
    pub(crate) fn new(target: AT, decay: f64) -> Self {
        LowPassIIR {
            target,
            decay,
            value: None,
        }
    }

    pub(crate) fn set_decay(&mut self, decay: f64) -> ChangeFlags {
        if self.decay != decay {
            self.decay = decay;
            ChangeFlags::ANIMATION
        } else {
            ChangeFlags::empty()
        }
    }
}

impl<AT: AnimatableElement<f64>> AnimatableElement<f64> for LowPassIIR<AT, f64> {
    fn animate(&mut self, cx: &mut LifeCycleCx) -> &f64 {
        let target_value = self.target.animate(cx);
        if let Some(value) = &mut self.value {
            if (*target_value - *value).abs() > 0.0001 {
                let delta_time = cx.time_since_last_render_request().as_secs_f64() * 100.0; // could be a different factor, and maybe more precisely a frequency based cutoff or something like that
                let time_adjusted_decay =
                    1.0 - ((1.0 - self.decay.clamp(0.0, 1.0)).powf(delta_time));
                *value += time_adjusted_decay * (*target_value - *value);
                cx.request_animation_update();
            } else if *value != *target_value {
                *value = *target_value;
                cx.request_animation_update();
            }
        } else {
            self.value = Some(*target_value);
        }
        self.value.as_ref().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Lerp<T, R> {
    pub(crate) tweenable: T,
    pub(crate) ratio: R,
}

impl<T, R> Lerp<T, R> {
    pub(crate) fn new(tweenable: T, ratio: R) -> Self {
        Lerp { tweenable, ratio }
    }
}

impl<V, T: TweenableElement<V>, R: AnimatableElement<f64>> AnimatableElement<V> for Lerp<T, R> {
    fn animate(&mut self, cx: &mut LifeCycleCx) -> &V {
        let ratio = self.ratio.animate(cx);
        self.tweenable.interpolate(cx, *ratio)
    }
}

// TODO Duration could also be animated, but I'm not sure it's worth the complexity (vs benefit)...
#[derive(Clone, Debug)]
pub struct Tween<PS, TW> {
    pub(crate) play_speed: PS,
    current_time: Duration,
    pub(crate) tweenable: TW,
    duration: Duration,
}

impl<PS, TW> Tween<PS, TW> {
    pub(crate) fn new(play_speed: PS, tweenable: TW, duration: Duration) -> Self {
        Tween {
            play_speed,
            tweenable,
            current_time: Duration::ZERO,
            duration,
        }
    }
}

impl<V: 'static, PS: AnimatableElement<f64>, TW: TweenableElement<V>> AnimatableElement<V>
    for Tween<PS, TW>
{
    fn animate(&mut self, cx: &mut LifeCycleCx) -> &V {
        let play_speed = self.play_speed.animate(cx);
        let duration_as_secs = self.duration.as_secs_f64();
        let current_time_as_secs = self.current_time.as_secs_f64();
        let new_time = (current_time_as_secs
            + *play_speed * cx.time_since_last_render_request().as_secs_f64())
        .clamp(0.0, duration_as_secs);
        // avoid division by zero
        let ratio = if duration_as_secs == 0.0 {
            0.0
        } else {
            current_time_as_secs / duration_as_secs
        };

        if !self.duration.is_zero()
            && ((*play_speed > 0.0 && self.current_time != self.duration)
                || (*play_speed < 0.0 && self.current_time != Duration::ZERO))
        {
            self.current_time = Duration::from_secs_f64(new_time);
            cx.request_animation_update();
        }
        self.tweenable.interpolate(cx, ratio)
    }
}

// ---------------------------------- TWEENABLE

pub trait TweenableElement<V>: 'static {
    fn interpolate(&mut self, cx: &mut LifeCycleCx, ratio: f64) -> &V;
}

pub struct TweenableRange<A, V> {
    pub(crate) start: A,
    pub(crate) end: A,
    value: V,
}

impl<A> TweenableRange<A, f64> {
    pub(crate) fn new(start: A, end: A) -> Self {
        Self {
            start,
            end,
            value: 0.0,
        }
    }
}

impl<A: AnimatableElement<f64>> TweenableElement<f64> for TweenableRange<A, f64> {
    fn interpolate(&mut self, cx: &mut LifeCycleCx, ratio: f64) -> &f64 {
        let start = self.start.animate(cx);
        let end = self.end.animate(cx);

        self.value = *start * (1.0 - ratio) + *end * ratio;
        &mut self.value
    }
}
pub mod ease {
    use crate::widget::LifeCycleCx;

    use super::TweenableElement;

    macro_rules! ease_fn {
        ($type: ident, $ease_fn: expr) => {
            #[derive(Clone, Debug)]
            pub struct $type<T>(pub(crate) T);

            impl<V, T: TweenableElement<V>> TweenableElement<V> for $type<T> {
                fn interpolate(&mut self, cx: &mut LifeCycleCx, ratio: f64) -> &V {
                    #[allow(clippy::redundant_closure_call)]
                    self.0.interpolate(cx, $ease_fn(ratio))
                }
            }
        };
    }

    ease_fn!(Reverse, |ratio| 1.0 - ratio);
    ease_fn!(QuadraticIn, |ratio| ratio * ratio);
    ease_fn!(QuadraticOut, |ratio: f64| -(ratio * (ratio - 2.0)));
    ease_fn!(QuadraticInOut, |ratio: f64| {
        if ratio < 0.5 {
            2.0 * ratio * ratio
        } else {
            (-2.0 * ratio * ratio) + (4.0 * ratio) - 1.0
        }
    });
    ease_fn!(ElasticInOut, |ratio: f64| {
        use std::f64::consts::TAU;
        (if ratio < 0.5 {
            0.5 * (13.0 * TAU * (2.0 * ratio)).sin() * 2.0_f64.powf(10.0 * ((2.0 * ratio) - 1.0))
        } else {
            0.5 * ((-13.0 * TAU * ((2.0 * ratio - 1.0) + 1.0)).sin()
                * 2.0_f64.powf(-10.0 * (2.0 * ratio - 1.0))
                + 2.0)
        })
        .clamp(0.0, 1.0)
    });
}
