use std::sync::Arc;

use anyhow::Result;
use ratatui::style::Color;
use trui::*;

fn main() -> Result<()> {
    let entry = |num| {
        weighted_h_stack((
            format!("{num}").weight(0.05),
            "Avatar"
                .border(BorderKind::Straight)
                .fill_max_height(1.0) // TODO not working
                .weight(0.3),
            v_stack((
                "Description"
                    .fg(Color::Red)
                    .margin((2, Position::LEFT))
                    .margin((1, Position::VERTICAL)),
                "Lorem ipsum dolor sit amet,\n\
                 consectetur adipiscing elit,\n\
                 sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n\
                 Ut enim ad minim veniam,\n\
                 quis nostrud exercitation ullamco laboris nisi ut aliquip \
                 ex ea commodo consequat.\n\
                 Duis aute irure dolor in reprehenderit in voluptate velit esse \
                 cillum dolore eu fugiat nulla pariatur.\n\
                 Excepteur sint occaecat cupidatat non proident,\n\
                 sunt in culpa qui officia deserunt mollit anim id est laborum."
                    .fg(Color::Blue),
            )),
        ))
        .border(BorderKind::Rounded)
        // .margin(1)
    };

    fn list_with_every_nth_red_otherwise_blue<T, A>(
        n: usize,
        views: impl Iterator<Item = impl View<T, A>>,
    ) -> Vec<impl View<T, A>> {
        views
            .enumerate()
            .map(|(i, v)| {
                if i % n == 0 {
                    OneOf2::A(
                        v.border(Borders::VERTICAL)
                            .fg(Color::Red)
                            .on_hover_bg(Color::Magenta),
                    )
                } else {
                    OneOf2::B(
                        v.border(Borders::VERTICAL)
                            .fg(Color::Blue)
                            .on_hover_bg(Color::Yellow),
                    )
                }
            })
            .collect()
    }
    let list = list_with_every_nth_red_otherwise_blue(2, (0..10).map(entry));
    let scroll_view = Arc::new(scroll_view(v_stack(list)));

    // let scroll_view = Arc::new(scroll_view());
    App::new((), move |()| scroll_view.clone()).run()
}
