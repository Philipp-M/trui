use bitflags::bitflags;
pub use crossterm::event::MouseEvent;
use crossterm::event::{KeyEvent, MouseEventKind};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::{any::Any, io::Stdout, ops::DerefMut};
use taffy::{tree::NodeId, Taffy};
use xilem_core::{Id, Message};

#[derive(Debug, Clone)]
pub enum Event {
    // TODO create a custom type...
    Mouse(MouseEvent),
    Key(KeyEvent),
    // FocusLost,
    // Resize { width: u16, height: u16 }, // TODO should trigger relayout (currently layout runs at every event...)
}

/// Static state that is shared between most contexts.
pub struct CxState<'a> {
    messages: &'a mut Vec<Message>,
}

impl<'a> CxState<'a> {
    pub fn new(messages: &'a mut Vec<Message>) -> Self {
        Self { messages }
    }
}

pub struct EventCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) is_handled: bool,
}

pub struct LayoutCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) taffy: &'a mut Taffy,
}

pub struct PaintCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a WidgetState,
    pub(crate) terminal: &'a mut Terminal<CrosstermBackend<Stdout>>,
    pub(crate) taffy: &'a mut Taffy,
    // TODO this kinda feels hacky, find a better solution for this issue:
    // this is currently necessary because the most outer styleable widget should be able to override the style for a styleable widget
    pub(crate) override_style: ratatui::style::Style,
}

/// A macro for implementing methods on multiple contexts.
///
/// There are a lot of methods defined on multiple contexts; this lets us only
/// have to write them out once.
macro_rules! impl_context_method {
    ($ty:ty,  { $($method:item)+ } ) => {
        #[allow(dead_code)] // TODO clean up
        impl $ty { $($method)+ }
    };
    ( $ty:ty, $($more:ty),+, { $($method:item)+ } ) => {
        impl_context_method!($ty, { $($method)+ });
        impl_context_method!($($more),+, { $($method)+ });
    };
}

// Methods on all contexts.
//
// These Methods return information about the widget
impl_context_method!(EventCx<'_, '_>, LayoutCx<'_, '_>, PaintCx<'_, '_>, {
    /// Returns whether this widget is hot.
    ///
    /// See [`is_hot`] for more details.
    ///
    /// [`is_hot`]: super::Pod::is_hot
    pub fn is_hot(&self) -> bool {
        self.widget_state.flags.contains(PodFlags::IS_HOT)
    }

    pub fn rect(&self) -> Rect {
        self.widget_state.rect
    }

    /// Returns whether this widget is active.
    ///
    /// See [`is_active`] for more details.
    ///
    /// [`is_active`]: super::Pod::is_active
    pub fn is_active(&self) -> bool {
        self.widget_state.flags.contains(PodFlags::IS_ACTIVE)
    }

    /// Returns `true` if any descendant is [`active`].
    ///
    /// [`active`]: Pod::is_active
    pub fn has_active(&self) -> bool {
        self.widget_state.flags.contains(PodFlags::HAS_ACTIVE)
    }
});

// TODO add the other contexts
// Methods on EventCx, UpdateCx, and LifeCycleCx
impl_context_method!(EventCx<'_, '_>, {
    /// Sends a message to the view tree.
    ///
    /// Sending messages is the main way of interacting with views.
    /// Generally a Widget will send messages to its View after an interaction with the user. The
    /// view will schedule a rebuild if necessary and update the widget accordingly.
    /// Since widget can send messages to all views control widgets store the IdPath of their view
    /// to target them.
    //TODO: Decide whether it should be possible to send messages from Layout?
    pub fn add_message(&mut self, message: Message) {
        self.cx_state.messages.push(message);
    }
});

impl<'a, 'b> EventCx<'a, 'b> {
    /// Set the [`active`] state of the widget.
    ///
    /// [`active`]: Pod::is_active.
    pub fn set_active(&mut self, is_active: bool) {
        self.widget_state.flags.set(PodFlags::IS_ACTIVE, is_active);
    }

    /// Set the event as "handled", which stops its propagation to other
    /// widgets.
    pub fn set_handled(&mut self, is_handled: bool) {
        self.is_handled = is_handled;
    }

    /// Determine whether the event has been handled by some other widget.
    pub fn is_handled(&self) -> bool {
        self.is_handled
    }
}

pub fn rect_contains(rect: &Rect, pos: Point) -> bool {
    pos.x >= rect.x
        && pos.y >= rect.y
        && pos.x < (rect.x + rect.width)
        && pos.y < (rect.y + rect.height)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

bitflags! {
    #[derive(Default)]
    #[must_use]
    pub struct ChangeFlags: u8 {
        const UPDATE = 1;
        const LAYOUT = 2;
        const PAINT = 8;
        const TREE = 0x10;
    }
}

bitflags! {
    #[derive(Default)]
    pub(crate) struct PodFlags: u32 {
        // These values are set to the values of their pendants in ChangeFlags to allow transmuting
        // between the two types.
        const REQUEST_UPDATE = ChangeFlags::UPDATE.bits as _;
        const REQUEST_LAYOUT = ChangeFlags::LAYOUT.bits as _;
        const REQUEST_PAINT = ChangeFlags::PAINT.bits as _;
        const TREE_CHANGED = ChangeFlags::TREE.bits as _;

        // Everything else uses bitmasks greater than the max value of ChangeFlags: mask >= 0x100
        const VIEW_CONTEXT_CHANGED = 0x100;

        const IS_HOT = 0x200;
        const IS_ACTIVE = 0x400;
        const HAS_ACTIVE = 0x800;

        const NEEDS_SET_ORIGIN = 0x1000;

        const UPWARD_FLAGS = Self::REQUEST_UPDATE.bits
            | Self::REQUEST_LAYOUT.bits
            | Self::REQUEST_PAINT.bits
            | Self::HAS_ACTIVE.bits
            | Self::TREE_CHANGED.bits
            | Self::VIEW_CONTEXT_CHANGED.bits;
        const INIT_FLAGS = Self::REQUEST_UPDATE.bits
            | Self::REQUEST_LAYOUT.bits
            | Self::REQUEST_PAINT.bits
            | Self::TREE_CHANGED.bits;
    }
}

impl PodFlags {
    /// Flags to be propagated upwards.
    pub(crate) fn upwards(self) -> Self {
        self & PodFlags::UPWARD_FLAGS
    }
}

impl ChangeFlags {
    // Change flags representing change of tree structure.
    pub fn tree_structure() -> Self {
        ChangeFlags::TREE
    }

    pub(crate) fn upwards(self) -> Self {
        // Note: this assumes PodFlags are a superset of ChangeFlags. This might
        // not always be the case, for example on "structure changed."
        let pod_flags = PodFlags::from_bits_truncate(self.bits as _);
        ChangeFlags::from_bits_truncate(pod_flags.upwards().bits as _)
    }
}
pub type IdPath = Vec<Id>;

#[derive(Debug)]
pub(crate) struct WidgetState {
    // TODO could be useful in the future, but not needed currently...
    // pub(crate) id: Id,
    pub(crate) flags: PodFlags,
    pub(crate) rect: Rect,
    // TODO useful?
    // /// The origin of the parent in the window coordinate space.
    // pub(crate) parent_window_origin: Point,
}

impl WidgetState {
    pub(crate) fn new() -> Self {
        // let id = Id::next();
        WidgetState {
            // id,
            flags: PodFlags::INIT_FLAGS,
            rect: Default::default(),
        }
    }

    fn request(&mut self, flags: PodFlags) {
        self.flags |= flags
    }

    fn merge_up(&mut self, child_state: &mut WidgetState) {
        self.flags |= child_state.flags.upwards();
    }
}

pub struct Pod {
    pub(crate) state: WidgetState,
    pub(crate) widget: Box<dyn AnyWidget>,
    pub(crate) layout_node: NodeId,
}

impl Pod {
    /// Create a new pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `Pod`
    /// so it can participate in layout and event flow.
    pub fn new(widget: impl Widget + 'static) -> Self {
        Self::new_from_box(Box::new(widget))
    }

    /// Create a new pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `Pod`
    /// so it can participate in layout and event flow.
    pub fn new_from_box(widget: Box<dyn AnyWidget>) -> Self {
        Pod {
            state: WidgetState::new(),
            widget,
            layout_node: NodeId::new(0),
        }
    }

    /// Returns the wrapped widget.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (*self.widget).as_any_mut().downcast_mut()
    }

    /// Sets the requested flags on this pod and returns the ChangeFlags the owner of this Pod should set.
    pub fn mark(&mut self, flags: ChangeFlags) -> ChangeFlags {
        self.state
            .request(PodFlags::from_bits_truncate(flags.bits as _));
        flags.upwards()
    }

    pub fn layout(&mut self, cx: &mut LayoutCx) -> NodeId {
        let inner_cx = &mut LayoutCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
            taffy: cx.taffy,
        };
        self.layout_node = self.widget.layout(inner_cx, self.layout_node);
        self.layout_node
    }

    pub fn paint(&mut self, cx: &mut PaintCx, parent_rect: Rect) {
        let r = cx.taffy.layout(self.layout_node).unwrap();
        self.state.rect = Rect {
            x: r.location.x as u16,
            y: r.location.y as u16,
            width: r.size.width as u16,
            height: r.size.height as u16,
        };
        self.state.rect.x += parent_rect.x;
        self.state.rect.y += parent_rect.y;
        let inner_cx = &mut PaintCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
            taffy: cx.taffy,
            terminal: cx.terminal,
            override_style: cx.override_style,
        };
        self.widget.paint(inner_cx)
    }

    // Return true if hot state has changed
    fn set_hot_state(widget_state: &mut WidgetState, mouse_pos: Point) -> bool {
        let had_hot = widget_state.flags.contains(PodFlags::IS_HOT);
        let is_hot = rect_contains(&widget_state.rect, mouse_pos);
        widget_state.flags.set(PodFlags::IS_HOT, is_hot);
        if had_hot != is_hot {
            // TODO
            //     let hot_changed_event = LifeCycle::HotChanged(is_hot);
            //     let mut child_cx = LifeCycleCx {
            //         cx_state,
            //         widget_state,
            //     };
            //     widget.lifecycle(&mut child_cx, &hot_changed_event);
            return true;
        }
        false
    }

    /// Propagate a platform event. As in Druid, a great deal of the event
    /// dispatching logic is in this function.
    ///
    /// This method calls [event](crate::widget::Widget::event) on the wrapped Widget if this event
    /// is relevant to this widget.
    pub fn event(&mut self, cx: &mut EventCx, event: &Event) {
        if cx.is_handled {
            return;
        }
        // let mut modified_event = None;
        let had_active = self.state.flags.contains(PodFlags::HAS_ACTIVE);
        let recurse = match event {
            Event::Mouse(mouse_event) => {
                let hot_changed = Pod::set_hot_state(
                    &mut self.state,
                    Point {
                        x: mouse_event.column,
                        y: mouse_event.row,
                    },
                );
                had_active
                    || self.state.flags.contains(PodFlags::IS_HOT)
                    || (hot_changed
                        && matches!(
                            mouse_event.kind,
                            MouseEventKind::Moved | MouseEventKind::Drag(_)
                        ))
                // TODO this is not optimal yet...
            }
            _ => return,
        };
        if recurse {
            let mut inner_cx = EventCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
                is_handled: false,
            };
            self.widget.event(&mut inner_cx, event);
            cx.is_handled |= inner_cx.is_handled;

            // This clears the has_active state. Pod needs to clear this state since merge up can
            // only set flags.
            self.state.flags.set(
                PodFlags::HAS_ACTIVE,
                self.state.flags.contains(PodFlags::IS_ACTIVE),
            );
            cx.widget_state.merge_up(&mut self.state);
        }
    }
}

pub trait Widget {
    fn paint(&mut self, cx: &mut PaintCx);

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId;

    fn event(&mut self, cx: &mut EventCx, event: &Event);
}

pub trait AnyWidget: Widget {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str;
}

impl<W: Widget + 'static> AnyWidget for W {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl Widget for Box<dyn AnyWidget> {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.deref_mut().paint(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, prev: NodeId) -> NodeId {
        self.deref_mut().layout(cx, prev)
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.deref_mut().event(cx, event)
    }
}

pub trait StyleableWidget {
    /// returns true, if it needs a repaint after setting the style (most of the time: style of the widget has changed)
    fn set_style(&mut self, style: ratatui::style::Style) -> bool;
}
