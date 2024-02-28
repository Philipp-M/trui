use std::marker::PhantomData;

use crate::{
    widget::{self, ChangeFlags},
    BorderStyle,
};

use super::{BorderKind, Borders, Cx, Styleable, View, ViewMarker};
use ratatui::style::{Color, Style};
use xilem_core::MessageResult;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Border<V, T, A> {
    pub(crate) content: V,
    pub(crate) borders: Borders,
    pub(crate) kind: BorderKind,
    pub(crate) style: Style,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, V> ViewMarker for Border<V, T, A> {}

impl<T, A, V: View<T, A>> View<T, A> for Border<V, T, A> {
    type State = V::State;

    type Element = widget::Border;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, state, element) = self.content.build(cx);
        let element = widget::Border::new(element, self.borders, self.style, self.kind);
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> crate::widget::ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        changeflags |= element.set_borders(self.borders);
        changeflags |= element.set_style(self.style);
        changeflags |= element.set_kind(self.kind);

        let content_el = element
            .content
            .downcast_mut()
            .expect("The border content widget changed its type, this should never happen!");

        let content_changeflags = self
            .content
            .rebuild(cx, &prev.content, id, state, content_el);
        let _ = element.content.mark(content_changeflags);
        changeflags | content_changeflags
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.content.message(id_path, state, message, app_state)
    }
}

impl<V, T, A> Styleable for Border<V, T, A> {
    type Output = Self;

    fn fg(mut self, color: Color) -> Self::Output {
        self.style.fg = Some(color);
        self
    }

    fn bg(mut self, color: Color) -> Self::Output {
        self.style.bg = Some(color);
        self
    }

    fn modifier(self, modifier: ratatui::style::Modifier) -> Self::Output {
        let _ = self.style.add_modifier(modifier);
        self
    }

    fn current_style(&self) -> Style {
        self.style
    }

    fn style(mut self, style: Style) -> Self::Output {
        self.style = style;
        self
    }
}

impl From<()> for BorderStyle {
    fn from(_: ()) -> Self {
        Borders::ALL.into()
    }
}

impl From<Borders> for BorderStyle {
    fn from(borders: Borders) -> Self {
        BorderStyle {
            borders,
            ..Default::default()
        }
    }
}

// TODO enable all borders?
impl From<Style> for BorderStyle {
    fn from(style: Style) -> Self {
        BorderStyle {
            borders: Borders::ALL,
            style,
            ..Default::default()
        }
    }
}

impl From<BorderKind> for BorderStyle {
    fn from(kind: BorderKind) -> Self {
        BorderStyle {
            borders: Borders::ALL,
            kind,
            ..Default::default()
        }
    }
}

impl From<(Borders, Style)> for BorderStyle {
    fn from((borders, style): (Borders, Style)) -> Self {
        BorderStyle {
            borders,
            style,
            ..Default::default()
        }
    }
}

impl From<(Style, Borders)> for BorderStyle {
    fn from((style, borders): (Style, Borders)) -> Self {
        BorderStyle {
            borders,
            style,
            ..Default::default()
        }
    }
}

impl From<(Borders, BorderKind)> for BorderStyle {
    fn from((borders, kind): (Borders, BorderKind)) -> Self {
        BorderStyle {
            borders,
            kind,
            ..Default::default()
        }
    }
}

impl From<(BorderKind, Borders)> for BorderStyle {
    fn from((kind, borders): (BorderKind, Borders)) -> Self {
        BorderStyle {
            borders,
            kind,
            ..Default::default()
        }
    }
}

impl From<(Style, Borders, BorderKind)> for BorderStyle {
    fn from((style, borders, kind): (Style, Borders, BorderKind)) -> Self {
        BorderStyle {
            borders,
            style,
            kind,
        }
    }
}

impl From<(Borders, Style, BorderKind)> for BorderStyle {
    fn from((borders, style, kind): (Borders, Style, BorderKind)) -> Self {
        BorderStyle {
            borders,
            style,
            kind,
        }
    }
}

impl From<(Borders, BorderKind, Style)> for BorderStyle {
    fn from((borders, kind, style): (Borders, BorderKind, Style)) -> Self {
        BorderStyle {
            borders,
            style,
            kind,
        }
    }
}

impl From<(BorderKind, Borders, Style)> for BorderStyle {
    fn from((kind, borders, style): (BorderKind, Borders, Style)) -> Self {
        BorderStyle {
            borders,
            style,
            kind,
        }
    }
}

impl From<(BorderKind, Style, Borders)> for BorderStyle {
    fn from((kind, style, borders): (BorderKind, Style, Borders)) -> Self {
        BorderStyle {
            borders,
            style,
            kind,
        }
    }
}

impl From<(Style, BorderKind, Borders)> for BorderStyle {
    fn from((style, kind, borders): (Style, BorderKind, Borders)) -> Self {
        BorderStyle {
            borders,
            style,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ratatui::layout::Size;

    use crate::{test_helper::render_view, ViewExt};

    use super::*;

    struct AppState;

    #[tokio::test]
    async fn simple_border_test() {
        let _guard = crate::test_helper::init_tracing("simple_border_test").unwrap();

        // console_subscriber::init();
        let local_set = tokio::task::LocalSet::new();
        local_set
            .run_until(async {
                let sut = Arc::new("some text".fg(Color::Cyan).border(()));
                let buffer = render_view(
                    Size {
                        width: 15,
                        height: 5,
                    },
                    sut,
                    AppState,
                )
                .await;

                insta::assert_debug_snapshot!(buffer);
            })
            .await
    }

    #[tokio::test]
    async fn too_small_viewport() {
        let _guard = crate::test_helper::init_tracing("too_small_viewport").unwrap();

        let local_set = tokio::task::LocalSet::new();
        local_set
            .run_until(async {
                tracing::debug!("too_small_viewport test");

                let sut = Arc::new("some text".fg(Color::Cyan).border(BorderKind::Straight));
                let buffer = render_view(
                    Size {
                        width: 7,
                        height: 5,
                    },
                    sut,
                    AppState,
                )
                .await;
                insta::assert_debug_snapshot!(buffer);
            })
            .await
    }
}
