use std::sync::Arc;

use anyhow::Result;
use ratatui::style::{Color, Style};
use trui::*;

fn main() -> Result<()> {
    let view = Arc::new(
        block(weighted_h_stack((
            v_stack((
                // block(("With".fg(Color::Yellow), " background").wrapped()).bg(Color::LightYellow),
                block("text inside block").with_borders(BorderKind::Straight),
            ))
            .weight(1.5),
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
        .with_borders(BorderKind::Rounded),
    );
    App::new((), move |()| view.clone()).run()
}
