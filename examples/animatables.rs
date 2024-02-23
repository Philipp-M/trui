use std::{f64::consts::PI, time::Duration};

use anyhow::Result;
use ratatui::style::{Color, Style};
use trui::logging::setup_logging;
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
    let _ = setup_logging(tracing::Level::DEBUG)?;

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

            let play_speed = if state.maximize { 1.0 } else { -1.0 };
            v_stack((
                button(
                    "Click this button to animate!".fg(Color::Green),
                    |state: &mut AppState| state.maximize = !state.maximize,
                ),
                "Click these tabs to maximize each",
                weighted_h_stack((tab(0), tab(1), tab(2), tab(3), tab(4))),
                "Elastic Title"
                    .fill_max_width(
                        (0.2..1.0)
                            .duration(Duration::from_secs(3))
                            .elastic_in_out_ease()
                            .play(play_speed),
                    )
                    .border((Borders::HORIZONTAL, BorderKind::ThickStraight)),
                "Quadratic Title"
                    .fill_max_width((0.2..1.0).quadratic_in_out_ease().play(play_speed))
                    .border((Borders::HORIZONTAL, BorderKind::ThickStraight)),
                "This does some weird stuff"
                    .fill_max_width(
                        (
                            (0.2..1.0).duration(Duration::from_secs(1)),
                            (1.0..0.5)
                                .duration(Duration::from_secs(2))
                                .quadratic_in_out_ease(),
                        )
                            .duration(Duration::from_secs(2)) // Note the ratio of the durations specified above stays the same
                            .play(play_speed),
                    )
                    .border((
                        Borders::HORIZONTAL,
                        BorderKind::ThickStraight,
                        Style::default().fg(Color::Blue),
                    )),
                "This is oscillating"
                    .fill_max_width(
                        (0.0..(4.0 * PI))
                            .map(|v| v.cos() * 0.3 + 0.7)
                            .duration(Duration::from_secs(4))
                            .play(play_speed),
                    )
                    .border((Borders::HORIZONTAL, BorderKind::ThickStraight)),
                h_stack((
                    "This box resizes"
                        .fill_max_size(low_pass(0.05, if state.maximize { 0.2 } else { 0.8 }))
                        .border(Style::default().fg(Color::Red)),
                    "same, but different"
                        .border(BorderKind::Rounded)
                        .fill_max_height(
                            (0.1..1.0)
                                .quadratic_in_out_ease()
                                .lerp(low_pass(0.05, if state.maximize { 0.7 } else { 0.1 })),
                        ),
                )),
                "Expanding Title 2"
                    .fill_max_width(
                        (0.2..1.0)
                            .reverse()
                            .map_ease(|r| 1.0 - r) // cancels out the reverse(). Why? Because it's possible...
                            .duration(Duration::from_millis(500))
                            .quadratic_out_ease()
                            .play(play_speed),
                    )
                    .border((Borders::HORIZONTAL, BorderKind::DoubleStraight)),
            ))
        },
    )
    .run()
}
