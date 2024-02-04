mod block;
mod common;
mod core;
mod defer;
mod events;
mod linear_layout;
mod text;
mod use_state;

use ratatui::style::{Color, Style};
pub use xilem_core::{Id, IdPath, VecSplice};

// TODO do this via a prelude instead (and possibly not wildcard export)
pub use self::core::*;
pub use block::*;
pub use common::*;
pub use defer::*;
pub use events::*;
pub use linear_layout::*;
pub use text::*;
pub use use_state::*;

// TODO this could maybe also be added directly to `View` (possibly copying the macro expanded version of it
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

    fn on_click<EH: EventHandler<T, A>>(self, event_handler: EH) -> OnClick<Self, EH> {
        OnClick {
            view: self,
            event_handler,
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
