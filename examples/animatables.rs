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
            let tab = |i: usize| {
                format!("Tab {i}")
                    .border((
                        Borders::RIGHT_WITH_CORNERS | Borders::HORIZONTAL,
                        BorderKind::Rounded,
                    ))
                    .on_click(move |state: &mut AppState| state.selected_tab = i)
                    .weight(low_pass(
                        0.2,
                        if state.selected_tab == i { 2.0 } else { 1.0 },
                    ))
            };
            v_stack((
                button(
                    "Click this button to toggle the size of the red box below".fg(Color::Green),
                    |state: &mut AppState| state.maximize = !state.maximize,
                ),
                "Click these tabs to maximize each",
                weighted_h_stack((tab(0), tab(1), tab(2), tab(3), tab(4))),
                "This box resizes"
                    // lerp(0.1, 1.0, low_pass(0.1, if state.maximize { 0.3 } else { 0.6 }))
                    .fill_max_size(low_pass(0.1, if state.maximize { 0.2 } else { 0.8 }))
                    .border(Style::default().fg(Color::Red)),
                "Expanding Title"
                    .fill_max_width(low_pass(0.2, if state.maximize { 1.0 } else { 0.2 }))
                    .border((Borders::HORIZONTAL, BorderKind::DoubleStraight)),
            ))
        },
    )
    .run()
}
