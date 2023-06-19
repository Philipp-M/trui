use bitflags::bitflags;
use ratatui::style::{Color, Modifier, Style};

use super::View;

bitflags! {
    /// Bitflags that can be composed to set the visible borders essentially on the block widget.
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
        const ALL        = Self::TOP.bits | Self::RIGHT.bits | Self::BOTTOM.bits | Self::LEFT.bits;
        /// Show top and bottom borders
        const HORIZONTAL = Self::BOTTOM.bits | Self::TOP.bits;
        /// Show top and bottom borders
        const VERTICAL   = Self::LEFT.bits | Self::RIGHT.bits;
    }
}

pub trait Styleable<T, A = ()> {
    type Output: View<T, A>;
    fn fg(self, color: Color) -> Self::Output;
    fn bg(self, color: Color) -> Self::Output;
    fn style(self, style: Style) -> Self::Output;
    fn modifier(self, modifier: Modifier) -> Self::Output;
    fn current_style(&self) -> Style;
}
