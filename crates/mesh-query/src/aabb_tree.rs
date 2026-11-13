//! Shared binary64 node core of the owned box hierarchies
//! ([`crate::distance_bvh`], [`crate::scene_bvh`]). Each owner keeps its own
//! split rule (axis choice and item order), so its tree shape, and every
//! answer built on it, stays exactly as before; only the node type, the box
//! accumulation and the recursive split are shared.
//!
//! The packed float32 picking BVH ([`crate::bvh`]) stays separate: its byte
//! layout is fixed by the TypeScript parity fixture.
use std::cmp::Ordering;

pub type P = [f64; 3];

/// Box node: a leaf holds `items`, a branch holds two children and no items.
pub struct BoxTree<T> {
    pub min: P,
    pub max: P,
    pub items: Vec<T>,
    pub children: Option<Box<[BoxTree<T>; 2]>>,
}

/// Node box and item-centre box of the items under a node.
pub struct Extents {
    pub min: P,
    pub max: P,
    pub center_min: P,
    pub center_max: P,
}

/// Split rule of one hierarchy.
pub struct SplitRule<T> {
    pub leaf: usize,
    pub bounds: fn(&T) -> (P, P),
    /// Axis to split a node with these extents.
    pub axis: fn(&Extents) -> usize,
    /// Order along `axis`; the lower half goes left.
    pub order: fn(&T, &T, usize) -> Ordering,
    /// Optional order of leaf items.
    pub leaf_order: Option<fn(&T, &T) -> Ordering>,
}

impl<T> BoxTree<T> {
    /// Median split by `rule`, counting nodes into `nodes`.
    pub fn build_with(mut items: Vec<T>, rule: &SplitRule<T>, nodes: &mut usize) -> Self {
        *nodes += 1;
        let mut e = Extents {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
            center_min: [f64::INFINITY; 3],
            center_max: [f64::NEG_INFINITY; 3],
        };
        for item in &items {
            let (lo, hi) = (rule.bounds)(item);
            for k in 0..3 {
                e.min[k] = e.min[k].min(lo[k]);
                e.max[k] = e.max[k].max(hi[k]);
                let c = lo[k] * 0.5 + hi[k] * 0.5;
                e.center_min[k] = e.center_min[k].min(c);
                e.center_max[k] = e.center_max[k].max(c);
            }
        }
        if items.len() <= rule.leaf {
            if let Some(order) = rule.leaf_order {
                items.sort_by(order);
            }
            return Self {
                min: e.min,
                max: e.max,
                items,
                children: None,
            };
        }
        let axis = (rule.axis)(&e);
        items.sort_by(|a, b| (rule.order)(a, b, axis));
        let right = items.split_off(items.len() / 2);
        Self {
            min: e.min,
            max: e.max,
            items: Vec::new(),
            children: Some(Box::new([
                Self::build_with(items, rule, nodes),
                Self::build_with(right, rule, nodes),
            ])),
        }
    }

    /// Depth-first walk: `enter` decides on each node box, `leaf` receives
    /// the items of every entered leaf. Left before right.
    pub fn walk<'a>(
        &'a self,
        mut enter: impl FnMut(P, P) -> bool,
        mut leaf: impl FnMut(&'a T),
    ) {
        let mut pending = vec![self];
        while let Some(node) = pending.pop() {
            if !enter(node.min, node.max) {
                continue;
            }
            match &node.children {
                Some(children) => {
                    pending.push(&children[1]);
                    pending.push(&children[0]);
                }
                None => node.items.iter().for_each(&mut leaf),
            }
        }
    }
}
