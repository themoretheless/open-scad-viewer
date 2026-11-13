//! Owned scene bounds hierarchy for broadphase picking, using f64 throughout.
use crate::aabb_tree::{BoxTree, SplitRule};
#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub id: i64,
    pub min: [f64; 3],
    pub max: [f64; 3],
}
pub struct Index {
    root: Option<BoxTree<Item>>,
    pub item_count: usize,
    pub node_count: usize,
}
#[derive(Debug, PartialEq)]
pub struct Candidate {
    pub id: i64,
    pub distance: f64,
}
impl Index {
    pub fn build(mut items: Vec<Item>, leaf: usize) -> Self {
        items.retain(|v| {
            v.id.unsigned_abs() <= 9_007_199_254_740_991
                && (0..3)
                    .all(|a| v.min[a].is_finite() && v.max[a].is_finite() && v.min[a] <= v.max[a])
        });
        let item_count = items.len();
        let mut node_count = 0;
        let root = if items.is_empty() {
            None
        } else {
            let rule = SplitRule {
                leaf: leaf.clamp(1, 32),
                ..SCENE_SPLIT
            };
            Some(BoxTree::build_with(items, &rule, &mut node_count))
        };
        Self {
            root,
            item_count,
            node_count,
        }
    }
    pub fn query(&self, origin: [f64; 3], direction: [f64; 3], maximum: f64) -> Vec<Candidate> {
        if !origin.iter().chain(&direction).all(|x| x.is_finite()) || direction == [0.; 3] {
            return vec![];
        }
        let maximum = if maximum.is_finite() {
            maximum.max(0.)
        } else {
            f64::INFINITY
        };
        let mut result = Vec::new();
        if let Some(root) = &self.root {
            root.walk(
                |min, max| entry(origin, direction, min, max, maximum).is_some(),
                |item| {
                    if let Some(distance) = entry(origin, direction, item.min, item.max, maximum) {
                        result.push(Candidate {
                            id: item.id,
                            distance,
                        })
                    }
                },
            );
        }
        result.sort_by(|a, b| a.distance.total_cmp(&b.distance).then(a.id.cmp(&b.id)));
        result
    }
}
const SCENE_SPLIT: SplitRule<Item> = SplitRule {
    leaf: 1,
    bounds: |item| (item.min, item.max),
    // Widest item-centre axis; ties keep the first such axis.
    axis: |e| {
        let mut axis = 0;
        for a in 1..3 {
            if e.center_max[a] - e.center_min[a] > e.center_max[axis] - e.center_min[axis] {
                axis = a
            }
        }
        axis
    },
    order: |a, b, axis| {
        center(a, axis)
            .total_cmp(&center(b, axis))
            .then(a.id.cmp(&b.id))
    },
    leaf_order: Some(|a, b| a.id.cmp(&b.id)),
};
fn center(item: &Item, a: usize) -> f64 {
    item.min[a] * 0.5 + item.max[a] * 0.5
}
fn parameter(bound: f64, origin: f64, direction: f64) -> f64 {
    let delta = bound - origin;
    if delta.is_finite() {
        delta / direction
    } else {
        (bound * 0.5 - origin * 0.5) / (direction * 0.5)
    }
}
fn entry(
    origin: [f64; 3],
    direction: [f64; 3],
    min: [f64; 3],
    max: [f64; 3],
    maximum: f64,
) -> Option<f64> {
    let mut near: f64 = 0.;
    let mut far = maximum;
    for a in 0..3 {
        // A small nonzero direction can enter a distant bound. Only exact zero
        // is parallel; an arbitrary epsilon would discard valid mesh hits.
        if direction[a] == 0. {
            if origin[a] < min[a] || origin[a] > max[a] {
                return None;
            }
            continue;
        }
        let mut start = parameter(min[a], origin[a], direction[a]);
        let mut end = parameter(max[a], origin[a], direction[a]);
        if start > end {
            std::mem::swap(&mut start, &mut end)
        }
        near = near.max(start);
        far = far.min(end);
        if far < near {
            return None;
        }
    }
    near.is_finite().then_some(near)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiny_directions_and_inclusive_bounds() {
        let tree = Index::build(
            vec![Item {
                id: 7,
                min: [1., -1., -1.],
                max: [2., 1., 1.],
            }],
            1,
        );
        assert_eq!(
            tree.query([0.; 3], [1e-14, 0., 0.], 1e14),
            vec![Candidate {
                id: 7,
                distance: 1e14
            }]
        );
        assert!(tree.query([0.; 3], [1e-14, 0., 0.], 9e13).is_empty());
        assert!(tree.query([0.; 3], [0.; 3], f64::INFINITY).is_empty());
    }
    #[test]
    fn ties_invalid_bounds_and_extreme_centers() {
        let tree = Index::build(
            vec![
                Item {
                    id: 8,
                    min: [-1e308; 3],
                    max: [1e308; 3],
                },
                Item {
                    id: 2,
                    min: [-1.; 3],
                    max: [1.; 3],
                },
                Item {
                    id: 3,
                    min: [2.; 3],
                    max: [1.; 3],
                },
            ],
            1,
        );
        assert_eq!(tree.item_count, 2);
        assert_eq!(tree.node_count, 3);
        assert_eq!(
            tree.query([0.; 3], [1., 0., 0.], 0.),
            vec![
                Candidate {
                    id: 2,
                    distance: 0.
                },
                Candidate {
                    id: 8,
                    distance: 0.
                }
            ]
        );
    }
    #[test]
    fn finite_parameter_survives_subtraction_overflow() {
        let tree = Index::build(
            vec![Item {
                id: 1,
                min: [1e308, -1., -1.],
                max: [1.5e308, 1., 1.],
            }],
            1,
        );
        assert_eq!(
            tree.query([-1e308, 0., 0.], [1e308, 0., 0.], 3.),
            vec![Candidate {
                id: 1,
                distance: 2.
            }]
        );
    }
}
