mod app;
mod view;
mod widget;

// wildcards at least temporarily for convenience...
pub use app::{App, StateUpdater};
pub use ratatui::style::{Color, Modifier, Style};
pub use view::*;
pub use widget::{Point, Rect};
