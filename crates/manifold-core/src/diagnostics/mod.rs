//! Geometric diagnostics of indexed triangle meshes: self-intersections,
//! wall thickness, clearance between disjoint shells and mass properties.
//!
//! Spatial queries reuse the shared box hierarchy of `mesh-query`
//! ([`mesh_query::aabb_tree::BoxTree`]); every pass with super-linear worst
//! case work takes a typed budget and fails with
//! [`DiagnosticsError::WorkBudgetExceeded`] instead of running unbounded.

mod clearance;
mod mass;
mod predicates;
mod self_intersection;
mod thickness;
mod tri;

pub use clearance::{ClearanceOptions, ClearancePair, ClearanceReport, clearance};
pub use mass::{MassProperties, mass_properties};
pub use self_intersection::{SelfIntersectionOptions, SelfIntersectionReport, self_intersections};
pub use thickness::{ThicknessOptions, ThicknessReport, wall_thickness};

use crate::IndexedMeshError;
use mesh_query::aabb_tree::{BoxTree, SplitRule};

/// Typed failure of a diagnostic pass.
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticsError {
    /// Structurally invalid mesh (index out of range, non-finite position…).
    InvalidMesh(IndexedMeshError),
    /// A parameter is non-finite, negative or otherwise out of its domain.
    InvalidParameter(&'static str),
    /// The pass needs more work units than its budget allows.
    WorkBudgetExceeded {
        stage: &'static str,
        budget: usize,
        required: usize,
    },
    /// Mass properties need a closed (boundary-free) surface.
    OpenMesh,
    /// The enclosed volume is zero, so mass properties are undefined.
    ZeroVolume,
}

impl std::fmt::Display for DiagnosticsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMesh(e) => write!(f, "invalid mesh: {e}"),
            Self::InvalidParameter(name) => write!(f, "invalid parameter: {name}"),
            Self::WorkBudgetExceeded {
                stage,
                budget,
                required,
            } => write!(
                f,
                "{stage} needs at least {required} work units, budget is {budget}"
            ),
            Self::OpenMesh => write!(f, "mesh is not closed"),
            Self::ZeroVolume => write!(f, "mesh encloses zero volume"),
        }
    }
}

impl std::error::Error for DiagnosticsError {}

pub(crate) type P = [f64; 3];

/// Validated mesh with triangles split into corner triples.
pub(crate) struct Mesh<'a> {
    pub positions: &'a [f64],
    pub triangles: Vec<[usize; 3]>,
}

impl<'a> Mesh<'a> {
    pub fn new(positions: &'a [f64], indices: &[usize]) -> Result<Self, DiagnosticsError> {
        crate::validate_indexed_mesh(positions, indices).map_err(DiagnosticsError::InvalidMesh)?;
        let triangles = indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| [t[0], t[1], t[2]])
            .collect();
        Ok(Self {
            positions,
            triangles,
        })
    }

    pub fn point(&self, i: usize) -> P {
        [
            self.positions[i * 3],
            self.positions[i * 3 + 1],
            self.positions[i * 3 + 2],
        ]
    }

    pub fn corners(&self, t: usize) -> [P; 3] {
        self.triangles[t].map(|i| self.point(i))
    }

    /// True for repeated corners or exactly collinear corners.
    pub fn is_degenerate(&self, t: usize) -> bool {
        let [a, b, c] = self.triangles[t];
        if a == b || b == c || a == c {
            return true;
        }
        let [p, q, r] = self.corners(t);
        (0..3).all(|axis| predicates::orient2d(p, q, r, axis).is_eq())
    }
}

/// Leaf item of a triangle box hierarchy.
#[derive(Clone)]
pub(crate) struct TriBox {
    pub index: usize,
    pub min: P,
    pub max: P,
}

const TRI_SPLIT: SplitRule<TriBox> = SplitRule {
    leaf: 8,
    bounds: |t| (t.min, t.max),
    axis: |e| {
        (0..3)
            .max_by(|&a, &b| (e.max[a] - e.min[a]).total_cmp(&(e.max[b] - e.min[b])))
            .unwrap_or(0)
    },
    order: |a, b, axis| (a.min[axis] + a.max[axis]).total_cmp(&(b.min[axis] + b.max[axis])),
    leaf_order: None,
};

/// Box of triangle `t`, grown by `pad` on every side.
pub(crate) fn tri_box(mesh: &Mesh<'_>, t: usize, pad: f64) -> TriBox {
    let c = mesh.corners(t);
    TriBox {
        index: t,
        min: std::array::from_fn(|k| c[0][k].min(c[1][k]).min(c[2][k]) - pad),
        max: std::array::from_fn(|k| c[0][k].max(c[1][k]).max(c[2][k]) + pad),
    }
}

/// Box hierarchy over the non-degenerate triangles accepted by `keep`.
pub(crate) fn triangle_tree(
    mesh: &Mesh<'_>,
    pad: f64,
    keep: impl Fn(usize) -> bool,
) -> Option<BoxTree<TriBox>> {
    let items: Vec<TriBox> = (0..mesh.triangles.len())
        .filter(|&t| keep(t) && !mesh.is_degenerate(t))
        .map(|t| tri_box(mesh, t, pad))
        .collect();
    (!items.is_empty()).then(|| BoxTree::build_with(items, &TRI_SPLIT, &mut 0))
}

pub(crate) fn boxes_overlap(amin: P, amax: P, bmin: P, bmax: P) -> bool {
    (0..3).all(|k| amin[k] <= bmax[k] && bmin[k] <= amax[k])
}

#[cfg(test)]
pub(crate) mod tests {
    /// Outward-oriented axis-aligned box, 8 vertices and 12 triangles.
    pub fn box_mesh(lo: [f64; 3], hi: [f64; 3]) -> (Vec<f64>, Vec<usize>) {
        let mut p = Vec::new();
        for i in 0..8 {
            p.extend([
                if i & 1 == 0 { lo[0] } else { hi[0] },
                if i & 2 == 0 { lo[1] } else { hi[1] },
                if i & 4 == 0 { lo[2] } else { hi[2] },
            ]);
        }
        let i = vec![
            0, 2, 3, 0, 3, 1, // z = lo
            4, 5, 7, 4, 7, 6, // z = hi
            0, 1, 5, 0, 5, 4, // y = lo
            2, 6, 7, 2, 7, 3, // y = hi
            0, 4, 6, 0, 6, 2, // x = lo
            1, 3, 7, 1, 7, 5, // x = hi
        ];
        (p, i)
    }

    pub fn merge(a: (Vec<f64>, Vec<usize>), b: (Vec<f64>, Vec<usize>)) -> (Vec<f64>, Vec<usize>) {
        let offset = a.0.len() / 3;
        let mut p = a.0;
        p.extend(b.0);
        let mut i = a.1;
        i.extend(b.1.iter().map(|v| v + offset));
        (p, i)
    }

    pub fn flip(i: &[usize]) -> Vec<usize> {
        i.as_chunks::<3>()
            .0
            .iter()
            .flat_map(|t| [t[0], t[2], t[1]])
            .collect()
    }

    #[test]
    fn box_helper_is_outward_and_closed() {
        let (p, i) = box_mesh([0.; 3], [1., 2., 3.]);
        let m = crate::metrics(&p, &i);
        assert!(m.watertight && (m.signed_volume - 6.0).abs() < 1e-12);
    }
}
