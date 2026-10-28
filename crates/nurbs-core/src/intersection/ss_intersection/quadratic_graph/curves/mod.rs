//! Compact saved NURBS proxies with global original-branch coefficient bounds.
//! Approximation, original-face pullback agreement and topology are distinct.
use super::*;
use crate::curve::Curve;
mod arithmetic;
mod boundary_roots;
pub use boundary_roots::{
    GraphBoundaryRoot, GraphBoundaryRootLimits, QualifiedGraphBoundaryRoots,
    qualify_graph_boundary_roots,
};
mod expressions;
mod proof;
use arithmetic::{Arithmetic, Rational};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphCurveTolerances {
    pub world: f64,
    pub uv: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct GraphCurveBuildLimits {
    pub max_operations: usize,
}
impl Default for GraphCurveBuildLimits {
    fn default() -> Self {
        Self {
            max_operations: 200000,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct GraphCurveVertex {
    pub original_vertex: usize,
    pub world: [f64; 3],
    pub native_uv: [[f64; 2]; 2],
    pub world_error_upper: f64,
    pub uv_error_upper: [f64; 2],
    pub lift_agreement_upper: [f64; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdgeCurves {
    pub original_edge: usize,
    pub world: Curve,
    pub pcurves: [Curve; 2],
    pub world_error_upper: f64,
    pub uv_error_upper: [f64; 2],
    pub lift_agreement_upper: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct QualifiedGraphCurves {
    original: QualifiedQuadraticGraphSS,
    tolerance: GraphCurveTolerances,
    limits: GraphCurveBuildLimits,
    vertices: Vec<GraphCurveVertex>,
    edges: Vec<GraphEdgeCurves>,
    operations: usize,
}
impl QualifiedGraphCurves {
    pub fn original(&self) -> &QualifiedQuadraticGraphSS {
        &self.original
    }
    pub fn vertices(&self) -> &[GraphCurveVertex] {
        &self.vertices
    }
    pub fn edges(&self) -> &[GraphEdgeCurves] {
        &self.edges
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    /// Rebuild from original supports, prove actual saved poles/weights, then
    /// match this snapshot. A mutable public report cannot create this owner.
    pub fn reverify(&self) -> Result<()> {
        self.original.reverify()?;
        let fresh = construct_graph_curves(&self.original, self.tolerance, self.limits)?;
        check(
            self.vertices == fresh.vertices
                && self.edges == fresh.edges
                && self.operations == fresh.operations,
            "Original SS NURBS proxy ownership changed",
        )
    }
}
fn proposal(r: &Rational, b: &mut Arithmetic) -> Result<Curve> {
    let mut weights = Vec::new();
    let mut control_points = Vec::new();
    for i in 0..r.denominator.len() {
        let weight = b.midpoint(r.denominator[i])?;
        check(weight > 0., "SS curve proposed nonpositive weight")?;
        let mut point = Vec::new();
        for axis in 0..r.numerator.len() {
            let numerator = b.midpoint(r.numerator[axis][i])?;
            b.tick(1)?;
            point.push(numerator / weight);
        }
        weights.push(weight);
        control_points.push(point);
    }
    let n = r.degree();
    let curve = Curve {
        degree: n,
        knots: vec![0.; n + 1].into_iter().chain(vec![1.; n + 1]).collect(),
        control_points,
        weights,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}
/// One shared operation allowance covers original vertex enclosures, coefficient
/// construction, actual saved-curve error and both original-face pullbacks.
/// Returned proxies do not authorize imprinting, embedding, sewing or a Solid.
pub fn construct_graph_curves(
    original: &QualifiedQuadraticGraphSS,
    tolerance: GraphCurveTolerances,
    limits: GraphCurveBuildLimits,
) -> Result<QualifiedGraphCurves> {
    check(
        tolerance.world.is_finite()
            && tolerance.world > 0.
            && tolerance.uv.is_finite()
            && tolerance.uv > 0.,
        "Invalid original SS curve tolerance",
    )?;
    check(
        (1..=1000000).contains(&limits.max_operations),
        "Invalid original SS curve work allowance",
    )?;
    let mut b = Arithmetic {
        used: 0,
        max: limits.max_operations,
    };
    b.tick(1)?;
    let mut vertices = Vec::new();
    for i in 0..original.vertices().len() {
        let available = (b.max - b.used).min(4096);
        if available == 0 {
            return Err(crate::resource("Original SS curve vertex work exhausted"));
        }
        let v = original.vertex_station(i, available)?;
        b.tick(v.operations)?;
        let mut world = [0.; 3];
        let mut native_uv = [[0.; 2]; 2];
        for axis in 0..3 {
            world[axis] = b.midpoint(I::new(v.world[axis][0], v.world[axis][1])?)?;
        }
        for (side, uv) in [v.first_uv, v.second_uv].into_iter().enumerate() {
            for axis in 0..2 {
                let d = original.data.domains[side][axis];
                native_uv[side][axis] = b
                    .midpoint(I::new(uv[axis][0], uv[axis][1])?)?
                    .clamp(d[0], d[1]);
            }
        }
        if let OriginalGraphSSPoint::Normalized(p) = &original.vertices()[i].point {
            // These are exact owned rectangle incidences, not a proximity snap.
            // Retain their original native endpoints and original world rails.
            for axis in 0..2 {
                if p[axis] == 0. || p[axis] == 1. {
                    let end = usize::from(p[axis] == 1.);
                    for side in 0..2 {
                        native_uv[side][axis] = original.data.domains[side][axis][end];
                    }
                    let source = original.source().first();
                    world[axis] = if axis == 0 {
                        source.control_points[end * source.degree_u][0][0]
                    } else {
                        source.control_points[0][end * source.degree_v][1]
                    };
                }
            }
            if p.iter().all(|q| *q == 0. || *q == 1.) {
                let source = original.source().first();
                world[2] = source.control_points[usize::from(p[0] == 1.) * source.degree_u]
                    [usize::from(p[1] == 1.) * source.degree_v][2];
            }
        }
        if let Some(height) = expressions::constant_height(original) {
            world[2] = height;
        }
        let world_error_upper = proof::vertex_error(&v.world, &world, &mut b)?;
        let uv_error_upper = [
            proof::vertex_error(&v.first_uv, &native_uv[0], &mut b)?,
            proof::vertex_error(&v.second_uv, &native_uv[1], &mut b)?,
        ];
        let lift_agreement_upper = proof::vertex_lifts(original, world, native_uv, &mut b)?;
        check(
            world_error_upper <= tolerance.world
                && uv_error_upper.iter().all(|e| *e <= tolerance.uv)
                && lift_agreement_upper.iter().all(|e| *e <= tolerance.world),
            "Original SS vertex deviation tolerance is unproven",
        )?;
        vertices.push(GraphCurveVertex {
            original_vertex: i,
            world,
            native_uv,
            world_error_upper,
            uv_error_upper,
            lift_agreement_upper,
        });
    }
    let mut edges = Vec::new();
    for i in 0..original.edges().len() {
        let uv = expressions::normalized(original, i, &mut b)?;
        let world_expression = expressions::world(
            original,
            &uv,
            expressions::preferred_height(original),
            &mut b,
        )?;
        let mut world = proposal(&world_expression, &mut b)?;
        if let Some(height) = expressions::constant_height(original) {
            for p in &mut world.control_points {
                p[2] = height;
            }
        }
        let mut pcurves = Vec::new();
        let mut uv_error_upper = [0.; 2];
        let mut lift_agreement_upper = [0.; 2];
        let ids = original.edges()[i].vertices;
        let last = world.control_points.len() - 1;
        for (end, pole) in [0, last].into_iter().enumerate() {
            world.control_points[pole] = vertices[ids[end]].world.to_vec();
        }
        let world_error_upper = proof::deviation(&world_expression, &world, &mut b)?;
        check(
            world_error_upper <= tolerance.world,
            "Original SS world-curve deviation tolerance is unproven",
        )?;
        for side in 0..2 {
            let expression = expressions::native(original, &uv, side, &mut b)?;
            let mut curve = proposal(&expression, &mut b)?;
            let last = curve.control_points.len() - 1;
            for (end, pole) in [0, last].into_iter().enumerate() {
                curve.control_points[pole] = vertices[ids[end]].native_uv[side].to_vec();
            }
            proof::domain(&curve, original.data.domains[side], &mut b)?;
            uv_error_upper[side] = proof::deviation(&expression, &curve, &mut b)?;
            let pullback = expressions::pullback(original, &curve, side, &mut b)?;
            lift_agreement_upper[side] = proof::deviation(&pullback, &world, &mut b)?;
            check(
                uv_error_upper[side] <= tolerance.uv
                    && lift_agreement_upper[side] <= tolerance.world,
                "Original SS pcurve or original-face lift agreement tolerance is unproven",
            )?;
            pcurves.push(curve);
        }
        edges.push(GraphEdgeCurves {
            original_edge: i,
            world,
            pcurves: pcurves.try_into().unwrap(),
            world_error_upper,
            uv_error_upper,
            lift_agreement_upper,
        });
    }
    Ok(QualifiedGraphCurves {
        original: original.clone(),
        tolerance,
        limits,
        vertices,
        edges,
        operations: b.used,
    })
}
#[cfg(test)]
mod replay_tests {
    use super::*;
    fn original() -> QualifiedQuadraticGraphSS {
        let surface = |offset: bool| Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            let u = i as f64 / 2.;
                            let v = j as f64 / 2.;
                            let q = [0.25, -0.25, 0.25];
                            vec![u, v, u * v + if offset { q[i] + q[j] - 0.0625 } else { 0. }]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        qualify_quadratic_graph_ss(
            &FiberSources::new(&surface(true), &surface(false)).unwrap(),
            Default::default(),
        )
        .unwrap()
    }
    #[test]
    fn actual_poles_weights_reports_and_rebound_sources_cannot_reuse_proxy_admission() {
        let original = original();
        let tolerance = GraphCurveTolerances {
            world: 1e-7,
            uv: 1e-9,
        };
        let owner = construct_graph_curves(&original, tolerance, Default::default()).unwrap();
        let mut altered = owner.clone();
        altered.edges[0].world.control_points[1][0] += 0.01;
        altered.edges[0].world_error_upper = 0.;
        assert!(altered.reverify().is_err());
        let mut altered = owner.clone();
        altered.edges[0].world.weights[1] *= 2.;
        let mut b = Arithmetic {
            used: 0,
            max: 200000,
        };
        let uv = expressions::normalized(&original, 0, &mut b).unwrap();
        let world = expressions::world(&original, &uv, 0, &mut b).unwrap();
        assert!(
            proof::deviation(&world, &altered.edges[0].world, &mut b).unwrap() > tolerance.world
        );
        assert!(altered.reverify().is_err());
        let mut altered = owner.clone();
        altered.edges[0].pcurves[1].control_points[1][1] += 0.01;
        assert!(altered.reverify().is_err());
        let mut changed_clock = owner.edges[0].world.clone();
        for k in &mut changed_clock.knots {
            *k += 1.;
        }
        let mut work = Arithmetic {
            used: 0,
            max: 200000,
        };
        assert!(proof::deviation(&world, &changed_clock, &mut work).is_err());
        let mut a = original.source().first().clone();
        let mut b = original.source().second().clone();
        for s in [&mut a, &mut b] {
            for row in &mut s.control_points {
                for p in row {
                    p[2] += 1.;
                }
            }
        }
        let rebound =
            qualify_quadratic_graph_ss(&FiberSources::new(&a, &b).unwrap(), Default::default())
                .unwrap();
        let mut altered = owner;
        altered.original = rebound;
        assert!(altered.reverify().is_err());
    }
}
