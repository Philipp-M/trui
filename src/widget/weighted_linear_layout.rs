use crate::geometry::{Axis, Size};

use super::{
    animatables::AnimatableElement,
    core::{EventCx, PaintCx},
    BoxConstraints, LayoutCx, LifeCycle, LifeCycleCx, Pod, Widget,
};

pub struct WeightedLinearLayout {
    pub children: Vec<Pod>,
    pub weights: Vec<f64>,
    pub axis: Axis,
}

pub struct WeightedLayoutElement {
    pub(crate) content: Pod,
    pub(crate) weight_animatable: Box<dyn AnimatableElement<f64>>,
    weight: f64,
}

impl WeightedLayoutElement {
    pub(crate) fn new(content: impl Widget, weight_element: impl AnimatableElement<f64>) -> Self {
        Self {
            content: Pod::new(content),
            weight_animatable: Box::new(weight_element),
            weight: 1.0,
        }
    }
}

impl Widget for WeightedLayoutElement {
    fn paint(&mut self, cx: &mut PaintCx) {
        self.content.paint(cx)
    }
    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        self.content.layout(cx, bc)
    }
    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        if let LifeCycle::Animate = event {
            let new_weight = *self.weight_animatable.animate(cx);
            if new_weight != self.weight {
                cx.request_layout();
                self.weight = new_weight;
            }
        }
        self.content.lifecycle(cx, event);
    }
    fn event(&mut self, cx: &mut EventCx, event: &super::Event) {
        self.content.event(cx, event)
    }
}

fn get_weights(children: &[Pod], weights: &mut Vec<f64>) -> f64 {
    weights.clear();
    let mut sum = 0.0;
    for child in children {
        let weight = if let Some(weighted_el) = child.downcast_ref::<WeightedLayoutElement>() {
            weighted_el.weight
        } else {
            1.0
        };
        sum += weight;
        weights.push(weight);
    }
    sum
}

impl WeightedLinearLayout {
    pub(crate) fn new(children: Vec<Pod>, axis: Axis) -> Self {
        let weights = Vec::with_capacity(children.len());
        WeightedLinearLayout {
            children,
            axis,
            weights,
        }
    }
}

impl Widget for WeightedLinearLayout {
    fn paint(&mut self, cx: &mut PaintCx) {
        for child in self.children.iter_mut() {
            child.paint(cx);
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let mut major_used: f64 = 0.0;
        let mut max_minor: f64 = 0.0;
        let major_max = self.axis.major(*bc).end;

        let total_weight_inv = 1.0 / get_weights(&self.children, &mut self.weights);
        let space_available = self.axis.major(*bc).end;

        for (index, child) in self.children.iter_mut().enumerate() {
            let constraint = if space_available != f64::INFINITY {
                let size = (space_available * (self.weights[index] * total_weight_inv))
                    .min(major_max - major_used);
                size..size // TODO loosen the minimum size (to 0)?
            } else {
                0.0..f64::INFINITY
            };
            let child_bc = self.axis.with_major(bc.loosen(), constraint);
            let size = child.layout(cx, &child_bc);
            child.set_origin(cx, self.axis.pack(major_used, 0.0));
            major_used += self.axis.major(size);
            max_minor = max_minor.max(self.axis.minor(size));
        }

        bc.constrain(self.axis.pack::<Size>(major_used, max_minor))
    }

    fn event(&mut self, cx: &mut EventCx, event: &super::Event) {
        for child in &mut self.children {
            child.event(cx, event);
        }
    }

    fn lifecycle(&mut self, cx: &mut super::core::LifeCycleCx, event: &LifeCycle) {
        for child in &mut self.children {
            child.lifecycle(cx, event);
        }
    }
}
