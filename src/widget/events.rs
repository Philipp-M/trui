use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use taffy::tree::NodeId;
use xilem_core::Message;

use super::{
    core::{IdPath, PaintCx},
    Event, EventCx, LayoutCx, StyleCx, Widget,
};

pub struct OnClick<E> {
    pub element: E,
    id_path: IdPath,
    is_pressed: bool,
}

impl<E> OnClick<E> {
    pub fn new(element: E, id_path: &IdPath) -> Self {
        OnClick {
            element,
            // TODO put this into core widget logic, like in xilem
            is_pressed: false,
            id_path: id_path.clone(),
        }
    }
}

impl<E: Widget> Widget for OnClick<E> {
    fn paint(&mut self, cx: &mut PaintCx, rect: Rect) {
        self.element.paint(cx, rect);
    }

    fn style(&mut self, cx: &mut StyleCx, prev: NodeId) -> NodeId {
        self.element.style(cx, prev)
    }

    fn layout(&mut self, cx: &mut LayoutCx, rect: Rect) {
        self.element.layout(cx, rect)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        if let Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            ..
        }) = event
        {
            self.is_pressed = cx.is_hot();
        }

        if let Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            ..
        }) = event
        {
            if self.is_pressed && cx.is_hot() {
                cx.add_message(Message::new(self.id_path.clone(), ()));
            }
            self.is_pressed = false;
        }
        // TODO catch/consume event?
        self.element.event(cx, event)
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
    fn paint(&mut self, cx: &mut PaintCx, rect: Rect) {
        self.element.paint(cx, rect);
    }

    fn style(&mut self, cx: &mut StyleCx, prev: NodeId) -> NodeId {
        self.element.style(cx, prev)
    }

    fn layout(&mut self, cx: &mut LayoutCx, rect: Rect) {
        self.element.layout(cx, rect)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        if matches!(event, Event::Mouse(_)) {
            if cx.is_hot() && !self.is_hovering {
                self.is_hovering = true;
                cx.add_message(Message::new(self.id_path.clone(), ()));
            } else if !cx.is_hot() && self.is_hovering {
                self.is_hovering = false;
            }
        }
        // TODO catch/consume event?
        self.element.event(cx, event)
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
    fn paint(&mut self, cx: &mut PaintCx, rect: Rect) {
        self.element.paint(cx, rect);
    }

    fn style(&mut self, cx: &mut StyleCx, prev: NodeId) -> NodeId {
        self.element.style(cx, prev)
    }

    fn layout(&mut self, cx: &mut LayoutCx, rect: Rect) {
        self.element.layout(cx, rect)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        if matches!(event, Event::Mouse(_)) {
            if cx.is_hot() && !self.is_hovering {
                self.is_hovering = true;
            } else if !cx.is_hot() && self.is_hovering {
                self.is_hovering = false;
                cx.add_message(Message::new(self.id_path.clone(), ()));
            }
        }
        self.element.event(cx, event)
    }
}
