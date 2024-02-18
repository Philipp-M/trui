mod app;
pub mod geometry;
mod view;
mod widget;

// wildcards at least temporarily for convenience...
pub use app::App;
pub use ratatui::style::{Color, Modifier, Style};
pub use view::*;
pub use widget::CatchMouseButton;

#[cfg(any(test, doctest))]
mod test_helper;
