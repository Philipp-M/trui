use anyhow::Result;
use futures::Stream;
use ratatui::style::Color;
use std::time::Duration;
use tokio::time::interval;
use trui::*;

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

fn main() -> Result<()> {
    App::new(String::new(), |text| {
        v_stack((
            block("Click me for some non-sense")
                .with_borders(BorderKind::Rounded)
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
                    |text: &mut String, message: StreamMessage<String>| match message {
                        StreamMessage::Begin(word) => {
                            *text = word;
                        }
                        StreamMessage::Update(word) => {
                            *text += " ";
                            *text += &word;
                        }
                        StreamMessage::Finished => (),
                    },
                )),
            block(text.clone().wrapped())
                .with_borders(())
                .fg(if text.is_empty() {
                    Color::White
                } else {
                    Color::Blue
                }),
        ))
    })
    .run()
}
