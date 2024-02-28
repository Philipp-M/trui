use anyhow::Result;
use futures::Stream;
use ratatui::style::Color;
use std::time::Duration;
use tokio::time::{interval, sleep};
use trui::*;

#[path = "./shared/logging.rs"]
mod logging;

pub fn words_stream(input: &str) -> impl Stream<Item = String> + Send {
    let words = input
        .split_whitespace()
        .map(String::from)
        .collect::<Vec<_>>();

    let interval = interval(Duration::from_millis(33));

    futures_util::stream::unfold(
        (interval, words.into_iter()),
        move |(mut interval, mut words)| async move {
            if let Some(word) = words.next() {
                for _ in word.chars() {
                    interval.tick().await;
                }
                Some((word, (interval, words)))
            } else {
                None
            }
        },
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = crate::logging::setup_logging(tracing::Level::DEBUG)?;

    App::new(String::new(), |app_state| {
        v_stack((
            "Click me for some non-sense"
                .border(BorderKind::Rounded)
                .on_hover_fg(Color::Blue)
                .on_pressed_fg(Color::Red)
                .on_click(stream(
                    |_, ()| {
                        words_stream(
                            "It is the expectation, indeed the express intention, \
                            that the particular arrangement of words \
                            and phrases we are currently discussing, \
                            which for the sake of clarity we shall refer to as 'this sentence', \
                            should not make its appearance with undue haste. Rather, \
                            it is the desired outcome that its arrival should be postponed, \
                            deferred, or in other words, \
                            that it should come in a manner \
                            that can be accurately described as 'delayed'.",
                        )
                    },
                    |app_state: &mut String, message: StreamMessage<String>| match message {
                        StreamMessage::Begin(word) => {
                            *app_state = word;
                        }
                        StreamMessage::Update(word) => {
                            *app_state += " ";
                            *app_state += &word;
                        }
                        StreamMessage::Finished => (),
                    },
                )),
            "Click me for some non-sense, but only once"
                .border(BorderKind::Rounded)
                .on_hover_fg(Color::Blue)
                .on_pressed_fg(Color::Green)
                .on_click(defer(
                    |app_state: &mut String, ()| {
                        let app_state_empty = app_state.is_empty();

                        async move {
                            sleep(Duration::from_secs(1)).await;
                            format!(
                                " This message came in delayed! {}",
                                if app_state_empty { "" } else { "again! " }
                            )
                        }
                    },
                    |app_state: &mut String, message: String| {
                        *app_state += &message;
                    },
                )),
            app_state
                .clone() //.wrapped()
                .border(())
                .fg(if app_state.is_empty() {
                    Color::White
                } else {
                    Color::Blue
                }),
        ))
    })
    .await
    .run()
    .await
}
