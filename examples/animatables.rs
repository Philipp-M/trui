use std::time::Duration;

use anyhow::Result;
use ratatui::style::{Color, Style};
use trui::*;

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

struct AppState {
    maximize: bool,
    selected_tab: usize,
}

fn main() -> Result<()> {
    App::new(
        AppState {
            maximize: false,
            selected_tab: 0,
        },
        |state| {
            // TODO jittery, maybe there's a better way to do something like this
            let tab = |i: usize| {
                format!("Tab {i}")
                    .border((
                        Borders::RIGHT_WITH_CORNERS | Borders::HORIZONTAL,
                        BorderKind::Rounded,
                    ))
                    .on_click(move |state: &mut AppState| state.selected_tab = i)
                    .weight(low_pass(
                        0.1,
                        if state.selected_tab == i { 2.0 } else { 1.0 },
                    ))
            };
            v_stack((
                button(
                    "Click this button to animate!".fg(Color::Green),
                    |state: &mut AppState| state.maximize = !state.maximize,
                ),
                "Click these tabs to maximize each",
                weighted_h_stack((tab(0), tab(1), tab(2), tab(3), tab(4))),
                "Elastic Title"
                    .fill_max_width((0.2..1.0).elastic_in_out_ease().tween(
                        Duration::from_secs_f64(3.5),
                        if state.maximize { 1.0 } else { -1.0 },
                    ))
                    .border((Borders::HORIZONTAL, BorderKind::ThickStraight)),
                "Quadratic Title"
                    .fill_max_width((0.2..1.0).quadratic_in_out_ease().tween(
                        Duration::from_secs_f64(1.0),
                        if state.maximize { 1.0 } else { -1.0 },
                    ))
                    .border((Borders::HORIZONTAL, BorderKind::ThickStraight)),
                h_stack((
                    "This box resizes"
                        .fill_max_size(low_pass(0.05, if state.maximize { 0.2 } else { 0.8 }))
                        .border(Style::default().fg(Color::Red)),
                    "same, but different"
                        .border(BorderKind::Rounded)
                        .fill_max_height(lerp(
                            (0.1..1.0).quadratic_in_out_ease(),
                            low_pass(0.05, if state.maximize { 0.7 } else { 0.1 }),
                        )),
                )),
                "Expanding Title 2"
                    .fill_max_width((0.2..1.0).reverse().quadratic_out_ease().tween(
                        Duration::from_secs_f64(1.5),
                        if state.maximize { 1.0 } else { -1.0 },
                    ))
                    .border((Borders::HORIZONTAL, BorderKind::DoubleStraight)),
            ))
        },
    )
    .run()
}
