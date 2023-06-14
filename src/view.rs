mod adapt;
mod border;
mod common;
mod core;
mod events;
mod linear_layout;
mod text;

pub use xilem_core::{Id, IdPath, VecSplice};

pub use self::core::{AnyView, Boxed, Cx, View, ViewMarker, ViewSequence};
pub use adapt::{Adapt, AdaptThunk};
pub use border::{border, Border};
pub use common::{Borders, Styleable};
pub use events::{Clickable, Hoverable, OnClick, OnHover};
pub use linear_layout::{h_stack, v_stack, LinearLayout};
pub use text::Text;
