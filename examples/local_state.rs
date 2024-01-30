use anyhow::Result;
use trui::*;

struct AppState {
    count: i32,
    local_state: i32,
}

// Multiple ways to manage "local state"

// completely private from the application using `WithState`
fn button_with_state<T>(view: impl View<T>) -> impl View<T> {
    view.with_state(
        || 123,
        |view, n| {
            block(v_stack((view, n.to_string())))
                .with_borders(BorderKind::ThickStraight)
                .on_click(|(_, l): &mut (Handle<T>, i32)| *l += 1)
        },
    )
}

// Via an accessor function that returns a reference to the local state
fn button_with_state_accessor<T>(
    state: &mut T,
    view: impl View<T>,
    access_local_state: impl Fn(&mut T) -> &mut i32 + Send + Sync,
) -> impl View<T> {
    block(v_stack((view, access_local_state(state).to_string())))
        .with_borders(BorderKind::Straight)
        .on_click(move |state: &mut T| *access_local_state(state) += 1)
}

// This has the disadvantage, that the composed view has to be in a closure (and thus state may be more difficult to use)
fn button_use_state<T, V: View<(Handle<T>, i32)>>(
    view: impl Fn() -> V + Send + Sync,
) -> impl View<T> {
    use_state(
        || 123,
        move |n| {
            block(v_stack((view(), n.to_string())))
                .with_borders(BorderKind::DoubleStraight)
                .on_click(|(_, n): &mut (Handle<T>, i32)| *n += 1)
        },
    )
}

fn main() -> Result<()> {
    App::new(
        AppState {
            count: 0,
            local_state: 123,
        },
        |state| {
            let count = state.count;
            v_stack((
                block(format!("Increment the global state: {}", state.count))
                    .with_borders(BorderKind::Rounded)
                    .on_click(|state: &mut AppState| state.count += 1),
                button_with_state(format!("With local state, app_state: {}", state.count)),
                button_with_state_accessor(
                    state,
                    format!("Without local state, app_state: {}", state.count),
                    |state| &mut state.local_state,
                ),
                button_use_state(move || {
                    format!("Increment this, global count: {count}").fg(Color::Red)
                }),
            ))
        },
    )
    .run()
}
