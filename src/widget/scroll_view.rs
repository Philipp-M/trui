use crossterm::event::MouseEventKind;
use kurbo::Size;

use super::{
    BoxConstraints, Canvas, Event, EventCx, LayoutCx, LifeCycle, LifeCycleCx, PaintCx, Pod,
    RawMouseEvent, Widget,
};

pub struct ScrollView {
    child: Pod,
    offset: f64,
    scroll_speed: f64,
    // TODO to avoid lifetime issues, this is a raw ratatui buffer, this should be a `Canvas` as some point though
    child_buffer: ratatui::buffer::Buffer,
}

impl ScrollView {
    pub fn new(child: impl Widget + 'static) -> Self {
        ScrollView {
            child: Pod::new(child),
            offset: 0.0,
            scroll_speed: 1.0,
            child_buffer: ratatui::buffer::Buffer::default(),
        }
    }

    pub fn child_mut(&mut self) -> &mut Pod {
        &mut self.child
    }
}

// TODO: scroll bars
impl Widget for ScrollView {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        // Pass event through to child, adjusting the coordinates of mouse events
        // by the scroll offset first.
        // TODO: scroll wheel + click-drag on scroll bars
        let child_event = if let Event::Mouse(mouse_event) = event {
            let mut mouse_event = *mouse_event;
            mouse_event.column += self.offset.round() as i16;
            Event::Mouse(mouse_event)
        } else {
            event.clone()
        };

        self.child.event(cx, &child_event);

        // Handle scroll wheel events
        // TODO scroll left/right
        if !cx.is_handled() {
            if let Event::Mouse(RawMouseEvent {
                kind: kind @ (MouseEventKind::ScrollDown | MouseEventKind::ScrollUp),
                ..
            }) = event
            {
                let max_offset = (self.child.size().height - cx.size().height).max(0.0);
                let y_delta = match kind {
                    MouseEventKind::ScrollDown => self.scroll_speed,
                    MouseEventKind::ScrollUp => -self.scroll_speed,
                    _ => unreachable!(),
                };

                let new_offset = (self.offset + y_delta).max(0.0).min(max_offset);
                if new_offset != self.offset {
                    self.offset = new_offset;
                    cx.set_handled(true);
                    cx.request_paint();
                }
            }
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.child.lifecycle(cx, event);
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        cx.request_paint();

        let cbc = BoxConstraints::new(
            Size::new(0.0, 0.0),
            Size::new(bc.max().width, f64::INFINITY),
        );
        let child_size = self.child.layout(cx, &cbc);
        let child_rect = ratatui::layout::Rect::new(
            0,
            0,
            child_size.width.round() as u16,
            child_size.height.round() as u16,
        );
        self.child_buffer.resize(child_rect);
        let size = Size::new(
            child_size.width.min(bc.max().width),
            child_size.height.min(bc.max().height),
        );

        // Ensure that scroll offset is within bounds
        let max_offset = (child_size.height - size.height).max(0.0);
        if max_offset < self.offset {
            self.offset = max_offset;
        }

        size
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        // TODO only repaint child, if it's really necessary (i.e. anything in child changed)
        self.child_buffer.reset();
        let mut child_canvas = Canvas::new(&mut self.child_buffer);
        let mut cx_child = PaintCx {
            cx_state: cx.cx_state,
            widget_state: cx.widget_state,
            canvas: &mut child_canvas,
            override_style: cx.override_style,
        };
        self.child.paint(&mut cx_child);
        cx.canvas
            .blit_with_offset(&child_canvas, (0.0, -self.offset));
    }
}
