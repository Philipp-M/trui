use anyhow::Result;
use ratatui::style::Color;
use trui::{
    memoize, v_stack, AnyView, App, BorderKind, Borders, EventHandler, IntoBoxedView, Styleable,
    View, ViewExt,
};

#[path = "./shared/logging.rs"]
mod logging;

// TODO this basic logic (hover, styling etc.) should probably be its own widget (state)...
pub fn button<T>(
    label: impl Into<String>,
    block_color: Color,
    click_cb: impl EventHandler<T>,
    hover_cb: impl EventHandler<T>,
    hover_lost_cb: impl EventHandler<T>,
) -> impl View<T> + Styleable {
    label
        .into()
        .border(Borders::ALL)
        .fg(block_color)
        .on_click(click_cb)
        .on_hover(hover_cb)
        .on_blur_hover(hover_lost_cb)
}

fn rainbow(normalized_value: f32) -> Color {
    let hue = (normalized_value.min(1.0).max(0.0) * 360.0) as u32;
    let chroma = 255.0;
    let x = (1.0 - ((hue as f32 / 60.0) % 2.0 - 1.0).abs()) * chroma;

    let (red, green, blue) = match hue {
        0..=59 => (chroma, x, 0.0),
        60..=119 => (x, chroma, 0.0),
        120..=179 => (0.0, chroma, x),
        180..=239 => (0.0, x, chroma),
        240..=299 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };

    Color::Rgb(red as u8, green as u8, blue as u8)
}

pub fn rainbow_blocks<T: 'static>(
    content: impl IntoBoxedView<T>,
    count: usize,
) -> Box<dyn AnyView<T>> {
    let mut view = content.boxed();
    for i in 0..count {
        let color = rainbow((i as f32 / (count - 1) as f32 + 0.001).max(0.0).min(1.0));
        view = view.border(BorderKind::Rounded).fg(color).boxed();
    }
    view
}

struct AppState {
    pub count: usize,
    pub current_button_color: Color,
    pub current_button_color2: Color,
}

impl AppState {
    fn change_color(&mut self) {
        self.current_button_color = Color::Rgb(rand::random(), rand::random(), rand::random());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = crate::logging::setup_logging(tracing::Level::DEBUG)?;

    App::new(
        AppState {
            count: 10,
            current_button_color: Color::Gray,
            current_button_color2: Color::Cyan,
        },
        |state| {
            v_stack((
                // for such simple things, I don't think it's more ergonomic to wrap these things into functions (with all the callbacks at least)...
                button(
                    "Click me",
                    state.current_button_color,
                    |state: &mut AppState| {
                        state.current_button_color = Color::Magenta;
                        state.count += 1;
                        state.change_color();
                    },
                    |state: &mut AppState| {
                        state.current_button_color = Color::Green;
                    },
                    |state: &mut AppState| {
                        state.current_button_color = Color::Cyan;
                    },
                ),
                // this is (almost) the equivalent...
                memoize(
                    (state.current_button_color2, state.count),
                    |(button_color, count)| {
                        tracing::debug!(
                            "This will be printed on every change\
                             of state.current_button_color2 or state.count"
                        );
                        let count = *count;
                        let v: Box<dyn AnyView<_>> = if count <= 10 {
                            Box::new(format!(
                                "Nothing interesting here to see, count is low at {}",
                                count
                            ))
                        } else if count > 10 && count < 42 {
                            Box::new(format!("Count is bigger than 10 ({})", count).fg(Color::Gray))
                        } else if count == 42 {
                            Box::new("You have found the sense of life!".fg(Color::Green))
                        } else {
                            let color = Color::Rgb(
                                255usize.saturating_sub(count * 2) as u8,
                                255usize.saturating_sub(count * 2) as u8,
                                255usize.saturating_sub(count * 2) as u8,
                            );
                            Box::new("Everything's downhill from here on...".fg(color))
                        };
                        v.border(BorderKind::DoubleStraight)
                            .fg(*button_color)
                            .on_click(|state: &mut AppState| {
                                state.current_button_color2 = Color::Red;
                                state.count = state.count.saturating_sub(1);
                            })
                            .on_hover(|state: &mut AppState| {
                                state.current_button_color2 = Color::Yellow;
                            })
                            .on_blur_hover(|state: &mut AppState| {
                                state.current_button_color2 = Color::Gray;
                            })
                    },
                ),
                rainbow_blocks("Rainbow blocks!", state.count),
            ))
        },
    )
    .await
    .run()
    .await
}
