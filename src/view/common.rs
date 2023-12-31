use bitflags::bitflags;
use ratatui::{
    style::{Color, Modifier, Style},
    symbols,
};

use super::View;

bitflags! {
    /// Bitflags that can be composed to set the visible borders essentially on the block widget.
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Borders: u8 {
        /// Show no border (default)
        const NONE       = 0b0000;
        /// Show the top border
        const TOP        = 0b0001;
        /// Show the right border
        const RIGHT      = 0b0010;
        /// Show the bottom border
        const BOTTOM     = 0b0100;
        /// Show the left border
        const LEFT       = 0b1000;
        /// Show all borders
        const ALL        = Self::TOP.bits() | Self::RIGHT.bits() | Self::BOTTOM.bits() | Self::LEFT.bits();
        /// Show top and bottom borders
        const HORIZONTAL = Self::BOTTOM.bits() | Self::TOP.bits();
        /// Show left and right borders
        const VERTICAL   = Self::LEFT.bits() | Self::RIGHT.bits();
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderStyle {
    pub add_borders: Borders,
    pub sub_borders: Borders,
    pub style: Style, // TODO generally find a better name for "Style" as it only applies modifiers and colors for each character
    pub kind: Option<BorderKind>,
}

// naming is hard...
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct BorderStyles(pub(crate) Vec<BorderStyle>);

impl BorderStyles {
    pub fn has_borders(&self, borders: Borders) -> bool {
        let enabled_borders = self.0.iter().fold(Borders::default(), |b, styles| {
            (b & styles.sub_borders.complement()) | styles.add_borders
        });
        enabled_borders.contains(borders)
    }

    /// if all of the borders are set in one style "frame" (i.e. with widget.with_borders(<borders>)), it returns the defined border kind
    pub fn border_kind(&self, borders: Borders) -> BorderKind {
        self.0.iter().fold(BorderKind::default(), |kind, style| {
            if style.add_borders.contains(borders) {
                if let Some(new_kind) = style.kind {
                    new_kind
                } else {
                    kind
                }
            } else if style.sub_borders.contains(borders) {
                BorderKind::default()
            } else {
                kind
            }
        })
    }

    pub fn style(&self, borders: Borders) -> Style {
        let mut style = Style::default();
        for border_style in self.0.iter().rev() {
            if border_style.add_borders.contains(borders) {
                style = border_style.style.patch(style);
            }
            if border_style.sub_borders.contains(borders) {
                return style;
            }
        }
        style
    }

    pub fn symbols(&self, borders: Borders) -> symbols::line::Set {
        match self.border_kind(borders) {
            BorderKind::Straight => symbols::line::NORMAL,
            BorderKind::Rounded => symbols::line::ROUNDED,
            BorderKind::DoubleStraight => symbols::line::DOUBLE,
            BorderKind::ThickStraight => symbols::line::THICK,
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BorderKind {
    #[default]
    Straight = 1,
    Rounded,
    DoubleStraight,
    ThickStraight,
}

pub trait Styleable<T, A = ()> {
    type Output: View<T, A>;
    fn fg(self, color: Color) -> Self::Output;
    fn bg(self, color: Color) -> Self::Output;
    fn style(self, style: Style) -> Self::Output;
    fn modifier(self, modifier: Modifier) -> Self::Output;
    fn current_style(&self) -> Style;
}
