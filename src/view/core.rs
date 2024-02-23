use std::{
    collections::HashSet,
    sync::{mpsc::SyncSender, Arc},
};

use crate::widget::{AnyWidget, ChangeFlags, Pod, Widget};
use futures_task::{ArcWake, Waker};
use xilem_core::{Id, IdPath};

xilem_core::generate_view_trait!(View, Widget, Cx, ChangeFlags; (ViewMarker + Send + Sync), (Send));
xilem_core::generate_viewsequence_trait! {ViewSequence, View, ViewMarker, Widget, Cx, ChangeFlags, Pod; (Send + Sync), (Send)}
xilem_core::generate_anyview_trait! {AnyView, View, ViewMarker, Cx, ChangeFlags, AnyWidget; (Send + Sync), (Send)}
xilem_core::generate_memoize_view! {Memoize, MemoizeState, View, ViewMarker, Cx, ChangeFlags, static_view, memoize; + Send + Sync}
xilem_core::generate_adapt_view! {View, Cx, ChangeFlags; + Send + Sync}
xilem_core::generate_adapt_state_view! {View, Cx, ChangeFlags; + Send + Sync}
xilem_core::generate_rc_view!(std::sync::Arc, View, ViewMarker, Cx, ChangeFlags, AnyView, AnyWidget; Send);

pub struct Cx {
    id_path: IdPath,
    req_chan: SyncSender<IdPath>,
    pub rt: tokio::runtime::Handle,
    pub(crate) pending_async: HashSet<Id>,
}

impl Cx {
    pub(crate) fn new(req_chan: &SyncSender<IdPath>, rt: tokio::runtime::Handle) -> Self {
        Cx {
            id_path: Vec::new(),
            req_chan: req_chan.clone(),
            rt,
            pending_async: HashSet::new(),
        }
    }

    pub fn push(&mut self, id: Id) {
        self.id_path.push(id);
    }

    pub fn pop(&mut self) {
        self.id_path.pop();
    }

    pub fn is_empty(&self) -> bool {
        self.id_path.is_empty()
    }

    pub fn id_path(&self) -> &IdPath {
        &self.id_path
    }

    /// Run some logic with an id added to the id path.
    ///
    /// This is an ergonomic helper that ensures proper nesting of the id path.
    pub fn with_id<T, F: FnOnce(&mut Cx) -> T>(&mut self, id: Id, f: F) -> T {
        self.push(id);
        let result = f(self);
        self.pop();
        result
    }

    /// Allocate a new id and run logic with the new id added to the id path.
    ///
    /// Also an ergonomic helper.
    pub fn with_new_id<T, F: FnOnce(&mut Cx) -> T>(&mut self, f: F) -> (Id, T) {
        let id = Id::next();
        self.push(id);
        let result = f(self);
        self.pop();
        (id, result)
    }

    pub fn waker(&self) -> Waker {
        futures_task::waker(Arc::new(MyWaker {
            id_path: self.id_path.clone(),
            req_chan: self.req_chan.clone(),
        }))
    }

    /// Add an id for a pending async future.
    ///
    /// Rendering may be delayed when there are pending async futures, to avoid
    /// flashing, and continues when all futures complete, or a timeout, whichever
    /// is first.
    pub fn add_pending_async(&mut self, id: Id) {
        self.pending_async.insert(id);
    }
}

struct MyWaker {
    id_path: IdPath,
    req_chan: SyncSender<IdPath>,
}

impl ArcWake for MyWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let _ = arc_self.req_chan.send(arc_self.id_path.clone());
    }
}

// TODO put this into the "xilem_core::generate_anyview_trait!" macro?
pub trait IntoBoxedView<T, A = ()> {
    fn boxed(self) -> Box<dyn AnyView<T, A>>;
}

// Same as `ViewExt` here, should these be their own traits, or just additional methods to the `View` trait?
impl<T, A, V> IntoBoxedView<T, A> for V
where
    V: View<T, A> + 'static,
    V::State: 'static,
    V::Element: 'static,
{
    fn boxed(self) -> Box<dyn AnyView<T, A>> {
        Box::from(self)
    }
}
