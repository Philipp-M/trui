use anyhow::Result;
use ratatui::style::{Color, Style};
use trui::*;

pub fn button<T: 'static, C>(
    content: impl IntoBoxedView<T, C>,
    click_cb: impl Fn(&mut T) + Send + 'static,
) -> impl View<T, C> + ViewMarker {
    block(content.boxed())
        .with_borders(BorderKind::ThickStraight)
        .on_hover_style(Style::default().fg(Color::Green).bg(Color::LightYellow))
        .on_pressed_fg(Color::Blue)
        .on_click(click_cb)
}

fn main() -> Result<()> {
    App::new(
        |_| 0,
        |count, _| {
            v_stack((
                button(
                    format!("Click me to increment the count: {count}").fg(Color::Green),
                    |count| *count += 1,
                ),
                button("Click me to decrement".fg(Color::Red), |count| *count -= 1),
            ))
        },
    )
    .run()
}
