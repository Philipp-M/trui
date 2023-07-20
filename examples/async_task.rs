use std::time::Duration;
use time::{format_description, OffsetDateTime, UtcOffset};

use anyhow::Result;
use ratatui::style::{Color, Style};
use tokio::time::sleep;
use trui::*;
use xilem_core::MessageResult;

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

struct AppState {
    pub current_time: OffsetDateTime,
}

enum Message {
    UpdateCurrentTime(OffsetDateTime),
}

impl<A> StateUpdater<A, Message> for AppState {
    fn update(&mut self, message: Message) -> MessageResult<A> {
        match message {
            Message::UpdateCurrentTime(current_time) => {
                if current_time != self.current_time {
                    self.current_time = current_time;
                    MessageResult::RequestRebuild
                } else {
                    MessageResult::Nop
                }
            }
        }
    }
}

fn main() -> Result<()> {
    // There seems to be a soundness bug with current_local_offset in multi threaded environments in linux...
    // that's why the offset is calculated here...
    let utc_offset = UtcOffset::current_local_offset()?;

    App::new(
        |cx| {
            cx.clone().runtime.spawn(async move {
                loop {
                    sleep(Duration::from_secs(1)).await;
                    let current_time = OffsetDateTime::now_utc().to_offset(utc_offset);
                    cx.update_state(Message::UpdateCurrentTime(current_time));
                }
            });

            AppState {
                current_time: OffsetDateTime::now_utc().to_offset(utc_offset),
            }
        },
        |state, _| {
            let format_time = |format| {
                state
                    .current_time
                    .format(&format_description::parse(format).unwrap())
                    .unwrap()
            };
            block(
                (
                    format_time("[year]-[month]-[day] ").fg(Color::Yellow),
                    format_time("[hour]:[minute]:[second]").fg(Color::Blue),
                )
                    .wrapped(),
            )
            .with_borders(())
        },
    )
    .run()
}
