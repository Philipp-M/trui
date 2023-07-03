use std::{collections::HashSet, sync::mpsc::SyncSender};

use crate::widget::{AnyWidget, ChangeFlags, Pod, Widget};
use xilem_core::{Id, IdPath};

xilem_core::generate_view_trait!(View, Widget, Cx, ChangeFlags; : Send);
xilem_core::generate_viewsequence_trait! {ViewSequence, View, ViewMarker, Widget, Cx, ChangeFlags, Pod; : Send}
xilem_core::generate_anyview_trait! {AnyView, View, ViewMarker, Cx, ChangeFlags, AnyWidget, BoxedView; + Send}

#[derive(Clone)]
pub struct Cx {
    id_path: IdPath,
    req_chan: SyncSender<IdPath>,
    pub(crate) pending_async: HashSet<Id>,
}

impl Cx {
    pub(crate) fn new(req_chan: &SyncSender<IdPath>) -> Self {
        Cx {
            id_path: Vec::new(),
            req_chan: req_chan.clone(),
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
}

// TODO put this into the "xilem_core::generate_anyview_trait!" macro?
pub trait IntoBoxedView<T, A = ()>: Send {
    fn boxed(self) -> BoxedView<T, A>;
}

impl<T, A, V> IntoBoxedView<T, A> for V
where
    V: View<T, A> + 'static,
    V::State: 'static,
    V::Element: 'static,
{
    fn boxed(self) -> BoxedView<T, A> {
        Box::from(self)
    }
}
