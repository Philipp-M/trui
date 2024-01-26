use super::{
    core::{update_layout_node, PaintCx},
    ChangeFlags, Event, EventCx, Pod, StyleableWidget, Widget,
};
use crate::view::{BorderStyles, Borders};
use ratatui::{layout::Rect, style::Style, symbols};
use taffy::tree::NodeId;

pub(crate) fn render_border(cx: &mut PaintCx, r: Rect, border_styles: &BorderStyles, style: Style) {
    use Borders as B; // unfortunately not possible to wildcard import since it's not an enum...
    if r.width == 0 || r.height == 0 {
        return;
    }
    let buf = cx.terminal.current_buffer_mut();

    let mut draw = |x, y, symbol, style| {
        if buf.area.x + x < buf.area.width && buf.area.y + y < buf.area.height {
            buf.get_mut(x, y).set_symbol(symbol).set_style(style);
        }
    };

    if r.width == 1 && r.height == 1 {
        let style = border_styles.style(B::ALL).patch(style);
        draw(r.x, r.y, symbols::DOT, style);
        return;
    }

    if r.width > 1 && border_styles.has_borders(B::TOP) {
        let style = border_styles.style(B::TOP).patch(style);
        let symbol = border_styles.symbols(B::TOP).horizontal;
        for x in r.x..(r.x + r.width) {
            draw(x, r.y, symbol, style);
        }
    }

    if r.width > 1 && border_styles.has_borders(B::BOTTOM) {
        let style = border_styles.style(B::BOTTOM).patch(style);
        let symbol = border_styles.symbols(B::BOTTOM).horizontal;
        for x in r.x..(r.x + r.width) {
            draw(x, r.y + r.height - 1, symbol, style);
        }
    }

    if r.height > 1 && border_styles.has_borders(B::LEFT) {
        let style = border_styles.style(B::LEFT).patch(style);
        let symbol = border_styles.symbols(B::LEFT).vertical;
        for y in r.y..(r.y + r.height) {
            draw(r.x, y, symbol, style);
        }
    }

    if r.height > 1 && border_styles.has_borders(B::RIGHT) {
        let style = border_styles.style(B::RIGHT).patch(style);
        let symbol = border_styles.symbols(B::RIGHT).vertical;
        for y in r.y..(r.y + r.height) {
            draw(r.x + r.width - 1, y, symbol, style);
        }
    }

    // corners
    if r.width > 1 && r.height > 1 {
        if border_styles.has_borders(B::LEFT | B::TOP) {
            let style = border_styles.style(B::LEFT | B::TOP).patch(style);
            let symbol = border_styles.symbols(B::LEFT | B::TOP).top_left;
            draw(r.x, r.y, symbol, style);
        }
        if border_styles.has_borders(B::LEFT | B::BOTTOM) {
            let style = border_styles.style(B::LEFT | B::BOTTOM).patch(style);
            let symbol = border_styles.symbols(B::LEFT | B::BOTTOM).bottom_left;
            draw(r.x, r.y + r.height - 1, symbol, style);
        }
        if border_styles.has_borders(B::RIGHT | B::BOTTOM) {
            let style = border_styles.style(B::RIGHT | B::BOTTOM).patch(style);
            let symbol = border_styles.symbols(B::RIGHT | B::BOTTOM).bottom_right;
            draw(r.x + r.width - 1, r.y + r.height - 1, symbol, style);
        }
        if border_styles.has_borders(B::RIGHT | B::TOP) {
            let style = border_styles.style(B::RIGHT | B::TOP).patch(style);
            let symbol = border_styles.symbols(B::RIGHT | B::TOP).top_right;
            draw(r.x + r.width - 1, r.y, symbol, style);
        }
    }
}

fn fill_block(cx: &mut PaintCx, r: Rect, style: Style) {
    let buf = cx.terminal.current_buffer_mut();

    for x in r.x..(buf.area.width.min(r.width + r.x)) {
        for y in r.y..(buf.area.height.min(r.height + r.y)) {
            buf.get_mut(x, y).set_style(style);
        }
    }
}

pub struct Block {
    pub(crate) content: Pod,
    border_styles: BorderStyles,
    style: Style,
    layout_style: taffy::style::Style,
    fill_with_bg: bool,
    inherit_style: bool,
}

impl Block {
    pub(crate) fn new(
        content: impl Widget,
        border_styles: BorderStyles,
        style: Style,
        inherit_style: bool,
        fill_with_bg: bool,
    ) -> Self {
        let pad = |b| {
            taffy::style::LengthPercentage::Length(if border_styles.has_borders(b) {
                1.0
            } else {
                0.0
            })
        };
        Block {
            content: Pod::new(content),
            fill_with_bg,
            style,
            inherit_style,
            layout_style: taffy::style::Style {
                padding: taffy::prelude::Rect {
                    left: pad(Borders::LEFT),
                    right: pad(Borders::RIGHT),
                    top: pad(Borders::TOP),
                    bottom: pad(Borders::BOTTOM),
                },
                size: taffy::prelude::Size {
                    width: taffy::style::Dimension::Percent(1.0),
                    height: taffy::style::Dimension::Percent(1.0),
                },
                ..Default::default()
            },
            border_styles,
        }
    }

    pub(crate) fn set_border_style(&mut self, border_style: &BorderStyles) -> ChangeFlags {
        if &self.border_styles != border_style {
            self.border_styles = border_style.clone();
            // TODO more sophisticated check for needed ChangeFlags (specifically layout)
            ChangeFlags::LAYOUT | ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }

    pub(crate) fn set_fill_with_bg(&mut self, fill_with_bg: bool) -> ChangeFlags {
        if self.fill_with_bg != fill_with_bg {
            self.fill_with_bg = fill_with_bg;
            ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }

    pub(crate) fn set_inherit_style(&mut self, inherit: bool) -> ChangeFlags {
        if self.inherit_style != inherit {
            self.inherit_style = inherit;
            ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }
}

impl StyleableWidget for Block {
    fn set_style(&mut self, style: Style) -> ChangeFlags {
        if style != self.style {
            self.style = style;
            ChangeFlags::PAINT
        } else {
            ChangeFlags::empty()
        }
    }
}

impl Widget for Block {
    fn paint(&mut self, cx: &mut PaintCx) {
        let style = self.style.patch(cx.override_style);
        cx.override_style = if self.inherit_style {
            style
        } else {
            Style::default()
        };

        if self.fill_with_bg {
            let fill_style = Style {
                bg: style.bg,
                ..Default::default()
            };
            fill_block(cx, cx.rect(), fill_style);
        }

        render_border(cx, cx.rect(), &self.border_styles, style);

        self.content.paint(cx, cx.rect())
    }

    fn layout(&mut self, cx: &mut super::LayoutCx, prev: NodeId) -> NodeId {
        let content = self.content.layout(cx);
        if !prev.is_null() {
            update_layout_node(prev, cx.taffy, &[content], &self.layout_style);
            prev
        } else {
            cx.taffy
                .new_with_children(self.layout_style.clone(), &[content])
                .unwrap()
        }
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.content.event(cx, event)
    }
}
