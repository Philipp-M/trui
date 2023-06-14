use crate::widget::{AnyWidget, ChangeFlags, Pod, Widget};
use xilem_core::{Id, IdPath};

xilem_core::generate_view_trait!(View, Widget, Cx, ChangeFlags;);
xilem_core::generate_viewsequence_trait! {ViewSequence, View, ViewMarker, Widget, Cx, ChangeFlags, Pod;}
xilem_core::generate_anyview_trait! {AnyView, View, ViewMarker, Cx, ChangeFlags, AnyWidget}

#[derive(Clone, Default)]
pub struct Cx {
    pub(crate) id_path: IdPath,
}

impl Cx {
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
pub trait Boxed<T, A> {
    fn boxed(self) -> Box<dyn AnyView<T, A>>;
}

impl<T, A, V> Boxed<T, A> for V
where
    V: View<T, A> + 'static,
    V::State: 'static,
    V::Element: 'static,
{
    fn boxed(self) -> Box<dyn AnyView<T, A>> {
        Box::from(self)
    }
}
