use super::{BoxConstraints, LifeCycle};
use crate::geometry::{Point, Rect, Size};
use bitflags::bitflags;
pub use crossterm::event::MouseEvent;
use crossterm::event::{KeyEvent, MouseEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{any::Any, io::Stdout, ops::DerefMut};
use xilem_core::{message, Id};

message!(Send);

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
    // TODO create a custom type...
    Mouse(MouseEvent),
    Key(KeyEvent),
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

/// A mutable context provided to the [`lifecycle`] method on widgets.
///
/// Certain methods on this context are only meaningful during the handling of
/// specific lifecycle events.
///
/// [`lifecycle`]: crate::widget::Widget::lifecycle
pub struct LifeCycleCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

pub struct LayoutCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

pub struct PaintCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    // TODO mutable? (xilem doesn't do this, but I think there are use cases for this...)
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) terminal: &'a mut Terminal<CrosstermBackend<Stdout>>,
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
impl_context_method!(
    EventCx<'_, '_>,
    LayoutCx<'_, '_>,
    PaintCx<'_, '_>,
    LifeCycleCx<'_, '_>,
    {
        /// Returns whether this widget is hot.
        ///
        /// See [`is_hot`] for more details.
        ///
        /// [`is_hot`]: super::Pod::is_hot
        pub fn is_hot(&self) -> bool {
            self.widget_state.flags.contains(PodFlags::IS_HOT)
        }

        // TODO do this differently?
        /// absolute positioned Rect
        pub fn rect(&self) -> Rect {
            self.widget_state.rect()
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

        /// Requests a call to [`paint`] for this widget.
        ///
        /// [`paint`]: super::Widget::paint
        pub fn request_paint(&mut self) {
            self.widget_state.flags |= PodFlags::REQUEST_PAINT;
        }

        /// Notify Trui that this widgets view context changed.
        ///
        /// A [`LifeCycle::ViewContextChanged`] event will be scheduled.
        /// Widgets only have to call this method in case they are changing the z-order of
        /// overlapping children or change the clip region all other changes are tracked internally.
        ///
        /// [`LifeCycle::ViewContextChanged`]: super::LifeCycle::ViewContextChanged
        pub fn view_context_changed(&mut self) {
            self.widget_state.flags |= PodFlags::VIEW_CONTEXT_CHANGED;
        }
    }
);

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

bitflags! {
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[must_use]
    pub struct ChangeFlags: u8 {
        const UPDATE = 1;
        const LAYOUT = 2;
        const PAINT = 8;
        const TREE = 0x10;
    }
}

bitflags! {
        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub(crate) struct PodFlags: u32 {
        // These values are set to the values of their pendants in ChangeFlags to allow transmuting
        // between the two types.
        const REQUEST_UPDATE = ChangeFlags::UPDATE.bits() as _;
        const REQUEST_LAYOUT = ChangeFlags::LAYOUT.bits() as _;
        const REQUEST_PAINT = ChangeFlags::PAINT.bits() as _;
        const TREE_CHANGED = ChangeFlags::TREE.bits() as _;

        // Everything else uses bitmasks greater than the max value of ChangeFlags: mask >= 0x100
        const VIEW_CONTEXT_CHANGED = 0x100;

        const IS_HOT = 0x200;
        const IS_ACTIVE = 0x400;
        const HAS_ACTIVE = 0x800;

        const NEEDS_SET_ORIGIN = 0x1000;

        const UPWARD_FLAGS = Self::REQUEST_UPDATE.bits()
            | Self::REQUEST_LAYOUT.bits()
            | Self::REQUEST_PAINT.bits()
            | Self::HAS_ACTIVE.bits()
            | Self::TREE_CHANGED.bits()
            | Self::VIEW_CONTEXT_CHANGED.bits();
        const INIT_FLAGS = Self::REQUEST_UPDATE.bits()
            | Self::REQUEST_LAYOUT.bits()
            | Self::REQUEST_PAINT.bits()
            | Self::TREE_CHANGED.bits();
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
        let pod_flags = PodFlags::from_bits_truncate(self.bits() as _);
        ChangeFlags::from_bits_truncate(pod_flags.upwards().bits() as _)
    }
}
pub type IdPath = Vec<Id>;

#[derive(Debug)]
pub(crate) struct WidgetState {
    // TODO could be useful in the future, but not needed currently...
    // pub(crate) id: Id,
    pub(crate) flags: PodFlags,
    pub(crate) size: Size,
    /// The origin of the child in the parent's coordinate space.
    pub(crate) origin: Point,
    /// The origin of the parent in the window coordinate space.
    pub(crate) parent_window_origin: Point,
}

impl WidgetState {
    pub(crate) fn new() -> Self {
        // let id = Id::next();
        WidgetState {
            // id,
            flags: PodFlags::INIT_FLAGS,
            size: Default::default(),
            origin: Default::default(),
            parent_window_origin: Default::default(),
        }
    }

    pub(crate) fn request(&mut self, flags: PodFlags) {
        self.flags |= flags
    }

    pub(crate) fn merge_up(&mut self, child_state: &mut WidgetState) {
        self.flags |= child_state.flags.upwards();
    }

    pub(crate) fn window_origin(&self) -> Point {
        self.parent_window_origin + self.origin.to_vec2()
    }

    // TODO do this differently?
    /// absolute positioned Rect
    pub(crate) fn rect(&self) -> Rect {
        let origin = self.window_origin();
        Rect::new(origin.x, origin.y, self.size.width, self.size.height)
    }
}

pub struct Pod {
    pub(crate) state: WidgetState,
    pub(crate) widget: Box<dyn AnyWidget>,
}

impl Pod {
    /// Create a new pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `Pod`
    /// so it can participate in layout and event flow.
    pub fn new(widget: impl Widget) -> Self {
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
        }
    }

    /// Returns the wrapped widget.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        (*self.widget).as_any().downcast_ref()
    }

    /// Returns the wrapped widget.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (*self.widget).as_any_mut().downcast_mut()
    }

    /// Sets the requested flags on this pod and returns the ChangeFlags the owner of this Pod should set.
    pub fn mark(&mut self, flags: ChangeFlags) -> ChangeFlags {
        self.state
            .request(PodFlags::from_bits_truncate(flags.bits() as _));
        flags.upwards()
    }

    pub fn size(&self) -> Size {
        self.state.size
    }

    /// Set the origin of this widget, in the parent's coordinate space.
    ///
    /// A container widget should call the [`Widget::layout`] method on its children in
    /// its own [`Widget::layout`] implementation, and then call `set_origin` to
    /// position those children.
    ///
    /// The changed origin won't be fully in effect until [`LifeCycle::ViewContextChanged`] has
    /// finished propagating. Specifically methods that depend on the widget's origin in relation
    /// to the window will return stale results during the period after calling `set_origin` but
    /// before [`LifeCycle::ViewContextChanged`] has finished propagating.
    ///
    /// The widget container can also call `set_origin` from other context, but calling `set_origin`
    /// after the widget received [`LifeCycle::ViewContextChanged`] and before the next event results
    /// in an inconsistent state of the widget tree.
    pub fn set_origin(&mut self, cx: &mut LayoutCx, origin: Point) {
        if origin != self.state.origin {
            self.state.origin = origin;
            // request paint is called on the parent instead of this widget, since this widget's
            // fragment does not change.
            cx.view_context_changed();
            cx.request_paint();

            self.state.flags.insert(PodFlags::VIEW_CONTEXT_CHANGED);
        }
    }

    /// Propagate a layout request.
    ///
    /// This method calls [layout](crate::widget::Widget::layout) on the wrapped Widget. The container
    /// widget is responsible for calling only the children which need a call to layout. These include
    /// any Pod which has [layout_requested](Pod::layout_requested) set.
    pub fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let mut child_cx = LayoutCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        let new_size = self.widget.layout(&mut child_cx, bc);
        if new_size != self.state.size {
            self.state.flags.insert(PodFlags::VIEW_CONTEXT_CHANGED);
        }
        self.state.size = new_size;
        // Note: here we're always doing requests for downstream processing, but if we
        // make layout more incremental, we'll probably want to do this only if there
        // is an actual layout change.
        self.state.flags.insert(PodFlags::NEEDS_SET_ORIGIN);
        self.state.flags.remove(PodFlags::REQUEST_LAYOUT);
        cx.widget_state.merge_up(&mut self.state);
        self.state.size
    }

    pub fn paint(&mut self, cx: &mut PaintCx) {
        let inner_cx = &mut PaintCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
            terminal: cx.terminal,
            override_style: cx.override_style,
        };
        self.widget.paint(inner_cx);

        self.state.flags.remove(PodFlags::REQUEST_PAINT);
    }

    // Return true if hot state has changed
    fn set_hot_state(
        widget: &mut dyn AnyWidget,
        widget_state: &mut WidgetState,
        cx_state: &mut CxState,
        mouse_pos: Option<Point>,
    ) -> bool {
        let rect = Rect::from_origin_size(widget_state.origin, widget_state.size);
        let had_hot = widget_state.flags.contains(PodFlags::IS_HOT);

        let is_hot = match mouse_pos {
            Some(pos) => rect.contains(pos),
            None => false,
        };
        widget_state.flags.set(PodFlags::IS_HOT, is_hot);
        if had_hot != is_hot {
            let hot_changed_event = LifeCycle::HotChanged(is_hot);
            let mut child_cx = LifeCycleCx {
                cx_state,
                widget_state,
            };
            widget.lifecycle(&mut child_cx, &hot_changed_event);
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
        let mut modified_event = None;
        let had_active = self.state.flags.contains(PodFlags::HAS_ACTIVE);
        let recurse = match event {
            Event::Mouse(mouse_event) => {
                let hot_changed = Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    Some(Point {
                        x: mouse_event.column as f64,
                        y: mouse_event.row as f64,
                    }),
                );
                if had_active
                    || self.state.flags.contains(PodFlags::IS_HOT)
                    || (hot_changed
                        && matches!(
                            mouse_event.kind,
                            MouseEventKind::Moved | MouseEventKind::Drag(_)
                        ))
                {
                    let mut mouse_event = *mouse_event;
                    let (x, y) = (
                        self.state.origin.x.round() as u16,
                        self.state.origin.y.round() as u16,
                    );
                    mouse_event.column = mouse_event.column.saturating_sub(x);
                    mouse_event.row = mouse_event.row.saturating_sub(y);
                    modified_event = Some(Event::Mouse(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::Resize { .. } => {
                // TODO for now request repaint and relayout for resize events for *every* widget,
                // this may change in the future to be more finegrained (and efficient)
                self.state
                    .request(PodFlags::REQUEST_PAINT | PodFlags::REQUEST_LAYOUT);
                true
            }
            Event::FocusLost => {
                // right now a FocusLost event will disable any ongoing pointer events,
                // since we can't really track if the state has changed in the meantime.
                // There may be workarounds/hacks to remember the previous mouse state (mostly),
                // but I think it's safer for now to just tell every widget, that there isn't a mouse anymore in focus...
                self.state
                    .flags
                    .set(PodFlags::IS_HOT | PodFlags::IS_ACTIVE, false);
                true
            }
            _ => return,
        };
        if recurse {
            // This clears the has_active state. Pod needs to clear this state since merge up can
            // only set flags.
            // This needs to happen before the `event` call, as that will also set our `HAS_ACTIVE`
            // flag if any of our children were active
            self.state.flags.set(
                PodFlags::HAS_ACTIVE,
                self.state.flags.contains(PodFlags::IS_ACTIVE),
            );
            let mut inner_cx = EventCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
                is_handled: false,
            };
            self.widget
                .event(&mut inner_cx, modified_event.as_ref().unwrap_or(event));
            cx.is_handled |= inner_cx.is_handled;

            cx.widget_state.merge_up(&mut self.state);
        }
    }

    /// Propagate a lifecycle event.
    ///
    /// This method calls [lifecycle](crate::widget::Widget::lifecycle) on the wrapped Widget if
    /// the lifecycle event is relevant to this widget.
    pub fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        let mut modified_event = None;
        let recurse = match event {
            LifeCycle::HotChanged(_) => false,
            LifeCycle::ViewContextChanged(view) => {
                self.state.parent_window_origin = view.window_origin;

                Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    view.mouse_position,
                );
                modified_event = Some(LifeCycle::ViewContextChanged(
                    view.translate_to(self.state.origin),
                ));
                self.state.flags.remove(PodFlags::VIEW_CONTEXT_CHANGED);
                true
            }
            LifeCycle::TreeUpdate => {
                // TODO...
                if self.state.flags.contains(PodFlags::TREE_CHANGED) {
                    // self.state.sub_tree.clear();
                    // self.state.sub_tree.add(&self.state.id);
                    self.state.flags.remove(PodFlags::TREE_CHANGED);
                    true
                } else {
                    false
                }
            }
        };
        let mut child_cx = LifeCycleCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        if recurse {
            self.widget
                .lifecycle(&mut child_cx, modified_event.as_ref().unwrap_or(event));
            cx.widget_state.merge_up(&mut self.state);
        }
    }
}

pub trait Widget: 'static {
    fn paint(&mut self, cx: &mut PaintCx);

    /// Compute layout.
    ///
    /// A leaf widget should determine its size (subject to the provided
    /// constraints) and return it.
    ///
    /// A container widget will recursively call [`WidgetPod::layout`] on its
    /// child widgets, providing each of them an appropriate box constraint,
    /// compute layout, then call [`set_origin`] on each of its children.
    /// Finally, it should return the size of the container. The container
    /// can recurse in any order, which can be helpful to, for example, compute
    /// the size of non-flex widgets first, to determine the amount of space
    /// available for the flex widgets.
    ///
    /// For efficiency, a container should only invoke layout of a child widget
    /// once, though there is nothing enforcing this.
    ///
    /// The layout strategy is strongly inspired by Flutter.
    ///
    /// [`WidgetPod::layout`]: struct.WidgetPod.html#method.layout
    /// [`set_origin`]: struct.WidgetPod.html#method.set_origin
    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size;

    /// Handle a life cycle notification.
    ///
    /// This method is called to notify your widget of certain special events,
    /// (available in the [`LifeCycle`] enum) that are generally related to
    /// changes in the widget graph or in the state of your specific widget.
    ///
    /// [`LifeCycle`]: enum.LifeCycle.html
    /// [`LifeCycleCx`]: struct.LifeCycleCx.html
    /// [`Command`]: struct.Command.html
    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle);

    fn event(&mut self, cx: &mut EventCx, event: &Event);
}

pub trait AnyWidget: Widget {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str;
}

impl<W: Widget> AnyWidget for W {
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

    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        self.deref_mut().event(cx, event)
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        self.deref_mut().layout(cx, bc)
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.deref_mut().lifecycle(cx, event)
    }
}
