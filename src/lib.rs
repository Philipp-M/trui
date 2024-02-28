mod app;
mod app_config;
pub mod geometry;
mod view;
mod widget;

// wildcards at least temporarily for convenience...
pub use app::App;
pub use app_config::AppConfig;
pub use ratatui::style::{Color, Modifier, Style};
pub use view::*;
pub use widget::CatchMouseButton;

#[cfg(test)]
mod test_helper;
