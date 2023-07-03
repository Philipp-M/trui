use anyhow::Result;
use ratatui::style::Color;
use trui::{
    block, v_stack, AnyView, App, BorderKind, Borders, BoxedView, Clickable, Hoverable,
    IntoBoxedView, Styleable, View, ViewMarker,
};

// TODO this basic logic (hover, styling etc.) should probably be its own widget (state)...
pub fn button<T>(
    label: impl Into<String>,
    block_color: Color,
    click_cb: impl Fn(&mut T) + Send,
    hover_cb: impl Fn(&mut T) + Send,
    hover_lost_cb: impl Fn(&mut T) + Send,
) -> impl View<T> + ViewMarker + Styleable<T> {
    block(label.into())
        .with_borders(Borders::ALL)
        .fg(block_color)
        .on_click(click_cb)
        .on_hover(hover_cb)
        .on_blur_hover(hover_lost_cb)
}

// Thanks ChatGPT...
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

pub fn rainbow_blocks<T: 'static>(content: impl IntoBoxedView<T>, count: usize) -> BoxedView<T> {
    let mut view = content.boxed();
    for i in 0..count {
        let color = rainbow((i as f32 / (count - 1) as f32 + 0.001).max(0.0).min(1.0));
        view = block(view)
            .with_borders(BorderKind::Rounded)
            .fg(color)
            .boxed();
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

fn main() -> Result<()> {
    App::new(
        AppState {
            count: 10,
            current_button_color: Color::Gray,
            current_button_color2: Color::Cyan,
        },
        |state| {
            let v: BoxedView<_> = if state.count <= 10 {
                Box::new(format!(
                    "Nothing interesting here to see, count is low at {}",
                    state.count
                ))
            } else if state.count > 10 && state.count < 42 {
                Box::new(format!("Count is bigger than 10 ({})", state.count).fg(Color::Gray))
            } else if state.count == 42 {
                Box::new(block("You have found the sense of life!".fg(Color::Green)).fg(Color::Red))
            } else {
                let color = Color::Rgb(
                    255usize.saturating_sub(state.count * 2) as u8,
                    255usize.saturating_sub(state.count * 2) as u8,
                    255usize.saturating_sub(state.count * 2) as u8,
                );
                Box::new(block("Everything's downhill from here on...".fg(color)).fg(color))
            };

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
                block(v)
                    .with_borders(BorderKind::DoubleStraight)
                    .fg(state.current_button_color2)
                    .on_click(|state: &mut AppState| {
                        state.current_button_color2 = Color::Red;
                        state.count = state.count.saturating_sub(1);
                    })
                    .on_hover(|state: &mut AppState| {
                        state.current_button_color2 = Color::Yellow;
                    })
                    .on_blur_hover(|state: &mut AppState| {
                        state.current_button_color2 = Color::Gray;
                    }),
                rainbow_blocks("Rainbow blocks!", state.count),
            ))
        },
    )
    .run()
}
