use std::sync::Arc;

use bitflags::bitflags;
use ratatui::{
    style::{Color, Modifier, Style},
    symbols,
};

bitflags! {
    /// Bitflags that can be composed to set the visible borders essentially on the block widget.
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Borders: u8 {
        /// Show no border (default)
        const NONE                    = 0b00000000;

        /// Show the top border
        const TOP                     = 0b00000001;
        /// Show the right border
        const RIGHT                   = 0b00000010;
        /// Show the bottom border
        const BOTTOM                  = 0b00000100;
        /// Show the left border
        const LEFT                    = 0b00001000;

        /// Show the top left corner
        const TOP_LEFT_CORNER         = 0b00010000;
        /// Show the top right corner
        const TOP_RIGHT_CORNER        = 0b00100000;
        /// Show the bottom left corner
        const BOTTOM_LEFT_CORNER      = 0b01000000;
        /// Show the bottom right corner
        const BOTTOM_RIGHT_CORNER     = 0b10000000;

        /// Show the top corners
        const TOP_CORNERS             = Self::TOP_LEFT_CORNER.bits() | Self::TOP_RIGHT_CORNER.bits();
        /// Show the bottom corners
        const BOTTOM_CORNERS          = Self::BOTTOM_LEFT_CORNER.bits() | Self::BOTTOM_RIGHT_CORNER.bits();
        /// Show the left corners
        const LEFT_CORNERS            = Self::TOP_LEFT_CORNER.bits() | Self::BOTTOM_LEFT_CORNER.bits();
        /// Show the right corners
        const RIGHT_CORNERS           = Self::TOP_RIGHT_CORNER.bits() | Self::BOTTOM_RIGHT_CORNER.bits();

        /// Show the top border including corners
        const TOP_WITH_CORNERS        = Self::TOP.bits() | Self::TOP_CORNERS.bits();
        /// Show the right border including corners
        const RIGHT_WITH_CORNERS      = Self::RIGHT.bits() | Self::RIGHT_CORNERS.bits();
        /// Show the bottom border including corners
        const BOTTOM_WITH_CORNERS     = Self::BOTTOM.bits() | Self::BOTTOM_CORNERS.bits();
        /// Show the left border including corners
        const LEFT_WITH_CORNERS       = Self::LEFT.bits() | Self::LEFT_CORNERS.bits();

        /// Show top and bottom borders
        const HORIZONTAL              = Self::BOTTOM.bits() | Self::TOP.bits();
        /// Show top and bottom borders including their corners
        const HORIZONTAL_WITH_CORNERS = Self::BOTTOM_WITH_CORNERS.bits() | Self::TOP_WITH_CORNERS.bits();
        /// Show left and right borders
        const VERTICAL                = Self::LEFT.bits() | Self::RIGHT.bits();
        /// Show left and right borders including their corners
        const VERTICAL_WITH_CORNERS   = Self::RIGHT_WITH_CORNERS.bits() | Self::LEFT_WITH_CORNERS.bits();

        /// Show all borders
        const ALL_BORDERS             = Self::HORIZONTAL.bits() | Self::VERTICAL.bits();
        /// Show all corners
        const ALL_CORNERS             = Self::TOP_CORNERS.bits() | Self::BOTTOM_CORNERS.bits();
        /// Show all borders including corners
        const ALL                     = Self::ALL_BORDERS.bits() | Self::ALL_CORNERS.bits();
    }
}

bitflags! {
    /// Bitflags that can be composed to set the visible borders essentially on the block widget.
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Position: u8 {
        const TOP        = 0b0001;
        const RIGHT      = 0b0010;
        const BOTTOM     = 0b0100;
        const LEFT       = 0b1000;
        const HORIZONTAL = Self::LEFT.bits() | Self::RIGHT.bits();
        const VERTICAL   = Self::TOP.bits() | Self::BOTTOM.bits();
        const ALL        = Self::HORIZONTAL.bits() | Self::VERTICAL.bits();
    }
}

bitflags! {
    /// Bitflags that can be composed to set the visible borders essentially on the block widget.
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Fill: u8 {
        const WIDTH      = 0b0001;
        const HEIGHT     = 0b0010;
        const ALL        = Self::WIDTH.bits() | Self::HEIGHT.bits();
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderStyle {
    pub borders: Borders,
    pub kind: BorderKind,
    pub style: Style, // TODO generally find a better name for "Style" as it only applies modifiers and colors for each character
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

impl BorderKind {
    pub fn symbols(self) -> symbols::line::Set {
        match self {
            BorderKind::Straight => symbols::line::NORMAL,
            BorderKind::Rounded => symbols::line::ROUNDED,
            BorderKind::DoubleStraight => symbols::line::DOUBLE,
            BorderKind::ThickStraight => symbols::line::THICK,
        }
    }
}

pub trait Styleable {
    type Output;
    fn fg(self, color: Color) -> Self::Output;
    fn bg(self, color: Color) -> Self::Output;
    fn style(self, style: Style) -> Self::Output;
    fn modifier(self, modifier: Modifier) -> Self::Output;
    fn current_style(&self) -> Style;
}

// TODO not super efficient, as the content of the Arc has to be cloned, is there a better solution?
impl<V: Styleable + Clone> Styleable for Arc<V> {
    type Output = Arc<V::Output>;

    fn fg(self, color: Color) -> Self::Output {
        Arc::new((*self).clone().fg(color))
    }

    fn bg(self, color: Color) -> Self::Output {
        Arc::new((*self).clone().bg(color))
    }

    fn style(self, style: Style) -> Self::Output {
        Arc::new((*self).clone().style(style))
    }

    fn modifier(self, modifier: Modifier) -> Self::Output {
        Arc::new((*self).clone().modifier(modifier))
    }

    fn current_style(&self) -> Style {
        (**self).current_style()
    }
}
