//! source profile body operations: inspect_curved_edge_interval_policy.
use super::*;
use nurbs_core::{
    interval_eval::Interval as I,
    normal_alignment::{PairReport, ProfilePathReport},
};

/// Inclusive upper bound on the cell and span budgets of one path walk.
const MAX_PATH_WORK: usize = 100_000;

fn path_work_refusal() -> Error {
    Error::new("BREP_PROFILE_SEAM", "Bound original pcurve and normal work")
}

fn validate_path_work(
    canonical: [f64; 2],
    threshold: f64,
    max_cells: usize,
    max_spans: usize,
) -> Result<()> {
    if !canonical.iter().all(|x| x.is_finite())
        || canonical[0] < 0.
        || canonical[1] > 1.
        || canonical[0] >= canonical[1]
        || !(1..=MAX_PATH_WORK).contains(&max_cells)
        || !(1..=MAX_PATH_WORK).contains(&max_spans)
        || !threshold.is_finite()
        || !(0. ..1.).contains(&threshold)
    {
        return Err(path_work_refusal());
    }
    Ok(())
}

/// Outcome of inspecting one pending parameter cell.
enum CellStep {
    Accepted,
    Split(f64),
    Stop,
}

/// Bisecting walk over one edge interval, certifying the tangent planes of
/// both incident faces cell by cell.
struct PathWalk<'e, 'p, 'a> {
    edge: &'e crate::source_shared_edge::SharedEdge,
    order: [usize; 2],
    profile: &'p WeightProfile<'a>,
    neighbor: Option<&'p WeightProfile<'a>>,
    pole: Option<bool>,
    threshold: f64,
    max_cells: usize,
    max_spans: usize,
}

impl PathWalk<'_, '_, '_> {
    fn run(&self, mut out: ProfilePathReport) -> Result<ProfilePathReport> {
        // Singular literal source endpoints are explicitly inspected before
        // repeatedly subdividing an interval that contains a zero normal.
        let canonical = out.pending[0];
        out.pending.push([canonical[1]; 2]);
        out.pending.push([canonical[0]; 2]);
        out.reason = "profile-path-work-limit";
        let mut upper = 0_f64;
        while let Some(q) = out.pending.pop() {
            match self.step(&mut out, &mut upper, q)? {
                CellStep::Accepted => {}
                CellStep::Split(mid) => {
                    out.pending.push([mid, q[1]]);
                    out.pending.push([q[0], mid]);
                }
                CellStep::Stop => {
                    out.pending.push(q);
                    return Ok(out);
                }
            }
        }
        out.aligned = Some(true);
        out.upper_sine_squared = Some(upper);
        out.reason = if self.pole.is_some() {
            "profile-edge-factored-limit-angles-qualified"
        } else {
            "profile-path-tangent-planes-qualified"
        };
        Ok(out)
    }

    fn step(&self, out: &mut ProfilePathReport, upper: &mut f64, q: [f64; 2]) -> Result<CellStep> {
        if out.cells == self.max_cells || out.spans == self.max_spans {
            return Ok(CellStep::Stop);
        }
        out.cells += 1;
        let Some(rectangles) = self.rectangles(out, q)? else {
            return Ok(CellStep::Stop);
        };
        if out.spans == self.max_spans {
            return Ok(CellStep::Stop);
        }
        let r = self.normals(rectangles, self.max_spans - out.spans)?;
        out.spans += r.spans;
        if r.aligned == Some(true) {
            let interval = r.sine_squared_interval.ok_or_else(path_work_refusal)?;
            *upper = upper.max(interval[1]);
            out.accepted_cells += 1;
            return Ok(CellStep::Accepted);
        }
        if r.aligned == Some(false) {
            out.aligned = Some(false);
            out.reason = "profile-path-angular-mismatch";
            return Ok(CellStep::Stop);
        }
        if q[0] == q[1] {
            out.reason = "profile-seam-endpoint-normal-unresolved";
            return Ok(CellStep::Stop);
        }
        if r.reason == "span-limit" {
            return Ok(CellStep::Stop);
        }
        let mid = q[0] + (q[1] - q[0]) * 0.5;
        if mid <= q[0] || mid >= q[1] {
            out.reason = "profile-path-resolution-limit";
            return Ok(CellStep::Stop);
        }
        Ok(CellStep::Split(mid))
    }

    /// Parameter rectangles of both pcurves over the cell, clipped to each
    /// surface's natural domain; `None` when the span budget is exhausted.
    fn rectangles(
        &self,
        out: &mut ProfilePathReport,
        q: [f64; 2],
    ) -> Result<Option<[[[f64; 2]; 2]; 2]>> {
        let mut rectangles = [[[0.; 2]; 2]; 2];
        for (side, rectangle) in rectangles.iter_mut().enumerate() {
            let use_ = &self.edge.uses()[self.order[side]];
            let c = use_.curve();
            let t = crate::source_seam_tangency::inverse(
                self.edge,
                self.order[side],
                I::new(q[0], q[1])?,
            )?;
            let spent = (c.degree..c.control_points.len())
                .filter(|&i| {
                    c.knots[i] < c.knots[i + 1] && c.knots[i] <= t.hi && c.knots[i + 1] >= t.lo
                })
                .count();
            if spent > self.max_spans - out.spans {
                return Ok(None);
            }
            out.spans += spent;
            let uv = nurbs_core::interval_eval::evaluate_interval(c, t)?;
            let s = use_.surface();
            let natural = [
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            ];
            for (axis, bounds) in rectangle.iter_mut().enumerate() {
                let r = uv[axis].intersect(natural[axis][0], natural[axis][1])?;
                *bounds = [r.lo, r.hi];
            }
        }
        Ok(Some(rectangles))
    }

    fn normals(&self, rectangles: [[[f64; 2]; 2]; 2], budget: usize) -> Result<PairReport> {
        let original = self.edge.uses()[self.order[1]].surface();
        match (self.pole, self.neighbor) {
            (Some(high), _) => self.profile.inspect_original_factored_pole_normals(
                original,
                rectangles,
                high,
                self.threshold,
                budget,
            ),
            (None, Some(other)) => {
                self.profile
                    .inspect_profile_normals(other, rectangles, self.threshold, budget)
            }
            (None, None) => {
                self.profile
                    .inspect_original_normals(original, rectangles, self.threshold, budget)
            }
        }
    }
}
impl<'a> Proposal<'a> {
pub(super) fn inspect_curved_edge_interval_policy(
        &self,
        edge: EdgeInterval,
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
        pole: Option<bool>,
        original_neighbor: bool,
    ) -> Result<ProfilePathReport> {
        let EdgeInterval {
            face,
            neighbor,
            index,
            canonical,
        } = edge;
        validate_path_work(canonical, threshold, max_cells, max_spans)?;
        let shell = self.body.geometry().shell();
        let edge = &shell.edges()[index];
        let addresses = &shell.uses()[index];
        let order = if addresses[0].face == face {
            [0, 1]
        } else {
            [1, 0]
        };
        let out = ProfilePathReport {
            aligned: None,
            cells: 0,
            spans: 0,
            accepted_cells: 0,
            upper_sine_squared: None,
            pending: vec![canonical],
            reason: "profile-seam-carrier-unproven",
        };
        let regions = shell.regions().ok_or_else(path_work_refusal)?;
        if (!edge.maps_complete_canonical_source()
            && !edge.covers_retained_canonical_interval(canonical)?)
            || [face, neighbor].iter().enumerate().any(|(side, &f)| {
                !same(
                    edge.uses()[order[side]].surface(),
                    regions[f].loops()[0][0].surface(),
                ) || edge.uses()[order[side]].curve().periodic
            })
        {
            return Ok(out);
        }
        let walk = PathWalk {
            edge,
            order,
            profile: self.profile(face).ok_or_else(path_work_refusal)?,
            neighbor: if original_neighbor {
                None
            } else {
                self.profile(neighbor)
            },
            pole,
            threshold,
            max_cells,
            max_spans,
        };
        walk.run(out)
    }
pub fn original_body(&self) -> &'a Body {
        self.body
    }
pub fn profile(&self, face: usize) -> Option<&WeightProfile<'a>> {
        self.faces
            .iter()
            .find(|f| f.spec.face == face)
            .map(|f| &f.profile)
    }
pub fn specs(&self) -> Vec<FaceSpec> {
        self.faces.iter().map(|f| f.spec).collect()
    }
pub fn axis_deviation_upper(&self) -> [f64; 3] {
        self.axis_deviation_upper
    }
/// Persist source and exact factor recipes, never flattened target weights or
    /// claimed geometry certificates. Restoration must rerun all admission gates.
    #[cfg(feature = "codec")]
    pub fn definition(&self) -> Result<value_codec::Value> {
        Ok(
            value_codec::json!({"version":1,"kind":"boundary-preserving-weight-profiles",
            "originalBody":self.body.definition()?,"positionTolerance":self.position_tolerance,
            "profiles":self.faces.iter().map(|f|value_codec::json!({"face":f.spec.face,"anchor":f.spec.anchor})).collect::<Vec<_>>()}),
        )
    }
}
