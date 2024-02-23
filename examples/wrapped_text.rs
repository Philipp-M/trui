use anyhow::Result;
use trui::logging::setup_logging;
// use ratatui::style::Color;
// use trui::*;

// TODO this currently doesn't work anymore
fn main() -> Result<()> {
    let _ = setup_logging(tracing::Level::DEBUG)?;
    // App::new((), |()| {
    //     h_stack((
    //         block(("Different ".fg(Color::Red), "Colors that are wrapped: Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.".fg(Color::Blue)).wrapped()),
    //         // TODO proper wrapping with new lines etc.
    //         block("This should be wrapped very soon:\n\n Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum.".wrapped()),
    //     ))
    // })
    // .run()
    Ok(())
}
