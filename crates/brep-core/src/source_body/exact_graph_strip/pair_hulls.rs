//! Conservative all-pair broad phase on actual exact source definitions.
//! Hull overlap, including a shared edge, remains unresolved, never accepted.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairHullStatus {
    Separated { axis: usize },
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct FacePairHull {
    pub faces: [usize; 2],
    pub status: PairHullStatus,
    /// Incidence metadata only; sharing an edge does not prove no intersection.
    pub shared_edges: Vec<usize>,
}
pub struct PairHullAudit {
    pairs: Vec<FacePairHull>,
}
impl PairHullAudit {
    pub fn pairs(&self) -> &[FacePairHull] {
        &self.pairs
    }
    pub fn separated(&self) -> usize {
        self.pairs
            .iter()
            .filter(|p| matches!(p.status, PairHullStatus::Separated { .. }))
            .count()
    }
    pub fn unresolved(&self) -> usize {
        self.pairs.len() - self.separated()
    }
}
fn bounds(points: impl IntoIterator<Item = Result<[Q; 3]>>) -> Result<[[Q; 2]; 3]> {
    let mut out: Option<[[Q; 2]; 3]> = None;
    for point in points {
        let point = point?;
        require(
            point.iter().all(|x| {
                x.denom().is_positive() && x.numer().bits() <= 8192 && x.denom().bits() <= 8192
            }),
            "Invalid pair-hull source coefficient",
        )?;
        if let Some(r) = &mut out {
            for k in 0..3 {
                r[k][0] = r[k][0].clone().min(point[k].clone());
                r[k][1] = r[k][1].clone().max(point[k].clone());
            }
        } else {
            out = Some(std::array::from_fn(|k| {
                [point[k].clone(), point[k].clone()]
            }));
        }
    }
    out.ok_or_else(|| Error::new("BREP_EXACT_PAIR_HULL", "Empty source hull"))
}
fn original(s: &nurbs_core::surface::Surface) -> Result<[[Q; 2]; 3]> {
    s.validate()?;
    require(
        !s.periodic_u
            && !s.periodic_v
            && s.control_points.len() == s.degree_u + 1
            && s.control_points[0].len() == s.degree_v + 1
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && s.weights.iter().flatten().all(|w| *w == s.weights[0][0]),
        "Pair hull requires qualified polynomial source support",
    )?;
    bounds(
        s.control_points
            .iter()
            .flatten()
            .map(|p| Ok(std::array::from_fn(|k| Q::from_float(p[k]).unwrap()))),
    )
}
impl ClosedShell {
    /// Visits every unordered pair or refuses before producing a report.
    /// Uses full original support hulls for retained faces (conservative).
    /// Strict hull disjointness is proven; touching/overlapping hulls are not.
    /// This does not inspect within-face intersections or admit a solid.
    pub fn inspect_pair_hulls(&self, max_pairs: usize, b: &mut Budget) -> Result<PairHullAudit> {
        let top = self.topology();
        let n = top.faces.len();
        let count = n
            .checked_mul(n.saturating_sub(1))
            .and_then(|v| v.checked_div(2))
            .ok_or_else(|| Error::new("BREP_EXACT_PAIR_HULL", "Pair count overflow"))?;
        require(
            count <= max_pairs && max_pairs <= 1_000_000,
            "Exact face-pair budget exhausted",
        )?;
        let geometry = inspect_roof_geometry(top, b)?;
        require(
            top.shells.len() == 1 && top.shells[0].closed && geometry.open_edges.is_empty(),
            "Pair hull audit requires exact closed shell",
        )?;
        let mut boxes = Vec::new();
        let mut edges = Vec::new();
        for face in &top.faces {
            boxes.push(match &face.surface {
                FaceGeometry::Transition(p) => {
                    bounds(p.coefficients().iter().flatten().cloned().map(Ok))?
                }
                FaceGeometry::Retained(r) => original(r.source())?,
                FaceGeometry::Imported(f) => original(f.source())?,
            });
            edges.push(
                top.loops[face.outer]
                    .coedges
                    .iter()
                    .map(|c| c.edge)
                    .collect::<Vec<_>>(),
            );
        }
        let mut pairs = Vec::with_capacity(count);
        for i in 0..n {
            for j in i + 1..n {
                let axis = (0..3)
                    .find(|&k| boxes[i][k][1] < boxes[j][k][0] || boxes[j][k][1] < boxes[i][k][0]);
                let status = axis.map_or(PairHullStatus::Unresolved, |axis| {
                    PairHullStatus::Separated { axis }
                });
                let shared_edges = edges[i]
                    .iter()
                    .filter(|e| edges[j].contains(e))
                    .copied()
                    .collect();
                pairs.push(FacePairHull {
                    faces: [i, j],
                    status,
                    shared_edges,
                });
            }
        }
        Ok(PairHullAudit { pairs })
    }
}
