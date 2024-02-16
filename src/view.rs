mod animatable_value;
mod border;
mod common;
mod core;
mod defer;
mod events;
mod fill_max_size;
mod linear_layout;
mod margin;
mod text;
mod use_state;
mod weighted_linear_layout;

use std::marker::PhantomData;

use ratatui::style::{Color, Style};
pub use xilem_core::{Id, IdPath, VecSplice};

// TODO do this via a prelude instead (and possibly not wildcard export)
pub use self::core::*;
pub use animatable_value::*;
pub use border::*;
pub use common::*;
pub use defer::*;
pub use events::*;
pub use fill_max_size::*;
pub use linear_layout::*;
pub use margin::*;
pub use text::*;
pub use use_state::*;
pub use weighted_linear_layout::*;

// TODO this could maybe also be added directly to `View` (possibly copying the macro expanded version of it)
/// A trait that makes it possible to use core views such as [`Adapt`] in the continuation/builder style.
pub trait ViewExt<T, A>: View<T, A> + Sized {
    fn adapt<ParentT, ParentA, F>(self, f: F) -> Adapt<ParentT, ParentA, T, A, Self, F>
    where
        F: Fn(&mut ParentT, AdaptThunk<T, A, Self>) -> xilem_core::MessageResult<ParentA>
            + Sync
            + Send,
    {
        Adapt::new(f, self)
    }

    fn adapt_state<ParentT, F>(self, f: F) -> AdaptState<ParentT, T, Self, F>
    where
        F: Fn(&mut ParentT) -> &mut T + Send + Sync + Send,
    {
        AdaptState::new(f, self)
    }

    fn margin<S: Into<MarginStyle>>(self, style: S) -> Margin<Self, T, A> {
        let style = style.into();
        Margin {
            content: self,
            position: style.position,
            amount: style.amount,
            phantom: PhantomData,
        }
    }

    /// # Examples
    /// ```
    /// use trui::*;
    /// App::new((), move |()| {
    ///    v_stack((
    ///        "Text with border".border(BorderKind::Rounded),
    ///        "Other style".border((Borders::VERTICAL, Style::default().fg(Color::Red))),
    ///    ))
    /// });
    /// ```
    fn border<S: Into<BorderStyle>>(self, style: S) -> Border<Self, T, A> {
        let style = style.into();
        Border {
            content: self,
            borders: style.borders,
            kind: style.kind,
            style: style.style,
            phantom: PhantomData,
        }
    }

    /// # Examples
    /// ```
    /// use trui::*;
    /// App::new((), move |()| {
    ///    "Fill half of the parent width/height"
    ///        .border(BorderKind::Rounded)
    ///        .fill_max_size(0.5)
    /// });
    /// ```
    fn fill_max_size<P: Animatable<f64>, S: IntoFillMaxSizeStyle<P>>(
        self,
        style: S,
    ) -> FillMaxSize<Self, P, T, A> {
        let style = style.into();
        FillMaxSize {
            content: self,
            fill: style.fill,
            percent: style.percent,
            phantom: PhantomData,
        }
    }

    fn fill_max_width<P: Animatable<f64>>(self, percent: P) -> FillMaxSize<Self, P, T, A> {
        FillMaxSize {
            content: self,
            fill: Fill::WIDTH,
            percent,
            phantom: PhantomData,
        }
    }

    fn fill_max_height<P: Animatable<f64>>(self, percent: P) -> FillMaxSize<Self, P, T, A> {
        FillMaxSize {
            content: self,
            fill: Fill::HEIGHT,
            percent,
            phantom: PhantomData,
        }
    }

    fn on_click<EH: EventHandler<T, A>>(self, event_handler: EH) -> OnClick<Self, EH> {
        OnClick {
            view: self,
            event_handler,
        }
    }

    fn weight<W: Animatable<f64>>(self, weight: W) -> WeightedLayoutElement<Self, W, T, A> {
        WeightedLayoutElement {
            content: self,
            weight,
            phantom: PhantomData,
        }
    }

    fn on_mouse<EH: EventHandler<T, A, crate::widget::MouseEvent>>(
        self,
        event_handler: EH,
    ) -> OnMouse<Self, EH> {
        OnMouse {
            view: self,
            catch_event: crate::CatchMouseButton::empty(),
            event_handler,
        }
    }

    fn on_hover<EH: EventHandler<T, A>>(self, event_handler: EH) -> OnHover<Self, EH> {
        OnHover {
            view: self,
            event_handler,
        }
    }

    fn on_blur_hover<EH: EventHandler<T, A>>(self, event_handler: EH) -> OnHoverLost<Self, EH> {
        OnHoverLost {
            view: self,
            event_handler,
        }
    }

    fn on_hover_style<VS>(self, style: Style) -> StyleOnHover<Self>
    where
        VS: View<T, A>,
        Self: Styleable<Output = VS>,
    {
        StyleOnHover { view: self, style }
    }

    fn on_hover_fg<VS>(self, color: Color) -> StyleOnHover<Self>
    where
        VS: View<T, A>,
        Self: Styleable<Output = VS>,
    {
        self.on_hover_style(Style::default().fg(color))
    }

    fn on_hover_bg<VS>(self, color: Color) -> StyleOnHover<Self>
    where
        VS: View<T, A>,
        Self: Styleable<Output = VS>,
    {
        self.on_hover_style(Style::default().bg(color))
    }

    fn on_pressed_style<VS>(self, style: Style) -> StyleOnPressed<Self>
    where
        VS: View<T, A>,
        Self: Styleable<Output = VS>,
    {
        StyleOnPressed { view: self, style }
    }

    fn on_pressed_fg<VS>(self, color: Color) -> StyleOnPressed<Self>
    where
        VS: View<T, A>,
        Self: Styleable<Output = VS>,
    {
        self.on_pressed_style(Style::default().fg(color))
    }

    fn on_pressed_bg<VS>(self, color: Color) -> StyleOnPressed<Self>
    where
        VS: View<T, A>,
        Self: Styleable<Output = VS>,
    {
        self.on_pressed_style(Style::default().bg(color))
    }
}

impl<T, A, V: View<T, A>> ViewExt<T, A> for V {}
