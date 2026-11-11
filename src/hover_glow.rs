use super::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
#[derive(Default, Clone)]
pub(super) struct HoverGlow(Rc<RefCell<Glow>>);
#[derive(Default)]
struct Glow {
    rows: HashMap<usize, Bounds<Pixels>>,
    target: Option<Bounds<Pixels>>,
    active: Option<usize>,
    visible: bool,
}
impl HoverGlow {
    pub fn measure(&self, index: usize, bounds: Bounds<Pixels>) -> bool {
        let mut state = self.0.borrow_mut();
        state.rows.insert(index, bounds);
        let changed = state.active == Some(index) && state.target != Some(bounds);
        if changed {
            state.target = Some(bounds);
        }
        changed
    }
    pub fn activate(&self, index: usize) {
        let mut state = self.0.borrow_mut();
        state.active = Some(index);
        if let Some(bounds) = state.rows.get(&index).copied() {
            state.target = Some(bounds);
            state.visible = true;
        }
    }
    pub fn hide(&self) {
        self.0.borrow_mut().visible = false;
    }
    pub fn view(&self, id: &'static str, color: Hsla) -> AnyElement {
        let state = self.0.borrow();
        let Some(target) = state.target else {
            return div().absolute().into_any_element();
        };
        let targets = [
            f32::from(target.origin.x),
            f32::from(target.origin.y),
            f32::from(target.size.width),
            f32::from(target.size.height),
            if state.visible { 1. } else { 0. },
        ];
        let values = Rc::new(RefCell::new(targets));
        let paint = values.clone();
        let mut layer = canvas(
            |_, _, _| {},
            move |_, _, w, _| {
                let v = *paint.borrow();
                let mut quad = fill(
                    Bounds::new(point(px(v[0]), px(v[1])), size(px(v[2]), px(v[3]))),
                    color.opacity(v[4].clamp(0., 1.)),
                );
                quad.corner_radii = px(8.).into();
                w.paint_quad(quad);
            },
        )
        .absolute()
        .inset_0()
        .into_any_element();
        // Same critically damped spring as Flowmoor's shared hover highlight.
        for (index, target) in targets.into_iter().enumerate() {
            let values = values.clone();
            layer = layer
                .with_spring(
                    (id, index),
                    SpringAnimation::new(SpringConfig::new(1400., 30., 0.16))
                        .to(target)
                        .from(if index == 4 { 0. } else { target }),
                    move |layer, value| {
                        values.borrow_mut()[index] = value;
                        layer
                    },
                )
                .into_any_element();
        }
        layer
    }
}
