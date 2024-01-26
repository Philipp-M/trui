use std::{future::Future, marker::PhantomData, pin::Pin};

use futures_task::{Context, Poll, Waker};
use tokio::task::{JoinHandle, Unconstrained};
use xilem_core::{AsyncWake, Id, MessageResult};

use crate::widget::{AnyWidget, ChangeFlags};

use super::{Cx, View, ViewMarker};

pub struct PendingTask<T> {
    waker: Waker,
    task: Unconstrained<JoinHandle<T>>,
    pub result: Option<T>,
}

impl<T> PendingTask<T> {
    pub fn new(waker: Waker, task: Unconstrained<JoinHandle<T>>) -> Self {
        PendingTask {
            waker,
            task,
            result: None,
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut future_cx = Context::from_waker(&self.waker);
        match Pin::new(&mut self.task).poll(&mut future_cx) {
            Poll::Ready(Ok(v)) => {
                self.result = Some(v);
                true
            }
            Poll::Ready(Err(err)) => {
                tracing::error!("error in defer view: {err}");
                false
            }
            Poll::Pending => false,
        }
    }
}

pub enum ViewState<IS, S> {
    Init(IS),
    Resolved(S),
}

pub struct DeferState<T, A, V, IV>
where
    V: View<T, A>,
    IV: View<T, A>,
{
    view_id: Id,
    view: Option<V>,
    view_state: ViewState<IV::State, V::State>,
    task: PendingTask<V>,
}

pub struct Defer<T, A, V, IV, F> {
    #[allow(clippy::type_complexity)]
    phantom: PhantomData<fn() -> (T, A, V, IV)>,
    callback: F,
    init_view: IV,
}

impl<T, A, V, IV, F> ViewMarker for Defer<T, A, V, IV, F> {}

impl<T, A, V, IV, FF, F> View<T, A> for Defer<T, A, V, IV, F>
where
    V: View<T, A> + 'static,
    IV: View<T, A>,
    V::Element: 'static,
    IV::Element: 'static,
    FF: Future<Output = V> + Send + Sync + 'static,
    F: Fn() -> FF + Send + Sync,
{
    type State = DeferState<T, A, V, IV>;

    type Element = Box<dyn AnyWidget>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let future = (self.callback)();
        let join_handle = cx.rt.spawn(Box::pin(future));
        let task = tokio::task::unconstrained(join_handle);
        let mut pending = true;
        let (id, (state, element)) = cx.with_new_id(|cx| {
            let waker = cx.waker();
            let mut task = PendingTask::new(waker, task);
            if task.poll() {
                let view = task.result.take().unwrap();
                let (view_id, view_state, element) = view.build(cx);
                let state = DeferState {
                    view: Some(view),
                    view_id,
                    view_state: ViewState::Resolved(view_state),
                    task,
                };
                pending = false;
                (state, Box::new(element) as Box<dyn AnyWidget>)
            } else {
                let (view_id, init_view_state, element) = self.init_view.build(cx);
                let state = DeferState {
                    view: None,
                    view_id,
                    view_state: ViewState::Init(init_view_state),
                    task,
                };
                (state, Box::new(element) as Box<dyn AnyWidget>)
            }
        });
        if pending {
            cx.add_pending_async(id);
        }
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        if state.view.is_none()
            && matches!(state.view_state, ViewState::Init(_))
            && state.task.poll()
        {
            state.view = state.task.result.take();
        }
        if state.view.is_some() && matches!(state.view_state, ViewState::Init(_)) {
            cx.with_id(*id, |cx| {
                let view = state.view.as_ref().unwrap();
                let (view_id, view_state, el) = view.build(cx);
                state.view_id = view_id;
                state.view_state = ViewState::Resolved(view_state);
                *element = Box::new(el);
            });
            return ChangeFlags::tree_structure();
        }
        if let ViewState::Init(view_state) = &mut state.view_state {
            cx.add_pending_async(*id);
            let element = (**element).as_any_mut().downcast_mut().unwrap();
            self.init_view
                .rebuild(cx, &prev.init_view, &mut state.view_id, view_state, element)
        } else {
            // Note: rebuild is not called on the resolved view
            ChangeFlags::empty()
        }
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        // downcast likely not necessary, but for clarity
        if id_path.is_empty() && message.downcast_ref::<AsyncWake>().is_some() {
            if state.task.poll() {
                state.view = state.task.result.take();
                MessageResult::RequestRebuild
            } else {
                MessageResult::Nop
            }
        } else if let [id, rest @ ..] = id_path {
            match &mut state.view_state {
                ViewState::Init(view_state) if *id == state.view_id => {
                    self.init_view.message(rest, view_state, message, app_state)
                }
                ViewState::Resolved(view_state) if *id == state.view_id => state
                    .view
                    .as_ref()
                    .expect("view has to be resolved at this point")
                    .message(rest, view_state, message, app_state),
                _ => MessageResult::Stale(message),
            }
        } else {
            MessageResult::Stale(message)
        }
    }
}

pub fn defer_view<T, A, V, IV, FF, F>(deferred: F, init: IV) -> Defer<T, A, V, IV, F>
where
    V: View<T, A>,
    IV: View<T, A>,
    FF: Future<Output = V> + Send + 'static,
    F: Fn() -> FF + Send,
{
    Defer {
        phantom: PhantomData,
        callback: deferred,
        init_view: init,
    }
}
