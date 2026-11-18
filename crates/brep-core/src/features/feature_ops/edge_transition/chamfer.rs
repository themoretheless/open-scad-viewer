//! Closed straight-edge chamfers with whole original-curve setback evidence.
use super::automatic::{self, Boundaries, CrossSection};
use super::{Ledger, Limits, TopologyTransition, insert_four_faces, refused};
use crate::{Model, Result};
use nurbs_core::{curve::Curve, interval_eval::Interval as I};
/// Whole positive-rational control-hull distance to the stored original straight
/// carrier. Snapshots bind the proof to the original edge and actual saved rail.
/// Source-line representation deviation is included in the returned error bound.
pub struct SetbackCertificate {
    original: Curve,
    actual: Curve,
    line: [[f64; 3]; 2],
    edge: usize,
    requested: f64,
    distance: [f64; 2],
    source_deviation: f64,
    error: f64,
}
impl SetbackCertificate {
    pub fn original_curve(&self) -> &Curve {
        &self.original
    }
    pub fn actual_curve(&self) -> &Curve {
        &self.actual
    }
    pub fn line(&self) -> [[f64; 3]; 2] {
        self.line
    }
    pub fn edge_index(&self) -> usize {
        self.edge
    }
    pub fn requested(&self) -> f64 {
        self.requested
    }
    pub fn distance_interval(&self) -> [f64; 2] {
        self.distance
    }
    pub fn source_line_deviation(&self) -> f64 {
        self.source_deviation
    }
    pub fn error_upper(&self) -> f64 {
        self.error
    }
}
pub struct ChamferTransition {
    topology: TopologyTransition,
    setbacks: [Vec<SetbackCertificate>; 2],
}
impl std::ops::Deref for ChamferTransition {
    type Target = TopologyTransition;
    fn deref(&self) -> &Self::Target {
        &self.topology
    }
}
impl ChamferTransition {
    pub fn into_model(self) -> Model {
        self.topology.into_model()
    }
    /// Order is the two original support IDs returned by source_supports().
    pub fn setbacks(&self) -> &[Vec<SetbackCertificate>; 2] {
        &self.setbacks
    }
}
fn norm_bounds(v: [I; 3]) -> Result<[f64; 2]> {
    let mut square = I::point(0.);
    for x in v {
        square = square.add(x.mul(x)?.intersect(0., f64::INFINITY)?)?;
    }
    Ok([
        square.lo.max(0.).sqrt().next_down().max(0.),
        square.hi.sqrt().next_up(),
    ])
}
/// Orthogonal projection is an affine function of each ORIGINAL Euclidean
/// pole. Positive rational B-spline basis functions preserve its component
/// hull on every original span. No sampled distance or fitted line is admitted.
fn line_distance(c: &Curve, line: [[f64; 3]; 2], work: &mut Ledger) -> Result<[f64; 2]> {
    c.validate()?;
    if c.periodic || c.control_points[0].len() != 3 || c.weights.iter().any(|w| *w <= 0.) {
        return Err(refused(
            "Setback proof requires an original positive rational curve",
        ));
    }
    work.charge(c.control_points.len(), 0)?;
    let mut d = [I::point(0.); 3];
    let mut denominator = I::point(0.);
    for k in 0..3 {
        d[k] = I::point(line[1][k]).sub(I::point(line[0][k]))?;
        denominator = denominator.add(d[k].mul(d[k])?)?;
    }
    if denominator.lo <= 0. {
        return Err(refused("Original straight carrier is unresolved"));
    }
    let mut hull = [I::point(0.); 3];
    for (i, p) in c.control_points.iter().enumerate() {
        let mut q = [I::point(0.); 3];
        let mut numerator = I::point(0.);
        for k in 0..3 {
            q[k] = I::point(p[k]).sub(I::point(line[0][k]))?;
            numerator = numerator.add(q[k].mul(d[k])?)?;
        }
        let t = numerator.div(denominator)?;
        for k in 0..3 {
            let residual = q[k].sub(d[k].mul(t)?)?;
            hull[k] = if i == 0 {
                residual
            } else {
                I::new(hull[k].lo.min(residual.lo), hull[k].hi.max(residual.hi))?
            };
        }
    }
    norm_bounds(hull)
}
fn qualify(
    source: &Model,
    selected: usize,
    mut topology: TopologyTransition,
    requested: [f64; 2],
) -> Result<ChamferTransition> {
    let line = source.edges[selected]
        .vertices
        .map(|v| source.vertices[v].point);
    let original = &source.edges[selected].curve;
    let source_deviation = line_distance(original, line, &mut topology.work)?[1];
    if source_deviation > source.tolerance_mm / 8. {
        return Err(refused(
            "Whole original edge does not lie on the straight carrier within allowance",
        ));
    }
    let mut certificates = [Vec::new(), Vec::new()];
    for side in 0..2 {
        for &edge in &topology.rails[side] {
            let c = &topology.cap.replacement.model.edges[edge].curve;
            let distance = line_distance(c, line, &mut topology.work)?;
            let residual = I::new(distance[0], distance[1])?.sub(I::point(requested[side]))?;
            let error = I::point(residual.lo.abs().max(residual.hi.abs()))
                .add(I::point(source_deviation))?
                .hi;
            if error > source.tolerance_mm {
                return Err(refused(
                    "Actual saved contact curve fails the whole requested setback",
                ));
            }
            certificates[side].push(SetbackCertificate {
                original: original.clone(),
                actual: c.clone(),
                line,
                edge,
                requested: requested[side],
                distance,
                source_deviation,
                error,
            });
        }
    }
    Ok(ChamferTransition {
        topology,
        setbacks: certificates,
    })
}
/// Two-distance chamfer on a straight original edge between planar supports.
/// Distances are perpendicular to the original edge carrier, in each support
/// plane; their order is the ascending original support face index. Curved end
/// contacts are constructed, four original faces are trimmed, and actual saved
/// rail setbacks plus closed trims/sewing must qualify. No G1/radius is claimed
/// for sharp chamfer joins. Curved spines, nonplanar supports, higher-valence
/// corners, chart extension and global embedding require separate constructors.
pub fn chamfer_straight_edge(
    source: &Model,
    edge: usize,
    setbacks: [f64; 2],
    limits: Limits,
) -> Result<ChamferTransition> {
    source.validate()?;
    if setbacks
        .iter()
        .any(|d| !d.is_finite() || *d <= source.tolerance_mm)
        || !(256..=100000).contains(&limits.cells)
        || !(1..=1000000).contains(&limits.exact_work)
    {
        return Err(refused(
            "Choose two finite resolved setbacks and bounded chamfer work",
        ));
    }
    let frame = automatic::frame(source, edge, CrossSection::Chamfer(setbacks))?;
    let Boundaries {
        contours,
        curves,
        cells,
    } = automatic::boundaries(source, &frame, limits)?;
    let mut rest = limits;
    rest.cells = rest
        .cells
        .checked_sub(cells)
        .ok_or_else(|| refused("Chamfer contact allowance exhausted"))?;
    let mut topology = insert_four_faces(
        source,
        edge,
        &frame.surface,
        contours.each_ref(),
        Some(&curves),
        rest,
    )?;
    topology.work.cells += cells;
    topology.work.limits = limits;
    qualify(source, edge, topology, setbacks)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_hull_proof_refuses_matching_endpoints_with_wrong_interior_setback() {
        let mut c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.5, 0., 0.], vec![0.9, 0., 1.], vec![0.5, 0., 2.]],
            weights: vec![1., 0.5, 1.],
            periodic: false,
        };
        let mut work = Ledger {
            cells: 0,
            exact: 0,
            limits: Limits::default(),
        };
        let line = [[0., 0., 0.], [0., 0., 2.]];
        let bad = line_distance(&c, line, &mut work).unwrap();
        assert!(bad[0] <= 0.5 && bad[1] >= 0.9);
        c.control_points[1][0] = 0.5;
        let good = line_distance(&c, line, &mut work).unwrap();
        assert!(good[0] <= 0.5 && good[1] >= 0.5 && good[1] - good[0] < 1e-12);
    }
}
