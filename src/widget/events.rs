use bitflags::bitflags;
use std::marker::PhantomData;

use crate::geometry::{Point, Size};
use crossterm::event::{MouseButton, MouseEventKind};
use ratatui::style::Style;

use super::{
    core::{IdPath, PaintCx},
    ChangeFlags, Event, EventCx, LayoutCx, Message, Pod, Widget,
};

#[derive(Debug)]
pub enum LifeCycle {
    HotChanged(bool),
    ViewContextChanged(ViewContext),
    TreeUpdate,
}

#[derive(Debug)]
pub struct ViewContext {
    pub window_origin: Point,
    // pub clip: Rect,
    pub mouse_position: Option<Point>,
}

impl ViewContext {
    pub fn translate_to(&self, new_origin: Point) -> ViewContext {
        // TODO I think the clip calculation is buggy in xilem (width/height?)
        // let clip = Rect {
        //     x: self.clip.x - new_origin.x,
        //     y: self.clip.y - new_origin.y,
        //     width: self.clip.width,
        //     height: self.clip.height,
        // };
        let translate = new_origin.to_vec2();
        ViewContext {
            window_origin: self.window_origin + translate,
            // clip,
            mouse_position: self.mouse_position.map(|p| p - translate),
        }
    }
}

#[derive(Debug)]
/// A message representing a mouse event.
pub struct MouseEvent {
    pub over_element: bool,
    pub is_active: bool,
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: crossterm::event::KeyModifiers,
}

impl MouseEvent {
    fn new(event: crossterm::event::MouseEvent, over_element: bool, is_active: bool) -> Self {
        MouseEvent {
            over_element,
            is_active,
            kind: event.kind,
            column: event.column,
            row: event.row,
            modifiers: event.modifiers,
        }
    }
}

bitflags! {
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[must_use]
    pub struct CatchMouseButton: u8 {
        const LEFT = 1;
        const RIGHT = 2;
        const MIDDLE = 4;
    }
}

pub struct OnMouse<E> {
    pub(crate) element: Pod,
    id_path: IdPath,
    catch_event: CatchMouseButton,
    phantom: PhantomData<E>,
}

impl<E: Widget> OnMouse<E> {
    pub fn new(element: E, id_path: &IdPath, catch_event: CatchMouseButton) -> Self {
        OnMouse {
            element: Pod::new(element),
            id_path: id_path.clone(),
            phantom: PhantomData,
            catch_event,
        }
    }
}

impl<E: Widget> Widget for OnMouse<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.element.paint(cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &super::BoxConstraints) -> Size {
        self.element.layout(cx, bc)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.element.event(cx, event);

        match event {
            Event::Mouse(
                event @ crossterm::event::MouseEvent {
                    kind: MouseEventKind::Down(button),
                    ..
                },
            ) => {
                let catch_event = matches!(button, MouseButton::Left if self.catch_event.intersects(CatchMouseButton::LEFT))
                    || matches!(button, MouseButton::Right if self.catch_event.intersects(CatchMouseButton::RIGHT))
                    || matches!(button, MouseButton::Middle if self.catch_event.intersects(CatchMouseButton::MIDDLE));

                if catch_event && cx.is_hot() {
                    cx.set_active(true);
                }

                if cx.is_hot() {
                    cx.add_message(Message::new(
                        self.id_path.clone(),
                        MouseEvent::new(*event, true, cx.is_active()),
                    ));
                }
            }
            Event::Mouse(event @ crossterm::event::MouseEvent { kind, .. }) => {
                let is_active = cx.is_active();
                if matches!(kind, MouseEventKind::Up(_)) {
                    cx.set_active(false);
                }
                if cx.is_hot() {
                    cx.add_message(Message::new(
                        self.id_path.clone(),
                        MouseEvent::new(*event, true, cx.is_active()),
                    ));
                // if it's not hot, and not active the event will likely not be propagated until here, but double checking doesn't hurt (much...)
                } else if is_active {
                    cx.add_message(Message::new(
                        self.id_path.clone(),
                        MouseEvent::new(*event, false, cx.is_active()),
                    ));
                }
            }
            // TODO handle other events like e.g. FocusLost
            Event::FocusLost => {
                // We can't be really sure, whether the mouse button was released in the outside the focus, so be conservative here...
                cx.set_active(false);
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, cx: &mut super::core::LifeCycleCx, event: &LifeCycle) {
        self.element.lifecycle(cx, event);
    }
}

pub struct OnClick<E> {
    pub(crate) element: Pod,
    id_path: IdPath,
    phantom: PhantomData<E>,
}

impl<E: Widget> OnClick<E> {
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
        self.element.paint(cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &super::BoxConstraints) -> Size {
        self.element.layout(cx, bc)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.element.event(cx, event);

        if let Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            ..
        }) = event
        {
            cx.set_active(cx.is_hot());
        }

        // TODO handle other events like e.g. FocusLost
        if let Event::Mouse(crossterm::event::MouseEvent {
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

    fn lifecycle(&mut self, cx: &mut super::core::LifeCycleCx, event: &LifeCycle) {
        self.element.lifecycle(cx, event);
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

    fn layout(&mut self, cx: &mut LayoutCx, bc: &super::BoxConstraints) -> Size {
        self.element.layout(cx, bc)
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

    fn lifecycle(&mut self, cx: &mut super::core::LifeCycleCx, event: &LifeCycle) {
        self.element.lifecycle(cx, event);
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

    fn layout(&mut self, cx: &mut LayoutCx, bc: &super::BoxConstraints) -> Size {
        self.element.layout(cx, bc)
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

    fn lifecycle(&mut self, cx: &mut super::core::LifeCycleCx, event: &LifeCycle) {
        self.element.lifecycle(cx, event);
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

impl<E: Widget> Widget for StyleOnHover<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        if cx.is_hot() {
            cx.override_style = self.style.patch(cx.override_style);
        };
        self.element.paint(cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &super::BoxConstraints) -> Size {
        self.element.layout(cx, bc)
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

    fn lifecycle(&mut self, cx: &mut super::core::LifeCycleCx, event: &LifeCycle) {
        self.element.lifecycle(cx, event);
    }
}

pub struct StyleOnPressed<E> {
    pub(crate) element: Pod,
    pub(crate) style: Style,
    phantom: PhantomData<E>,
}

impl<E: Widget> StyleOnPressed<E> {
    pub fn new(element: E, style: Style) -> Self {
        StyleOnPressed {
            element: Pod::new(element),
            style,
            phantom: PhantomData,
        }
    }
}

impl<E: Widget> Widget for StyleOnPressed<E> {
    fn paint(&mut self, cx: &mut PaintCx) {
        if cx.is_active() {
            cx.override_style = self.style.patch(cx.override_style);
        };
        self.element.paint(cx);
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &super::BoxConstraints) -> Size {
        self.element.layout(cx, bc)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.element.event(cx, event);

        match event {
            Event::Mouse(crossterm::event::MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                ..
            }) => {
                cx.request_paint();
                cx.set_active(cx.is_hot());
            }
            Event::Mouse(crossterm::event::MouseEvent {
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

    fn lifecycle(&mut self, cx: &mut super::core::LifeCycleCx, event: &LifeCycle) {
        self.element.lifecycle(cx, event);
    }
}
