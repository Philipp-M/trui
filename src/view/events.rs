use super::{Cx, PendingTask, Styleable, View, ViewMarker};
use crate::widget::{self, ChangeFlags, StyleableWidget, Widget};
use futures_task::Waker;
use futures_util::{Future, Stream, StreamExt};
use ratatui::style::{Color, Style};
use std::{marker::PhantomData, sync::Arc};
use tokio::{runtime::Runtime, sync::mpsc::Receiver, task::JoinHandle};
use xilem_core::{AsyncWake, Id, MessageResult};

pub trait EventHandler<T, A = (), E = ()>: Send {
    type State: Send;
    fn build(&self, cx: &mut Cx) -> (Id, Self::State);

    // TODO should id be mutable like in View::rebuild?
    #[allow(unused)]
    fn rebuild(&self, cx: &mut Cx, id: &Id, state: &mut Self::State) -> ChangeFlags {
        ChangeFlags::empty()
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A>;

    // TODO this requires additional AppMessages and a background (event-handler) Waker
    // fn keep_alive(&self) -> bool;
}

// TODO A blanket implementation for F where <E, F: Fn(&mut T, E) -> A>
//      needs the negative bounds feature (E: !() because of the implementation below)
//      I think it makes sense to be more explicit and implement it for concrete events,
//      instead of a blanket implementation for all kinds of events,
//      to avoid having something like |&mut T, ()| {} where otherwise |&mut T| {} is sufficient (and more convenient to use)
//      Manual implementations with custom event callbacks could probably be simplified (less boilerplate) with macros

impl<T, A, F: Fn(&mut T) -> A + Send> EventHandler<T, A> for F {
    type State = ();

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State) {
        (Id::next(), ())
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        _state: &mut Self::State,
        event: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        debug_assert!(id_path.is_empty() && event.downcast::<()>().is_ok());
        MessageResult::Action(self(app_state))
    }
}

/// This currently broadcasts the messages to each of the sub event handlers.
/// TODO should this filter instead, or is this usable at all?
impl<T, A, E: Clone + 'static, E1: EventHandler<T, A, E>, E2: EventHandler<T, A, E>>
    EventHandler<T, A, E> for (E1, E2)
{
    type State = ((Id, E1::State), (Id, E2::State));

    fn build(&self, cx: &mut Cx) -> (Id, Self::State) {
        cx.with_new_id(|cx| (self.0.build(cx), self.1.build(cx)))
    }

    fn rebuild(&self, cx: &mut Cx, _id: &Id, state: &mut Self::State) -> ChangeFlags {
        self.0.rebuild(cx, &state.0 .0, &mut state.0 .1)
            | self.1.rebuild(cx, &state.1 .0, &mut state.1 .1)
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        match id_path {
            [id, rest_path @ ..] if *id == state.0 .0 => {
                self.0
                    .message(rest_path, &mut state.0 .1, message, app_state)
            }
            [id, rest_path @ ..] if *id == state.1 .0 => {
                self.1
                    .message(rest_path, &mut state.1 .1, message, app_state)
            }
            [] if message.downcast_ref::<E>().is_some() => {
                let message = message.downcast::<E>().unwrap();
                let res1 = self
                    .0
                    .message(&[], &mut state.0 .1, message.clone(), app_state);
                let res2 = self.1.message(&[], &mut state.1 .1, message, app_state);
                // TODO currently the second message result will be discarded...
                res1.or(|_| res2)
            }
            [..] => MessageResult::Stale(message),
        }
    }
}

pub enum StreamMessage<E> {
    Begin(E),
    Update(E),
    Finished,
}

pub struct StreamEventHandlerState<E> {
    waker: Waker,
    runtime: Arc<Runtime>,
    chan: Option<Receiver<Option<E>>>,
    started: bool,
    is_streaming: bool,
    join_handle: Option<JoinHandle<()>>,
}

impl<E: Send + 'static> StreamEventHandlerState<E> {
    fn new(waker: Waker, runtime: Arc<Runtime>) -> Self {
        Self {
            waker,
            runtime,
            chan: None,
            join_handle: None,
            started: false,
            is_streaming: false,
        }
    }

    fn dispatch<S: Stream<Item = E> + Send + 'static>(&mut self, stream: S) {
        let waker = self.waker.clone();

        let (stream_tx, stream_rx) = tokio::sync::mpsc::channel(1000);

        self.chan = Some(stream_rx);

        let join_handle = self.runtime.spawn(async move {
            let mut stream = Box::pin(stream);

            while let Some(s) = stream.next().await {
                if (stream_tx.send(Some(s)).await).is_ok() {
                    waker.clone().wake();
                } else {
                    break;
                }
            }
        });

        self.started = true;
        self.is_streaming = true;
        self.join_handle = Some(join_handle);
    }

    fn poll(&mut self) -> Option<StreamMessage<E>> {
        match self.chan.as_mut().unwrap().try_recv() {
            Ok(Some(message)) if self.started => {
                self.started = false;
                Some(StreamMessage::Begin(message))
            }
            Ok(Some(message)) => Some(StreamMessage::Update(message)),
            Ok(None) => {
                self.is_streaming = false;
                Some(StreamMessage::Finished)
            }
            Err(_) => {
                self.is_streaming = false;
                None
            }
        }
    }
}

pub struct StreamEventHandler<T, A, E, S, SF, UF> {
    #[allow(clippy::complexity)]
    phantom: PhantomData<fn() -> (T, A, E, S)>,
    stream_fn: SF,
    update_fn: UF,
}

impl<T, A, SE, S, SF, UF, E: 'static> EventHandler<T, A, E>
    for StreamEventHandler<T, A, SE, S, SF, UF>
where
    SE: Send + 'static,
    S: Stream<Item = SE> + Send + 'static,
    SF: Fn(&mut T, E) -> S + Send,
    UF: Fn(&mut T, StreamMessage<SE>) + Send,
{
    type State = StreamEventHandlerState<SE>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State) {
        cx.with_new_id(|cx| {
            let waker = cx.waker();
            StreamEventHandlerState::new(waker, cx.rt.clone())
        })
    }

    fn rebuild(&self, cx: &mut Cx, id: &Id, state: &mut Self::State) -> ChangeFlags {
        if state.is_streaming {
            cx.add_pending_async(*id)
        }
        ChangeFlags::empty()
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        if !id_path.is_empty() {
            return MessageResult::Stale(message);
        }
        if message.downcast_ref::<AsyncWake>().is_some() {
            let mut has_updates = false;
            while let Some(p) = state.poll() {
                (self.update_fn)(app_state, p);
                has_updates = true;
            }
            if has_updates {
                MessageResult::RequestRebuild
            } else {
                MessageResult::Nop
            }
        } else if message.downcast_ref::<E>().is_some() {
            state.dispatch((self.stream_fn)(
                app_state,
                *message.downcast::<E>().unwrap(),
            ));
            MessageResult::Nop
        } else {
            MessageResult::Stale(message)
        }
    }
}

pub fn stream<T, A, SI, S, E, SF, UF>(
    stream_fn: SF,
    update_fn: UF,
) -> StreamEventHandler<T, A, SI, S, SF, UF>
where
    SI: Send + 'static,
    S: Stream<Item = SI> + Send + 'static,
    SF: Fn(&mut T, E) -> S + Send,
    UF: Fn(&mut T, StreamMessage<SI>) + Send,
{
    StreamEventHandler {
        phantom: PhantomData,
        stream_fn,
        update_fn,
    }
}

pub struct DeferEventHandler<T, A, FO, F, FF, CF> {
    #[allow(clippy::complexity)]
    phantom: PhantomData<fn() -> (T, A, FO, F)>,
    future_fn: FF,
    callback_fn: CF,
}

impl<T, A, FO, F, E, FF, CF> EventHandler<T, A, E> for DeferEventHandler<T, A, FO, F, FF, CF>
where
    E: 'static,
    FO: Send + 'static,
    F: Future<Output = FO> + Send + 'static,
    FF: Fn(&mut T, E) -> F + Send,
    CF: Fn(&mut T, FO) + Send,
{
    type State = (Option<PendingTask<FO>>, Arc<Runtime>, Waker);

    fn build(&self, cx: &mut Cx) -> (Id, Self::State) {
        cx.with_new_id(|cx| (None, cx.rt.clone(), cx.waker()))
    }

    fn rebuild(&self, cx: &mut Cx, id: &Id, state: &mut Self::State) -> ChangeFlags {
        if state.0.is_some() {
            cx.add_pending_async(*id)
        }
        ChangeFlags::empty()
    }

    // TODO deduplicate/"beautify" a little bit of that code below...
    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        if !id_path.is_empty() {
            return MessageResult::Stale(message);
        }
        if message.downcast_ref::<AsyncWake>().is_some() {
            let Some(task) = &mut state.0 else {
                return MessageResult::Stale(message);
            };
            if task.poll() {
                let Some(result) = task.result.take() else {
                    return MessageResult::Stale(message);
                };
                state.0.take();
                (self.callback_fn)(app_state, result);
                MessageResult::RequestRebuild
            } else {
                MessageResult::Nop
            }
        } else if message.downcast_ref::<E>().is_some() {
            let event = *message.downcast::<E>().unwrap();
            let future = (self.future_fn)(app_state, event);
            let join_handle = state.1.spawn(Box::pin(future));
            let task = tokio::task::unconstrained(join_handle); // TODO really unconstrained?
            let mut task = PendingTask::new(state.2.clone(), task);
            if task.poll() {
                if let Some(result) = task.result.take() {
                    state.0.take();
                    (self.callback_fn)(app_state, result);
                };
            } else {
                state.0 = Some(task);
            }
            MessageResult::RequestRebuild
        } else {
            MessageResult::Stale(message)
        }
    }
}

pub fn defer<T, A, E, FO, F, FF, CF>(
    future_fn: FF,
    callback_fn: CF,
) -> DeferEventHandler<T, A, FO, F, FF, CF>
where
    FO: Send + 'static,
    F: Future<Output = FO> + Send + 'static,
    FF: Fn(&mut T, E) -> F + Send,
    CF: Fn(&mut T, FO) + Send,
{
    DeferEventHandler {
        phantom: PhantomData,
        future_fn,
        callback_fn,
    }
}

pub trait Hoverable<T, A, V, EH: EventHandler<T, A>> {
    fn on_hover(self, event_handler: EH) -> OnHover<T, A, V, EH>;
    fn on_blur_hover(self, event_handler: EH) -> OnHoverLost<T, A, V, EH>;
}

impl<T, A, V, EH> Hoverable<T, A, V, EH> for V
where
    V: View<T, A>,
    EH: EventHandler<T, A>,
{
    fn on_hover(self, event_handler: EH) -> OnHover<T, A, V, EH> {
        OnHover {
            view: self,
            event_handler,
            phantom: PhantomData,
        }
    }

    fn on_blur_hover(self, event_handler: EH) -> OnHoverLost<T, A, V, EH> {
        OnHoverLost {
            view: self,
            event_handler,
            phantom: PhantomData,
        }
    }
}

pub trait Clickable<T, A, V, EH: EventHandler<T, A>> {
    fn on_click(self, event_handler: EH) -> OnClick<T, A, V, EH>;
}

impl<T, A, V, EH> Clickable<T, A, V, EH> for V
where
    V: View<T, A>,
    EH: EventHandler<T, A> + Send,
{
    fn on_click(self, event_handler: EH) -> OnClick<T, A, V, EH> {
        OnClick {
            view: self,
            event_handler,
            phantom: PhantomData,
        }
    }
}

macro_rules! styled_event_views {
    ($($name:ident),*) => {
        $(
        pub struct $name<T, A, V> {
            view: V,
            style: Style,
            phantom: PhantomData<fn() -> (T, A)>,
        }

        impl<T, A, V> ViewMarker for $name<T, A, V> {}

        impl<T, A, V> Styleable<T, A> for $name<T, A, V>
        where
            V: View<T, A> + Styleable<T, A>,
            V::Output: Styleable<T, A>,
            <<V as Styleable<T, A>>::Output as View<T, A>>::Element: StyleableWidget + 'static,
        {
            type Output = $name<T, A, <V as Styleable<T, A>>::Output>;

            fn fg(self, color: ratatui::style::Color) -> Self::Output {
                $name {
                    view: self.view.fg(color),
                    style: self.style,
                    phantom: PhantomData,
                }
            }

            fn bg(self, color: ratatui::style::Color) -> Self::Output {
                $name {
                    view: self.view.bg(color),
                    style: self.style,
                    phantom: PhantomData,
                }
            }

            fn modifier(self, modifier: ratatui::style::Modifier) -> Self::Output {
                $name {
                    view: self.view.modifier(modifier),
                    style: self.style,
                    phantom: PhantomData,
                }
            }

            fn style(self, style: ratatui::style::Style) -> Self::Output {
                $name {
                    view: self.view.style(style),
                    style: self.style,
                    phantom: PhantomData,
                }
            }

            fn current_style(&self) -> Style {
                self.view.current_style()
            }
        }
        )*
    }
}

// TODO is "invisible" (i.e. without id) a good idea?
// it never should receive events (or other things) directly and is just a trait on top of any *actual* view?
impl<T, A, VS, V> View<T, A> for StyleOnHover<T, A, V>
where
    VS: View<T, A>,
    V::Element: StyleableWidget,
    V: View<T, A> + Styleable<T, A, Output = VS>,
{
    type State = V::State;

    type Element = widget::StyleOnHover<V::Element>;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, state, element) = self.view.build(cx);

        (id, state, widget::StyleOnHover::new(element, self.style))
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        if element.style != self.style {
            element.style = self.style;
            changeflags |= ChangeFlags::PAINT;
        }
        changeflags |= self
            .view
            .rebuild(cx, &prev.view, id, state, &mut element.element);
        changeflags
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        self.view.message(id_path, state, message, app_state)
    }
}

impl<T, A, VS, V> View<T, A> for StyleOnPressed<T, A, V>
where
    VS: View<T, A>,
    V::Element: StyleableWidget + Widget + 'static,
    V: View<T, A> + Styleable<T, A, Output = VS>,
{
    type State = (V::State, Id);

    type Element = widget::StyleOnPressed<V::Element>;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (child_id, state, element) = self.view.build(cx);

            (
                (state, child_id),
                widget::StyleOnPressed::new(element, self.style),
            )
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        (state, child_id): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut changeflags = ChangeFlags::empty();
        if element.style != self.style {
            element.style = self.style;
            changeflags |= ChangeFlags::PAINT;
        }
        changeflags |= cx.with_id(*id, |cx| {
            self.view.rebuild(
                cx,
                &prev.view,
                child_id,
                state,
                element.element.downcast_mut().expect(
                    "The style on pressed content widget changed its type, this should never happen!",
                ),
            )
        });
        changeflags
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        (state, child_id): &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        match id_path.split_first() {
            Some((first, rest_path)) if first == child_id => {
                self.view.message(rest_path, state, message, app_state)
            }
            _ => xilem_core::MessageResult::Stale(message),
        }
    }
}

styled_event_views!(StyleOnHover, StyleOnPressed);

pub trait HoverStyleable<T, A, V: View<T, A>> {
    fn on_hover_style(self, style: Style) -> StyleOnHover<T, A, V>;

    fn on_hover_fg(self, color: Color) -> StyleOnHover<T, A, V>
    where
        Self: Sized,
    {
        self.on_hover_style(Style::default().fg(color))
    }

    fn on_hover_bg(self, color: Color) -> StyleOnHover<T, A, V>
    where
        Self: Sized,
    {
        self.on_hover_style(Style::default().bg(color))
    }
}

impl<T, A, VS, V> HoverStyleable<T, A, V> for V
where
    VS: View<T, A>,
    V: View<T, A> + Styleable<T, A, Output = VS>,
{
    fn on_hover_style(self, style: Style) -> StyleOnHover<T, A, V> {
        StyleOnHover {
            view: self,
            style,
            phantom: PhantomData,
        }
    }
}

pub trait PressedStyleable<T, A, V: View<T, A>> {
    fn on_pressed_style(self, style: Style) -> StyleOnPressed<T, A, V>;

    fn on_pressed_fg(self, color: Color) -> StyleOnPressed<T, A, V>
    where
        Self: Sized,
    {
        self.on_pressed_style(Style::default().fg(color))
    }

    fn on_pressed_bg(self, color: Color) -> StyleOnPressed<T, A, V>
    where
        Self: Sized,
    {
        self.on_pressed_style(Style::default().bg(color))
    }
}

impl<T, A, VS, V> PressedStyleable<T, A, V> for V
where
    VS: View<T, A>,
    V: View<T, A> + Styleable<T, A, Output = VS>,
{
    fn on_pressed_style(self, style: Style) -> StyleOnPressed<T, A, V> {
        StyleOnPressed {
            view: self,
            style,
            phantom: PhantomData,
        }
    }
}

// TODO own state (id_path etc.)
macro_rules! event_views {
    ($($name:ident),*) => {
        $(
        pub struct $name<T, A, V, EH> {
            view: V,
            event_handler: EH,
            phantom: PhantomData<fn() -> (T, A)>,
        }

        impl<T, A, V, EH> ViewMarker for $name<T, A, V, EH> {}

        impl<T, A, V, EH> View<T, A> for $name<T, A, V, EH>
        where
            V: View<T, A>,
            EH: EventHandler<T, A>,
        {
            type State = (V::State, Id, (Id, EH::State));

            type Element = widget::$name<V::Element>;

            fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
                let (id, (state, element)) = cx.with_new_id(|cx| {
                    let (child_id, state, element) = self.view.build(cx);

                    ((state, child_id, self.event_handler.build(cx)), widget::$name::new(element, cx.id_path()))
                });
                (id, state, element)
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                id: &mut xilem_core::Id,
        (state, child_id, (eh_id, eh_state)): &mut Self::State,
                element: &mut Self::Element,
            ) -> ChangeFlags {
                let changeflags = cx.with_id(*id, |cx| {
                    self.view.rebuild(
                        cx,
                        &prev.view,
                        child_id,
                        state,
                        &mut element.element,
                    )
                });

                changeflags | self.event_handler.rebuild(cx, eh_id, eh_state)
            }

            fn message(
                &self,
                id_path: &[xilem_core::Id],
                (state, child_id, (event_handler_id, event_handler_state)): &mut Self::State,
                message: Box<dyn std::any::Any>,
                app_state: &mut T,
            ) -> xilem_core::MessageResult<A> {
                match id_path {
                    [first, rest_path @ ..] if first == child_id => {
                        self.view.message(rest_path, state, message, app_state)
                    }
                    [first, rest_path @ ..] if first == event_handler_id => {
                        self.event_handler
                            .message(rest_path, event_handler_state, message, app_state)
                    }
                    [] => self
                        .event_handler
                        .message(&[], event_handler_state, message, app_state),
                    [..] => xilem_core::MessageResult::Stale(message),
                }
            }
        }

        impl<T, A, V, EH> Styleable<T, A> for $name<T, A, V, EH>
        where
            V: View<T, A> + Styleable<T, A>,
            EH: EventHandler<T, A>,
        {
            type Output = $name<T, A, <V as Styleable<T, A>>::Output, EH>;

            fn fg(self, color: ratatui::style::Color) -> Self::Output {
                $name {
                    view: self.view.fg(color),
                    event_handler: self.event_handler,
                    phantom: PhantomData,
                }
            }

            fn bg(self, color: ratatui::style::Color) -> Self::Output {
                $name {
                    view: self.view.bg(color),
                    event_handler: self.event_handler,
                    phantom: PhantomData,
                }
            }

            fn modifier(self, modifier: ratatui::style::Modifier) -> Self::Output {
                $name {
                    view: self.view.modifier(modifier),
                    event_handler: self.event_handler,
                    phantom: PhantomData,
                }
            }

            fn style(self, style: ratatui::style::Style) -> Self::Output {
                $name {
                    view: self.view.style(style),
                    event_handler: self.event_handler,
                    phantom: PhantomData,
                }
            }

            fn current_style(&self) -> Style {
                self.view.current_style()
            }
        }
        )*
    };
}

event_views!(OnHover, OnHoverLost);

// TODO this should probably be generated by the macro above (but for better IDE experience and easier prototyping this not yet)
pub struct OnClick<T, A, V, EH> {
    view: V,
    event_handler: EH,
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<T, A, V, EH> ViewMarker for OnClick<T, A, V, EH> {}

impl<T, A, V, EH> View<T, A> for OnClick<T, A, V, EH>
where
    V: View<T, A>,
    <V as View<T, A>>::Element: 'static,
    EH: EventHandler<T, A>,
{
    type State = (V::State, Id, (Id, EH::State));

    type Element = widget::OnClick<V::Element>;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let (child_id, state, element) = self.view.build(cx);

            (
                (state, child_id, self.event_handler.build(cx)),
                widget::OnClick::new(element, cx.id_path()),
            )
        });
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        (state, child_id, (eh_id, eh_state)): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let changeflags = cx.with_id(*id, |cx| {
            self.view.rebuild(
                cx,
                &prev.view,
                child_id,
                state,
                element.element.downcast_mut().expect(
                    "The style on pressed content widget changed its type,\
                     this should never happen!",
                ),
            )
        });

        changeflags | self.event_handler.rebuild(cx, eh_id, eh_state)
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        (state, child_id, (event_handler_id, event_handler_state)): &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        match id_path {
            [first, rest_path @ ..] if first == child_id => {
                self.view.message(rest_path, state, message, app_state)
            }
            [first, rest_path @ ..] if first == event_handler_id => {
                self.event_handler
                    .message(rest_path, event_handler_state, message, app_state)
            }
            [] => self
                .event_handler
                .message(&[], event_handler_state, message, app_state),
            [..] => xilem_core::MessageResult::Stale(message),
        }
    }
}

impl<T, A, V, EH> Styleable<T, A> for OnClick<T, A, V, EH>
where
    V: View<T, A> + Styleable<T, A>,
    // <V as Styleable<T, A>>::Output: 'static,
    <<V as Styleable<T, A>>::Output as View<T, A>>::Element: 'static,
    EH: EventHandler<T, A>,
{
    type Output = OnClick<T, A, <V as Styleable<T, A>>::Output, EH>;

    fn fg(self, color: ratatui::style::Color) -> Self::Output {
        OnClick {
            view: self.view.fg(color),
            event_handler: self.event_handler,
            phantom: PhantomData,
        }
    }

    fn bg(self, color: ratatui::style::Color) -> Self::Output {
        OnClick {
            view: self.view.bg(color),
            event_handler: self.event_handler,
            phantom: PhantomData,
        }
    }

    fn modifier(self, modifier: ratatui::style::Modifier) -> Self::Output {
        OnClick {
            view: self.view.modifier(modifier),
            event_handler: self.event_handler,
            phantom: PhantomData,
        }
    }

    fn style(self, style: ratatui::style::Style) -> Self::Output {
        OnClick {
            view: self.view.style(style),
            event_handler: self.event_handler,
            phantom: PhantomData,
        }
    }

    fn current_style(&self) -> Style {
        self.view.current_style()
    }
}
