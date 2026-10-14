//! Whole positive-rational control hulls, with exact binary64 ordering.
//! A strict coordinate gap proves every pair in a face/range product disjoint.
use crate::Model;

type Bounds = [[f64; 2]; 3];
#[derive(Clone, Debug)]
pub struct Certificate {
    pub face: usize,
    /// Half-open contiguous face range, wholly after `face`.
    pub range: [usize; 2],
    pub axis: usize,
    pub first_before_range: bool,
    pub first_bounds: Bounds,
    pub range_bounds: Bounds,
}
pub(crate) struct Node {
    pub range: [usize; 2],
    pub children: Option<[usize; 2]>,
    bounds: Option<Bounds>,
}
pub(crate) struct Cover {
    pub nodes: Vec<Node>,
    pub root: usize,
    faces: Vec<Option<Bounds>>,
}
impl Cover {
    /// Each original pole and tree node consumes one shared geometry cell.
    /// No allocation or traversal of additional poles follows exhaustion.
    pub fn prepare(model: &Model, budget: usize) -> (Option<Self>, usize) {
        let mut cells = 0;
        let mut faces = Vec::new();
        for face in &model.faces {
            let s = &face.surface;
            let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
            let mut valid = !s.periodic_u && !s.periodic_v;
            let natural = [
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            ];
            // A pole hull bounds the chart only on its natural domain. Prove
            // containment of every authored trim curve too, conservatively.
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for coedge in &model.loops[wire].coedges {
                    for (point, weight) in coedge
                        .pcurve
                        .control_points
                        .iter()
                        .zip(&coedge.pcurve.weights)
                    {
                        if cells == budget {
                            return (None, cells);
                        }
                        cells += 1;
                        valid &= *weight > 0. && weight.is_finite();
                        for k in 0..2 {
                            valid &= point[k] >= natural[k][0] && point[k] <= natural[k][1];
                        }
                    }
                }
            }
            for (row, weights) in s.control_points.iter().zip(&s.weights) {
                for (point, weight) in row.iter().zip(weights) {
                    if cells == budget {
                        return (None, cells);
                    }
                    cells += 1;
                    valid &= *weight > 0. && weight.is_finite();
                    for k in 0..3 {
                        valid &= point[k].is_finite();
                        bounds[k][0] = bounds[k][0].min(point[k]);
                        bounds[k][1] = bounds[k][1].max(point[k]);
                    }
                }
            }
            faces.push(valid.then_some(bounds));
        }
        if faces.is_empty() {
            return (None, cells);
        }
        fn build(
            faces: &[Option<Bounds>],
            range: [usize; 2],
            nodes: &mut Vec<Node>,
            cells: &mut usize,
            budget: usize,
        ) -> Option<usize> {
            if *cells == budget {
                return None;
            }
            *cells += 1;
            let children = if range[1] - range[0] > 1 {
                let mid = range[0] + (range[1] - range[0]) / 2;
                Some([
                    build(faces, [range[0], mid], nodes, cells, budget)?,
                    build(faces, [mid, range[1]], nodes, cells, budget)?,
                ])
            } else {
                None
            };
            let bounds = if let Some([a, b]) = children {
                nodes[a].bounds.zip(nodes[b].bounds).map(|(a, b)| {
                    std::array::from_fn(|k| [a[k][0].min(b[k][0]), a[k][1].max(b[k][1])])
                })
            } else {
                faces[range[0]]
            };
            let index = nodes.len();
            nodes.push(Node {
                range,
                children,
                bounds,
            });
            Some(index)
        }
        let mut nodes = Vec::new();
        let Some(root) = build(&faces, [0, faces.len()], &mut nodes, &mut cells, budget) else {
            return (None, cells);
        };
        (Some(Self { nodes, root, faces }), cells)
    }
    /// Caller charges one shared cell for this attempted group certificate.
    pub fn certify(&self, face: usize, node: usize) -> Option<Certificate> {
        let node = &self.nodes[node];
        if node.range[0] <= face || node.range[1] <= node.range[0] {
            return None;
        }
        let a = self.faces[face]?;
        let b = node.bounds?;
        for axis in 0..3 {
            // No arithmetic rounding or tolerance: extrema are original pole
            // coordinates, and positive rational basis functions form a hull.
            if a[axis][1] < b[axis][0] || b[axis][1] < a[axis][0] {
                return Some(Certificate {
                    face,
                    range: node.range,
                    axis,
                    first_before_range: a[axis][1] < b[axis][0],
                    first_bounds: a,
                    range_bounds: b,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn separated() -> Model {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let surface = model.faces[0].surface.clone();
        for (i, face) in model.faces.iter_mut().enumerate() {
            face.surface = surface.clone();
            for row in &mut face.surface.control_points {
                for point in row {
                    point[0] += 10. * i as f64;
                }
            }
        }
        model
    }
    #[test]
    fn strict_original_hulls_cover_ranges_and_charge_preparation() {
        let model = separated();
        let before = format!("{model:?}");
        let (cover, cells) = Cover::prepare(&model, 1000);
        let cover = cover.unwrap();
        assert!(cells > model.faces.len());
        assert!(Cover::prepare(&model, 0).0.is_none());
        assert_eq!(Cover::prepare(&model, cells - 1).1, cells - 1);
        assert!(Cover::prepare(&model, cells - 1).0.is_none());
        let node = cover.nodes.iter().position(|n| n.range == [3, 6]).unwrap();
        let certificate = cover.certify(0, node).unwrap();
        assert_eq!(certificate.axis, 0);
        assert_eq!(certificate.range, [3, 6]);
        assert!(certificate.first_before_range);
        // A singleton range carries the same strict original-hull proof;
        // it need not consume the separate expensive pair-search quota.
        let leaf = cover.nodes.iter().position(|n| n.range == [3, 4]).unwrap();
        assert_eq!(cover.certify(0, leaf).unwrap().range, [3, 4]);
        assert_eq!(format!("{model:?}"), before);
        let mut outside = model.clone();
        let wire = outside.faces[3].outer;
        outside.loops[wire].coedges[0].pcurve.control_points[0][0] = -1.;
        assert!(
            Cover::prepare(&outside, 1000)
                .0
                .unwrap()
                .certify(0, node)
                .is_none()
        );
        let mut periodic = model.clone();
        periodic.faces[3].surface.periodic_u = true;
        assert!(
            Cover::prepare(&periodic, 1000)
                .0
                .unwrap()
                .certify(0, node)
                .is_none()
        );
    }
    #[test]
    fn contact_and_one_ulp_gap_are_distinct_without_tolerance() {
        let mut model = separated();
        for (i, face) in model.faces.iter_mut().enumerate() {
            for row in &mut face.surface.control_points {
                for point in row {
                    point[0] = if i == 0 { 1. } else { 1_f64.next_up() };
                }
            }
        }
        let (cover, _) = Cover::prepare(&model, 1000);
        let cover = cover.unwrap();
        let node = cover.nodes.iter().position(|n| n.range == [3, 6]).unwrap();
        assert!(cover.certify(0, node).is_some());
        for row in &mut model.faces[3].surface.control_points {
            for point in row {
                point[0] = 1.;
            }
        }
        let (cover, _) = Cover::prepare(&model, 1000);
        assert!(cover.unwrap().certify(0, node).is_none());
    }
}
