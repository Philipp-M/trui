use std::marker::PhantomData;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::style::Style;
use taffy::tree::NodeId;

use super::{
    core::{IdPath, PaintCx, StyleableWidget},
    Event, EventCx, LayoutCx, Message, Pod, Widget,
};

pub struct OnClick<E> {
    pub(crate) element: Pod,
    id_path: IdPath,
    phantom: PhantomData<E>,
}

impl<E: Widget + 'static> OnClick<E> {
    pub fn new(element: E, id_path: &IdPath) -> Self {
        OnClick {
            element: Pod::new(element),
            id_path: id_path.clone(),
            phantom: PhantomData,
        }
    }
}

impl<E: Widget> Widget for OnClick<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.element.paint(cx, cx.rect());
    }

    fn layout(&mut self, cx: &mut LayoutCx, _prev: NodeId) -> NodeId {
        self.element.layout(cx)
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

        // TODO handle other events like e.g. FocusLost
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

impl<E: Widget + StyleableWidget + 'static> StyleableWidget for OnClick<E> {
    fn set_style(&mut self, style: ratatui::style::Style) -> bool {
        self.element
            .downcast_mut::<E>()
            .map(|e| e.set_style(style))
            .unwrap_or(true)
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

        if matches!(event, Event::Mouse(_) | Event::FocusLost) {
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

        if matches!(event, Event::Mouse(_) | Event::FocusLost) {
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
    pub(crate) style: Style,
}

impl<E> StyleOnHover<E> {
    pub fn new(element: E, style: Style) -> Self {
        StyleOnHover {
            element,
            style,
            is_hovering: false,
        }
    }
}

impl<E: Widget + StyleableWidget> Widget for StyleOnHover<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        if cx.is_hot() {
            cx.override_style = self.style.patch(cx.override_style);
        };
        self.element.paint(cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
        self.element.layout(cx, prev)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.element.event(cx, event);
        if cx.is_hot() && !self.is_hovering {
            cx.request_paint();
            self.is_hovering = true;
        } else if !cx.is_hot() && self.is_hovering {
            cx.request_paint();
            self.is_hovering = false;
        }
    }
}

pub struct StyleOnPressed<E> {
    pub(crate) element: Pod,
    pub(crate) style: Style,
    phantom: PhantomData<E>,
}

impl<E: Widget + 'static> StyleOnPressed<E> {
    pub fn new(element: E, style: Style) -> Self {
        StyleOnPressed {
            element: Pod::new(element),
            style,
            phantom: PhantomData,
        }
    }
}

impl<E: Widget + StyleableWidget + 'static> StyleableWidget for StyleOnPressed<E> {
    fn set_style(&mut self, style: ratatui::style::Style) -> bool {
        self.element
            .downcast_mut::<E>()
            .map(|e| e.set_style(style))
            .unwrap_or(true)
    }
}

impl<E: Widget + StyleableWidget> Widget for StyleOnPressed<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        if cx.is_active() {
            cx.override_style = self.style.patch(cx.override_style);
        };
        self.element.paint(cx, cx.rect());
    }

    fn layout(&mut self, cx: &mut LayoutCx, _prev: NodeId) -> NodeId {
        self.element.layout(cx)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.element.event(cx, event);

        match event {
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                ..
            }) => {
                cx.request_paint();
                cx.set_active(cx.is_hot());
            }
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left) | MouseEventKind::Moved,
                ..
            })
            | Event::FocusLost => {
                cx.request_paint();
                cx.set_active(false);
            }
            _ => (),
        }
    }
}

macro_rules! styleable_widget_events {
    ($($name:ident),*) => {
    $(
    impl<E: Widget + StyleableWidget> StyleableWidget for $name<E> {
        fn set_style(&mut self, style: ratatui::style::Style) -> bool {
            self.element.set_style(style)
        }
    }
    )*
    };
}

styleable_widget_events!(OnHover, OnHoverLost, StyleOnHover);
