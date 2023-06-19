use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::style::Style;
use taffy::tree::NodeId;
use xilem_core::Message;

use super::{
    core::{IdPath, PaintCx, StyleableWidget},
    Event, EventCx, LayoutCx, Widget,
};

pub struct OnClick<E> {
    pub element: E,
    id_path: IdPath,
}

impl<E> OnClick<E> {
    pub fn new(element: E, id_path: &IdPath) -> Self {
        OnClick {
            element,
            id_path: id_path.clone(),
        }
    }
}

impl<E: Widget> Widget for OnClick<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.element.paint(cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
        self.element.layout(cx, prev)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.element.event(cx, event);

        if let Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            ..
        }) = event
        {
            cx.set_active(cx.is_hot());
        }

        if let Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            ..
        }) = event
        {
            if cx.is_hot() && cx.is_active() {
                cx.add_message(Message::new(self.id_path.clone(), ()));
            }
            cx.set_active(false);
        }
    }
}

pub struct OnHover<E> {
    pub element: E,
    id_path: IdPath,
    is_hovering: bool,
}

impl<E> OnHover<E> {
    pub fn new(element: E, id_path: &IdPath) -> Self {
        OnHover {
            element,
            is_hovering: false,
            id_path: id_path.clone(),
        }
    }
}

impl<E: Widget> Widget for OnHover<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.element.paint(cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
        self.element.layout(cx, prev)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.element.event(cx, event);

        if matches!(event, Event::Mouse(_)) {
            if cx.is_hot() && !self.is_hovering {
                self.is_hovering = true;
                cx.add_message(Message::new(self.id_path.clone(), ()));
            } else if !cx.is_hot() && self.is_hovering {
                self.is_hovering = false;
            }
        }
    }
}

pub struct OnHoverLost<E> {
    pub element: E,
    id_path: IdPath,
    is_hovering: bool,
}

impl<E> OnHoverLost<E> {
    pub fn new(element: E, id_path: &IdPath) -> Self {
        OnHoverLost {
            element,
            is_hovering: false,
            id_path: id_path.clone(),
        }
    }
}

impl<E: Widget> Widget for OnHoverLost<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.element.paint(cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
        self.element.layout(cx, prev)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.element.event(cx, event);

        if matches!(event, Event::Mouse(_)) {
            if cx.is_hot() && !self.is_hovering {
                self.is_hovering = true;
            } else if !cx.is_hot() && self.is_hovering {
                self.is_hovering = false;
                cx.add_message(Message::new(self.id_path.clone(), ()));
            }
        }
    }
}

pub struct StyleOnHover<E> {
    pub element: E,
    is_hovering: bool,
    style: Style,
}

impl<E> StyleOnHover<E> {
    pub fn new(element: E, style: Style) -> Self {
        StyleOnHover {
            element,
            is_hovering: false,
            style,
        }
    }
}

impl<E: Widget + StyleableWidget> StyleableWidget for StyleOnHover<E> {
    fn set_style(&mut self, style: ratatui::style::Style) -> bool {
        self.element.set_style(style)
    }
}

impl<E: Widget + StyleableWidget> Widget for StyleOnHover<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        if cx.override_style.is_none() {
            if self.is_hovering {
                cx.override_style = Some(self.style);
            };
            self.element.paint(cx);
            cx.override_style = None;
        } else {
            self.element.paint(cx);
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
        self.element.layout(cx, prev)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.element.event(cx, event);

        if matches!(event, Event::Mouse(_)) {
            self.is_hovering = cx.is_hot();
        }
    }
}
