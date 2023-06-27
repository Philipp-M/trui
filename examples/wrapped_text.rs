use anyhow::Result;
use ratatui::style::Color;
use trui::*;

fn main() -> Result<()> {
    App::new((), |()| {
        h_stack((
            block(("Different ".fg(Color::Red), "Colors that are wrapped: Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.".fg(Color::Blue)).wrapped()),
            // TODO proper wrapping with new lines etc.
            block("This should be wrapped very soon:\n\n Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum.".wrapped()),
        ))
    })
    .run()
}
