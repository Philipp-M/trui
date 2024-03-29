use bitflags::bitflags;
use std::marker::PhantomData;

use crate::geometry::{Point, Size};
use crossterm::event::{MouseButton, MouseEventKind};
use ratatui::style::Style;

use super::{
    core::{IdPath, PaintCx},
    EventCx, LayoutCx, Message, Pod, Widget,
};

#[derive(Debug, Clone)]
pub enum Event {
    /// Only sent once at the start of the application
    Start,
    Quit,
    /// Sent e.g. when a future requests waking up the application
    Wake,
    FocusLost,
    FocusGained,
    Resize {
        width: u16,
        height: u16,
    },
    Mouse(RawMouseEvent),
    Key(crossterm::event::KeyEvent),
}

#[derive(Debug)]
pub enum LifeCycle {
    HotChanged(bool),
    ViewContextChanged(ViewContext),
    TreeUpdate,
    Animate,
}

#[derive(Debug)]
pub struct ViewContext {
    pub window_origin: Point,
    // pub clip: Rect,
    pub mouse_position: Option<Point>,
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
pub struct RawMouseEvent {
    pub kind: MouseEventKind,
    pub column: i16,
    pub row: i16,
    pub modifiers: crossterm::event::KeyModifiers,
}

impl From<crossterm::event::MouseEvent> for RawMouseEvent {
    fn from(event: crossterm::event::MouseEvent) -> Self {
        RawMouseEvent {
            kind: event.kind,
            column: event.column as i16,
            row: event.row as i16,
            modifiers: event.modifiers,
        }
    }
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

// TODO separate the widgets etc. into its own module?

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A message representing a mouse event.
pub struct MouseEvent {
    pub over_element: bool,
    pub is_active: bool,
    pub kind: MouseEventKind,
    pub column: i16,
    pub row: i16,
    pub modifiers: crossterm::event::KeyModifiers,
}

impl MouseEvent {
    fn new(event: RawMouseEvent, over_element: bool, is_active: bool) -> Self {
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
                event @ RawMouseEvent {
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
            Event::Mouse(event @ RawMouseEvent { kind, .. }) => {
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

        if let Event::Mouse(RawMouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            ..
        }) = event
        {
            cx.set_active(cx.is_hot());
        }

        // TODO handle other events like e.g. FocusLost
        if let Event::Mouse(RawMouseEvent {
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

pub struct OnHover {
    pub(crate) element: Pod,
    id_path: IdPath,
    is_hovering: bool,
}

impl OnHover {
    pub fn new<E: Widget>(element: E, id_path: &IdPath) -> Self {
        OnHover {
            element: Pod::new(element),
            is_hovering: false,
            id_path: id_path.clone(),
        }
    }
}

impl Widget for OnHover {
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

pub struct OnHoverLost {
    pub element: Pod,
    id_path: IdPath,
    is_hovering: bool,
}

impl OnHoverLost {
    pub fn new<E: Widget>(element: E, id_path: &IdPath) -> Self {
        OnHoverLost {
            element: Pod::new(element),
            is_hovering: false,
            id_path: id_path.clone(),
        }
    }
}

impl Widget for OnHoverLost {
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

pub struct StyleOnHover {
    pub element: Pod,
    is_hovering: bool,
    pub(crate) style: Style,
}

impl StyleOnHover {
    pub fn new<E: Widget>(element: E, style: Style) -> Self {
        StyleOnHover {
            element: Pod::new(element),
            style,
            is_hovering: false,
        }
    }
}

impl Widget for StyleOnHover {
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

pub struct StyleOnPressed {
    pub(crate) element: Pod,
    pub(crate) style: Style,
}

impl StyleOnPressed {
    pub fn new<E: Widget>(element: E, style: Style) -> Self {
        StyleOnPressed {
            element: Pod::new(element),
            style,
        }
    }
}

impl Widget for StyleOnPressed {
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
            Event::Mouse(RawMouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                ..
            }) => {
                cx.request_paint();
                cx.set_active(cx.is_hot());
            }
            Event::Mouse(RawMouseEvent {
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
