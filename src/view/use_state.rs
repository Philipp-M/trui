use std::{
    any::Any,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use xilem_core::{Id, MessageResult};

use crate::{widget::ChangeFlags, Cx, View, ViewMarker};

/// This Handle is a workaround to erase the lifetime of &mut T,
/// it can only be constructed in contexts,
/// where it is actually safe to use (such as UseState)
pub struct Handle<T>(*mut T);

impl<T> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T> DerefMut for Handle<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}

/// An implementation of the "use_state" pattern familiar in reactive UI.
///
/// This may not be the final form. In this version, the parent app data
/// is `Rc<T>`, and the child is `(Rc<T>, S)` where S is the local state.
///
/// The first callback creates the initial state (it is called on build but
/// not rebuild). The second callback takes that state as an argument. It
/// is not passed the app state, but since that state is `Rc`, it would be
/// natural to clone it and capture it in a `move` closure.
pub struct UseState<T, A, S, V, FInit, F> {
    f_init: FInit,
    f: F,
    #[allow(clippy::type_complexity)]
    phantom: PhantomData<fn() -> (T, A, S, V)>,
}

pub struct UseStateState<T, A, S, V: View<(Handle<T>, S), A>> {
    state: Option<S>,
    view: V,
    view_state: V::State,
}

impl<T, A, S, V, FInit: Fn() -> S, F: Fn(&mut S) -> V> UseState<T, A, S, V, FInit, F> {
    #[allow(unused)]
    pub fn new(f_init: FInit, f: F) -> Self {
        let phantom = Default::default();
        UseState { f_init, f, phantom }
    }
}

impl<T, A, S, V, FInit, F> ViewMarker for UseState<T, A, S, V, FInit, F> {}

impl<T, A, S, V, FInit, F> View<T, A> for UseState<T, A, S, V, FInit, F>
where
    V: View<(Handle<T>, S), A>,
    S: Send,
    FInit: Fn() -> S + Send + Sync,
    F: Fn(&mut S) -> V + Send + Sync,
{
    type State = UseStateState<T, A, S, V>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut state = (self.f_init)();
        let view = (self.f)(&mut state);
        let (id, view_state, element) = view.build(cx);
        let my_state = UseStateState {
            state: Some(state),
            view,
            view_state,
        };
        (id, my_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let view = (self.f)(state.state.as_mut().unwrap());
        let changeflags = view.rebuild(cx, &state.view, id, &mut state.view_state, element);
        state.view = view;
        changeflags
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        let p = Handle(app_state);
        let mut local_state = (p, state.state.take().unwrap());
        let a = state
            .view
            .message(id_path, &mut state.view_state, event, &mut local_state);
        let (_, my_state) = local_state;
        state.state = Some(my_state);
        a
    }
}

/// "Injects" additional local state into the provided view (which is returned by `f`)
/// The initial state is provided by `f_init` (when this view is first built)
pub fn use_state<T, A, S, V, FInit, F>(f_init: FInit, f: F) -> UseState<T, A, S, V, FInit, F>
where
    V: View<(Handle<T>, S), A>,
    FInit: Fn() -> S,
    F: Fn(&mut S) -> V,
{
    UseState::new(f_init, f)
}

pub trait WithState<T, A, Vi: View<T, A>>: Into<Arc<Vi>> {
    /// Compose a view with added local state.
    /// The local state is added within a closure additional to the app state via a tuple.
    /// It's initialized with the first closure.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// fn with_counter<T, V: View<T> + Clickable>(view: V) -> impl View<T> {
    ///     view.with_state(
    ///         || 42,
    ///         |view, local_state| {
    ///             v_stack((
    ///                 format!("Click the view below to increment this: {local_state}"),
    ///                 view.on_click(|(_app_state, local_state): &mut (Handle<T>, i32)| {
    ///                     *local_state += 1;
    ///                 }),
    ///             ))
    ///         },
    ///     )
    /// }
    /// ```
    fn with_state<
        S: Send,
        Finit: Fn() -> S + Send + Sync,
        Vo: View<(Handle<T>, S), A>,
        F: Fn(HandleState<Vi>, &mut S) -> Vo + Send + Sync,
    >(
        self,
        init: Finit,
        view_factory: F,
    ) -> WithLocalState<Finit, F, Vi, Vo> {
        WithLocalState {
            init,
            view: self.into(),
            view_factory,
            phantom: PhantomData,
        }
    }
}

impl<T, A, Vi: View<T, A>, V: Into<Arc<Vi>>> WithState<T, A, Vi> for V {}

pub struct HandleState<V>(Arc<V>);

impl<V> ViewMarker for HandleState<V> {}

impl<T, A, V: View<T, A>, S> View<(Handle<T>, S), A> for HandleState<V> {
    type State = V::State;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        self.0.build(cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        self.0.rebuild(cx, &prev.0, id, state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        (app_state, _local_state): &mut (Handle<T>, S),
    ) -> MessageResult<A> {
        self.0.message(id_path, state, message, app_state)
    }
}

pub struct WithLocalState<Finit, F, Vi, Vo> {
    init: Finit,
    view: Arc<Vi>,
    view_factory: F,
    phantom: PhantomData<fn() -> (Vi, Vo)>,
}

impl<Vi, Vo, Finit, F> ViewMarker for WithLocalState<Finit, F, Vi, Vo> {}

impl<
        T,
        A,
        S: Send,
        Finit: Fn() -> S + Send + Sync,
        Vi: View<T, A>,
        Vo: View<(Handle<T>, S), A>,
        F: Fn(HandleState<Vi>, &mut S) -> Vo + Send + Sync,
    > View<T, A> for WithLocalState<Finit, F, Vi, Vo>
{
    type State = (Option<S>, Vo, Vo::State);

    type Element = Vo::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut local_state = (self.init)();
        let handle_state = HandleState(self.view.clone());
        let view = (self.view_factory)(handle_state, &mut local_state);
        let (id, vo_state, element) = view.build(cx);

        (id, (Some(local_state), view, vo_state), element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        id: &mut Id,
        (local_state, state_view, state_view_state): &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let handle_state = HandleState(self.view.clone());
        let view = (self.view_factory)(handle_state, local_state.as_mut().unwrap());
        let changeflags = view.rebuild(cx, state_view, id, state_view_state, element);
        *state_view = view;
        changeflags
    }

    fn message(
        &self,
        id_path: &[Id],
        (local_state, state_view, state_view_state): &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        let p = Handle(app_state);
        let mut state = (p, local_state.take().unwrap());
        let a = state_view.message(id_path, state_view_state, message, &mut state);
        let (_, new_local_state) = state;
        *local_state = Some(new_local_state);
        a
    }
}
