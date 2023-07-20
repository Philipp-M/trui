use std::{
    collections::HashSet,
    sync::{mpsc::SyncSender, Arc},
};

use futures_task::{ArcWake, Waker};

use crate::widget::{AnyWidget, ChangeFlags, Pod, Widget};
use xilem_core::{Id, IdPath};

xilem_core::generate_view_trait!(View <C>, Widget, Cx<C>, ChangeFlags; : Send);
xilem_core::generate_viewsequence_trait! {ViewSequence, View <C>, ViewMarker, Widget, Cx<C>, ChangeFlags, Pod; : Send}
xilem_core::generate_anyview_trait! {AnyView, View <C>, ViewMarker, Cx<C>, ChangeFlags, AnyWidget, BoxedView; + Send}
xilem_core::generate_memoize_view! {Memoize, MemoizeState, View <C>, ViewMarker, Cx<C>, ChangeFlags, s, memoize; + Send}

pub struct Cx<C> {
    id_path: IdPath,
    req_chan: SyncSender<IdPath>,
    pub app_context: C,
    pub(crate) pending_async: HashSet<Id>,
}

impl<C> Cx<C> {
    pub(crate) fn new(req_chan: &SyncSender<IdPath>, app_context: C) -> Self {
        Cx {
            id_path: Vec::new(),
            req_chan: req_chan.clone(),
            app_context,
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
    pub fn with_id<T, F: FnOnce(&mut Cx<C>) -> T>(&mut self, id: Id, f: F) -> T {
        self.push(id);
        let result = f(self);
        self.pop();
        result
    }

    /// Allocate a new id and run logic with the new id added to the id path.
    ///
    /// Also an ergonomic helper.
    pub fn with_new_id<T, F: FnOnce(&mut Cx<C>) -> T>(&mut self, f: F) -> (Id, T) {
        let id = Id::next();
        self.push(id);
        let result = f(self);
        self.pop();
        (id, result)
    }

    pub fn ui_waker(&self) -> Waker {
        futures_task::waker(Arc::new(MyWaker {
            id_path: self.id_path.clone(),
            req_chan: self.req_chan.clone(),
        }))
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
pub trait IntoBoxedView<T, C, A = ()> {
    fn boxed(self) -> BoxedView<T, C, A>;
}

impl<T, C, A, V> IntoBoxedView<T, C, A> for V
where
    V: View<T, C, A> + 'static,
    V::State: 'static,
    V::Element: 'static,
{
    fn boxed(self) -> BoxedView<T, C, A> {
        Box::from(self)
    }
}
