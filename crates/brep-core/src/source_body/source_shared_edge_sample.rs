//! One canonical interior point for both uses of a privately admitted edge.
//! Root caps remain expressions; their uncertain ends are never rounded here.
use super::SharedEdge;
use crate::source_seam_tangency::Carrier;
use nurbs_core::{
    Error, Result,
    interval_eval::{self, Interval as I},
};

pub struct BoundarySample<'a,E = SharedEdge> {
    edge: &'a E,
    canonical: f64,
    parameters: [[f64; 2]; 2],
    uv: [[[f64; 2]; 2]; 2],
    world: [[f64; 2]; 3],
}
impl<E> BoundarySample<'_,E> {
    pub fn edge(&self) -> &E {
        self.edge
    }
    pub fn canonical(&self) -> f64 {
        self.canonical
    }
    pub fn parameter_bounds(&self) -> [[f64; 2]; 2] {
        self.parameters
    }
    pub fn uv_bounds(&self) -> [[[f64; 2]; 2]; 2] {
        self.uv
    }
    pub fn world_bounds(&self) -> [[f64; 2]; 3] {
        self.world
    }
}
pub struct BoundarySampleReport<'a,E = SharedEdge> {
    pub sample: Option<BoundarySample<'a,E>>,
    pub spans: usize,
    pub reason: &'static str,
}
pub struct BoundaryChainPoint<'a,E = SharedEdge> {
    pub canonical: f64,
    pub report: BoundarySampleReport<'a,E>,
}
/// A requested discrete partition, not a whole-edge error certificate.
pub struct BoundaryChainReport<'a,E = SharedEdge> {
    pub points: Vec<BoundaryChainPoint<'a,E>>,
    pub spans: usize,
    pub pending: usize,
}
impl SharedEdge {
    pub fn sample_interior_chain(&self,canonical:&[f64],max_spans:usize)->Result<BoundaryChainReport<'_>> {
        sample_interior_chain(self,canonical,max_spans)
    }
    pub fn sample_interior(&self,canonical:f64,max_spans:usize)->Result<BoundarySampleReport<'_>> {
        sample_interior(self,canonical,max_spans)
    }
}

    /// Preserve every requested point, including points refused by the retained
    /// domain or the shared work budget. Both incident faces consume this same
    /// ordered list; they must not independently round or resample its points.
pub(crate) fn sample_interior_chain<'a,E:Carrier>(
        edge:&'a E,
        canonical: &[f64],
        max_spans: usize,
    ) -> Result<BoundaryChainReport<'a,E>> {
        if canonical.len() > 100000
            || max_spans > 100000
            || canonical
                .iter()
                .any(|t| !t.is_finite() || !(0.0..=1.0).contains(t))
            || canonical.windows(2).any(|p| p[0] >= p[1])
        {
            return Err(Error::new(
                "BREP_SHARED_SAMPLE",
                "Choose strictly increasing canonical parameters and bounded work",
            ));
        }
        let mut chain = BoundaryChainReport {
            points: Vec::with_capacity(canonical.len()),
            spans: 0,
            pending: 0,
        };
        for &canonical in canonical {
            let report = sample_interior(edge,canonical, max_spans - chain.spans)?;
            chain.spans += report.spans;
            chain.pending += usize::from(report.sample.is_none());
            chain.points.push(BoundaryChainPoint { canonical, report });
        }
        Ok(chain)
    }
    /// Canonical coordinates are normalized to the original world curve.
    /// Admission requires the entire inverse enclosure to lie strictly inside
    /// both retained uses. No proximity-based vertex welding is performed.
pub(crate) fn sample_interior<'a,E:Carrier>(
        edge:&'a E,
        canonical: f64,
        max_spans: usize,
    ) -> Result<BoundarySampleReport<'a,E>> {
        if !canonical.is_finite() || !(0.0..=1.0).contains(&canonical) || max_spans > 100000 {
            return Err(Error::new(
                "BREP_SHARED_SAMPLE",
                "Choose a finite canonical parameter and bounded span work",
            ));
        }
        let mut report = BoundarySampleReport {
            sample: None,
            spans: 0,
            reason: "boundary-sample-source-map-unproven",
        };
        let mut parameters = [[0.; 2]; 2];
        for side in 0..2 {
            // Exact admitted parameter maps may cover only part of the canonical
            // source. An inverse outside an original chart is an unresolved
            // sample, not permission to extrapolate or clamp a retained end.
            let Ok(t) = edge.inverse_parameter(side, I::point(canonical)) else {
                report.reason = "boundary-sample-retained-interior-unproven";
                return Ok(report);
            };
            let ends = edge.fragment(side).parameter_bounds();
            if t.lo <= ends[0][1].min(ends[1][1]) || t.hi >= ends[0][0].max(ends[1][0]) {
                report.reason = "boundary-sample-retained-interior-unproven";
                return Ok(report);
            }
            parameters[side] = [t.lo, t.hi];
        }
        let d = edge.world().domain();
        let t = I::point(d[0])
            .add(I::point(canonical).mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
            .intersect(d[0], d[1])?;
        let queries = [
            (
                edge.fragment(0).curve(),
                I::new(parameters[0][0], parameters[0][1])?,
            ),
            (
                edge.fragment(1).curve(),
                I::new(parameters[1][0], parameters[1][1])?,
            ),
            (edge.world(), t),
        ];
        let mut values = Vec::new();
        for (curve, t) in queries {
            let spent = (curve.degree..curve.control_points.len())
                .filter(|&i| {
                    curve.knots[i] < curve.knots[i + 1]
                        && curve.knots[i] <= t.hi
                        && curve.knots[i + 1] >= t.lo
                })
                .count();
            if spent > max_spans - report.spans {
                report.reason = "boundary-sample-work-limit";
                return Ok(report);
            }
            report.spans += spent;
            values.push(interval_eval::evaluate_interval(curve, t)?);
        }
        report.sample = Some(BoundarySample {
            edge,
            canonical,
            parameters,
            uv: std::array::from_fn(|side| {
                std::array::from_fn(|axis| [values[side][axis].lo, values[side][axis].hi])
            }),
            world: std::array::from_fn(|axis| [values[2][axis].lo, values[2][axis].hi]),
        });
        report.reason = "boundary-sample-qualified";
        Ok(report)
    }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_boundary_fragment::{Endpoint, Fragment};
    use nurbs_core::{curve::Curve, surface::Surface};
    #[test]
    fn canonical_sample_owns_both_reversed_uses_and_refuses_caps_or_exhaustion() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let uv = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let world = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let a = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let b = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(1.),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let qualified = super::super::qualify(&world, [&a, &b], [false, false], 100000).unwrap();
        let edge = qualified.edge.expect("exact shared line");
        let report = edge.sample_interior(0.375, 3).unwrap();
        assert_eq!(report.spans, 3);
        let sample = report.sample.unwrap();
        assert!(std::ptr::eq(sample.edge(), &edge));
        for t in sample.parameter_bounds() {
            assert!(t[0] <= 0.375 && t[1] >= 0.375);
        }
        let bounds = sample.world_bounds();
        assert!(bounds[0][0] <= 0.375 && bounds[0][1] >= 0.375);
        assert!(edge.sample_interior(0., 3).unwrap().sample.is_none());
        assert!(edge.sample_interior(1., 3).unwrap().sample.is_none());
        assert!(edge.sample_interior(0.375, 2).unwrap().sample.is_none());
        assert!(edge.sample_interior(f64::NAN, 3).is_err());
        let a = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(0.25),
            Endpoint::Parameter(0.75),
        )
        .unwrap();
        let b = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(0.75),
            Endpoint::Parameter(0.25),
        )
        .unwrap();
        let trimmed = super::super::qualify(&world, [&a, &b], [false, false], 100000).unwrap();
        let trimmed = trimmed.edge.expect("exact restricted shared line");
        for end in 0..2 {
            let endpoint = trimmed.sample_endpoint(end, 3).unwrap().endpoint.unwrap();
            assert!(std::ptr::eq(endpoint.edge(), &trimmed));
            assert_eq!(endpoint.ends(), [end, 1 - end]);
            let value = if end == 0 { 0.25 } else { 0.75 };
            let bounds = endpoint.world_bounds();
            assert!(bounds[0][0] <= value && value <= bounds[0][1]);
        }
        let limits = |cells, spans| super::super::BoundaryMeshLimits {
            max_deviation_mm: 1e-6, cells, spans, max_depth: 16,
        };
        let mesh = trimmed.prepare_boundary_mesh(limits(100, 100)).unwrap();
        assert!(mesh.pending.is_empty());
        assert_eq!(mesh.segments.len(), 1);
        assert_eq!(mesh.vertices.len(), 2);
        assert_eq!(mesh.segments[0].vertices, [0, 1]);
        assert!(mesh.segments[0].deviation_upper_mm <= 1e-6);
        let no_cells = trimmed.prepare_boundary_mesh(limits(0, 100)).unwrap();
        assert_eq!(no_cells.pending.len(), 1);
        assert_eq!(no_cells.pending[0].vertices, Some([0, 1]));
        assert_eq!(no_cells.cells, 0);
        let no_spans = trimmed.prepare_boundary_mesh(limits(100, 0)).unwrap();
        assert_eq!(no_spans.pending.len(), 1);
        assert!(no_spans.vertices.is_empty());
        assert_eq!(no_spans.spans, 0);
        assert!(trimmed.sample_endpoint(0, 2).unwrap().endpoint.is_none());
        assert!(trimmed.sample_endpoint(2, 3).is_err());
        assert!(trimmed.sample_interior(0.5, 3).unwrap().sample.is_some());
        for t in [0.1, 0.25, 0.75, 0.9] {
            assert!(trimmed.sample_interior(t, 3).unwrap().sample.is_none());
        }
        let chain = trimmed
            .sample_interior_chain(&[0.25, 0.375, 0.5, 0.625, 0.75], 6)
            .unwrap();
        assert_eq!(chain.points.len(), 5);
        assert_eq!(chain.spans, 6);
        assert_eq!(chain.pending, 3);
        assert_eq!(
            chain
                .points
                .iter()
                .filter_map(|p| p.report.sample.as_ref().map(|s| s.canonical()))
                .collect::<Vec<_>>(),
            vec![0.375, 0.5]
        );
        let empty_work = trimmed
            .sample_interior_chain(&[0.375, 0.5, 0.625], 0)
            .unwrap();
        assert_eq!(empty_work.points.len(), 3);
        assert_eq!(empty_work.pending, 3);
        assert_eq!(empty_work.spans, 0);
        for parameters in [vec![0.5, 0.5], vec![0.6, 0.4], vec![f64::NAN], vec![1.1]] {
            assert!(trimmed.sample_interior_chain(&parameters, 100).is_err());
        }
    }
}
