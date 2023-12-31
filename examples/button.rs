use anyhow::Result;
use ratatui::style::{Color, Style};
use trui::*;

pub fn button<T: 'static>(
    content: impl IntoBoxedView<T>,
    click_cb: impl EventHandler<T> + Send + 'static,
) -> impl View<T> + ViewMarker {
    block(content.boxed())
        .with_borders(BorderKind::ThickStraight)
        .on_hover_style(Style::default().fg(Color::Green).bg(Color::LightYellow))
        .on_pressed_fg(Color::Blue)
        .on_click(click_cb)
}

fn main() -> Result<()> {
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
    .run()
}
