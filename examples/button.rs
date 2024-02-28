use anyhow::Result;
use ratatui::style::{Color, Style};
use trui::*;

#[path = "./shared/logging.rs"]
mod logging;

pub fn button<T>(
    content: impl View<T>,
    click_cb: impl EventHandler<T> + Send,
) -> impl View<T> + Styleable {
    content
        .border(BorderKind::ThickStraight)
        .on_hover_style(Style::default().fg(Color::Green).bg(Color::LightYellow))
        .on_pressed_fg(Color::Blue)
        .on_click(click_cb)
}

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = crate::logging::setup_logging(tracing::Level::DEBUG)?;

    App::new(0, |count| {
        v_stack((
            button(
                format!("Click me to increment the count: {count}").fg(Color::Green),
                (|count: &mut i32| *count += 1, |count: &mut i32| *count += 3),
            ),
            button("Click me to decrement".fg(Color::Red), |count: &mut i32| {
                *count -= 1
            }),
        ))
    })
    .await
    .run()
    .await
}
