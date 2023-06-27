use std::marker::PhantomData;

use crate::widget::{self, ChangeFlags, StyleableWidget};

use super::{BorderKind, BorderStyle, BorderStyles, Borders, Cx, Styleable, View, ViewMarker};
use ratatui::style::{Color, Style};
use xilem_core::MessageResult;

pub struct Block<T, A, V> {
    content: V,
    phantom: PhantomData<fn() -> (T, A)>,
    border_styles: BorderStyles,
    style: Style, // base style, merged on top of border style currently (overrides attributes if they are defined in border_style)
    fill_with_bg: bool,
    inherit_style: bool,
}

impl<T, A, V> ViewMarker for Block<T, A, V> {}

impl<T, A, V> View<T, A> for Block<T, A, V>
where
    V: View<T, A>,
    V::Element: 'static,
{
    type State = (V::State, xilem_core::Id);

    type Element = widget::Block;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (child_id, state, element) = self.content.build(cx);
            (
                (state, child_id),
                widget::Block::new(
                    element,
                    self.border_styles.clone(),
                    self.style,
                    self.inherit_style,
                    self.fill_with_bg,
                ),
            )
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        (state, child_id): &mut Self::State,
        element: &mut Self::Element,
    ) -> crate::widget::ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        changeflags |= element.set_border_style(&self.border_styles);
        if element.set_style(self.style) {
            changeflags |= ChangeFlags::PAINT;
        }
        changeflags |= element.set_inherit_style(self.inherit_style);
        changeflags |= element.set_fill_with_bg(self.fill_with_bg);

        let element = element
            .content
            .downcast_mut()
            .expect("The border content widget changed its type, this should never happen!");

        changeflags
            | cx.with_id(*id, |cx| {
                self.content
                    .rebuild(cx, &prev.content, child_id, state, element)
            })
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        (state, child_id): &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        if let Some((first, rest_path)) = id_path.split_first() {
            if first == child_id {
                self.content.message(rest_path, state, message, app_state)
            } else {
                xilem_core::MessageResult::Stale(message)
            }
        } else {
            xilem_core::MessageResult::Nop
        }
    }
}

impl<T, A, V> Styleable<T, A> for Block<T, A, V>
where
    V: View<T, A>,
    V::Element: 'static,
{
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
        self.style.add_modifier(modifier);
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

impl<T, A, V> Block<T, A, V> {
    pub fn inherit_style(mut self, inherit: bool) -> Self {
        self.inherit_style = inherit;
        self
    }

    pub fn fill_with_bg(mut self, fill: bool) -> Self {
        self.fill_with_bg = fill;
        self
    }

    // keep it simple for now, but the future offers: extension traits!
    pub fn with_borders(mut self, borders: Borders, style: Style, kind: BorderKind) -> Self {
        self.border_styles.0.push(BorderStyle {
            add_borders: borders,
            sub_borders: Borders::NONE,
            style,
            kind: Some(kind),
        });
        self
    }

    /// reverse previously applied borders
    pub fn without_borders(mut self, borders: Borders) -> Self {
        self.border_styles.0.push(BorderStyle {
            add_borders: Borders::NONE,
            sub_borders: borders,
            style: Style::default(),
            kind: None,
        });
        self
    }
}

pub fn block<T, A, V>(content: V) -> Block<T, A, V> {
    Block {
        content,
        phantom: PhantomData,
        border_styles: Default::default(),
        style: Style::default(),
        inherit_style: false,
        fill_with_bg: true,
    }
}

pub fn bordered_block<T, A, V>(content: V) -> Block<T, A, V> {
    block(content).with_borders(Borders::ALL, Style::default(), BorderKind::Rounded)
}
