use anyhow::Result;
use ratatui::style::{Color, Style};
use trui::*;

// TODO layouting is not really optimal yet (needs to be configurable, a good abstraction on top of taffy likely)
fn main() -> Result<()> {
    App::new(
        |_| (),
        |(), _| {
            s(|| {
                tracing::debug!("This should only be run only once!");
                block(h_stack((
                    v_stack((
                        block(("With".fg(Color::Yellow), " background").wrapped())
                            .bg(Color::LightYellow),
                        block("text inside block").with_borders(BorderKind::Straight),
                    )),
                    v_stack((
                        block("Styled title".bg(Color::Red).fg(Color::White)).bg(Color::LightCyan),
                        block(v_stack((
                            "With styled borders and doubled borders",
                            block("Block inside block").with_borders(BorderKind::Straight),
                        )))
                        .with_borders((
                            Borders::VERTICAL,
                            Style::default().fg(Color::Cyan),
                            BorderKind::DoubleStraight,
                        )),
                    )),
                )))
                .with_borders(BorderKind::Rounded)
            })
        },
    )
    .run()
}
