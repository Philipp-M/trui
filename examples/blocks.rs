use std::sync::Arc;

use anyhow::Result;
use ratatui::style::{Color, Style};
use trui::*;

#[path = "./shared/logging.rs"]
mod logging;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = crate::logging::setup_logging(tracing::Level::DEBUG)?;

    let view = Arc::new(
        weighted_h_stack((
            "text inside block"
                .fill_max_height(0.5)
                .border(BorderKind::Straight)
                .fill_max_width(1.0)
                .margin((Position::VERTICAL, 1))
                .weight(1.5),
            v_stack((
                "Styled title".bg(Color::Red).fg(Color::White),
                v_stack((
                    "With styled borders and doubled borders",
                    "Block inside block"
                        .border(BorderKind::Straight)
                        .fill_max_size(0.7)
                        .margin(2),
                ))
                .margin((Position::TOP, 1))
                .margin((Position::HORIZONTAL, 2))
                .fill_max_height(1.0)
                .border((
                    Borders::VERTICAL,
                    Style::default().fg(Color::Cyan),
                    BorderKind::DoubleStraight,
                )),
            )),
        ))
        .border(BorderKind::Rounded)
        .margin(1),
    );
    App::new((), move |()| view.clone()).await.run().await
}
