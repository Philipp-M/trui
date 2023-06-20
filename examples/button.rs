use anyhow::Result;
use ratatui::style::{Color, Style};
use trui::*;

pub fn button<T: 'static>(
    content: impl BoxedView<T>,
    click_cb: impl Fn(&mut T) + Send + 'static,
) -> impl View<T> + ViewMarker {
    border(content.boxed())
        .bg(Color::LightGreen)
        .on_hover_style(Style::default().fg(Color::Green).bg(Color::LightYellow))
        .on_pressed_fg(Color::Blue)
        .on_click(click_cb)
}

fn main() -> Result<()> {
    App::new(0, |count| {
        v_stack((
            button(format!("Click me, ({count})").fg(Color::Green), |count| {
                *count += 1
            }),
            button("Click me to decrement".fg(Color::Red), |count| *count -= 1),
        ))
    })
    .run()
}
