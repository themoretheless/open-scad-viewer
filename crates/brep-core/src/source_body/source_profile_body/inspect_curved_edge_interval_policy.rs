//! source profile body operations: inspect_curved_edge_interval_policy.
use super::*;
impl<'a> Proposal<'a> {
pub(super) fn inspect_curved_edge_interval_policy(
        &self,
        face: usize,
        neighbor: usize,
        index: usize,
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
        canonical: [f64; 2],
        pole: Option<bool>,
        original_neighbor: bool,
    ) -> Result<nurbs_core::normal_alignment::ProfilePathReport> {
        use nurbs_core::{interval_eval::Interval as I, normal_alignment::ProfilePathReport};
        if !canonical.iter().all(|x| x.is_finite())
            || canonical[0] < 0.
            || canonical[1] > 1.
            || canonical[0] >= canonical[1]
            || !(1..=100000).contains(&max_cells)
            || !(1..=100000).contains(&max_spans)
            || !threshold.is_finite()
            || !(0. ..1.).contains(&threshold)
        {
            return Err(Error::new(
                "BREP_PROFILE_SEAM",
                "Bound original pcurve and normal work",
            ));
        }
        let shell = self.body.geometry().shell();
        let edge = &shell.edges()[index];
        let addresses = &shell.uses()[index];
        let order = if addresses[0].face == face {
            [0, 1]
        } else {
            [1, 0]
        };
        let mut out = ProfilePathReport {
            aligned: None,
            cells: 0,
            spans: 0,
            accepted_cells: 0,
            upper_sine_squared: None,
            pending: vec![canonical],
            reason: "profile-seam-carrier-unproven",
        };
        let regions = shell.regions().unwrap();
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
        let profile = self.profile(face).unwrap();
        // Singular literal source endpoints are explicitly inspected before
        // repeatedly subdividing an interval that contains a zero normal.
        out.pending.push([canonical[1]; 2]);
        out.pending.push([canonical[0]; 2]);
        out.reason = "profile-path-work-limit";
        let mut upper = 0_f64;
        while let Some(q) = out.pending.pop() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(q);
                return Ok(out);
            }
            out.cells += 1;
            let mut rectangles = [[[0.; 2]; 2]; 2];
            for side in 0..2 {
                let use_ = &edge.uses()[order[side]];
                let c = use_.curve();
                let t =
                    crate::source_seam_tangency::inverse(edge, order[side], I::new(q[0], q[1])?)?;
                let spent = (c.degree..c.control_points.len())
                    .filter(|&i| {
                        c.knots[i] < c.knots[i + 1] && c.knots[i] <= t.hi && c.knots[i + 1] >= t.lo
                    })
                    .count();
                if spent > max_spans - out.spans {
                    out.pending.push(q);
                    return Ok(out);
                }
                out.spans += spent;
                let uv = nurbs_core::interval_eval::evaluate_interval(c, t)?;
                let s = use_.surface();
                let natural = [
                    [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                    [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
                ];
                for axis in 0..2 {
                    let r = uv[axis].intersect(natural[axis][0], natural[axis][1])?;
                    rectangles[side][axis] = [r.lo, r.hi];
                }
            }
            if out.spans == max_spans {
                out.pending.push(q);
                return Ok(out);
            }
            let r = if let Some(high) = pole {
                profile.inspect_original_factored_pole_normals(
                    edge.uses()[order[1]].surface(),
                    rectangles,
                    high,
                    threshold,
                    max_spans - out.spans,
                )?
            } else {
                match if original_neighbor {
                    None
                } else {
                    self.profile(neighbor)
                } {
                    Some(other) => profile.inspect_profile_normals(
                        other,
                        rectangles,
                        threshold,
                        max_spans - out.spans,
                    )?,
                    None => profile.inspect_original_normals(
                        edge.uses()[order[1]].surface(),
                        rectangles,
                        threshold,
                        max_spans - out.spans,
                    )?,
                }
            };
            out.spans += r.spans;
            if r.aligned == Some(true) {
                upper = upper.max(r.sine_squared_interval.unwrap()[1]);
                out.accepted_cells += 1;
                continue;
            }
            if r.aligned == Some(false) {
                out.pending.push(q);
                out.aligned = Some(false);
                out.reason = "profile-path-angular-mismatch";
                return Ok(out);
            }
            if q[0] == q[1] {
                out.pending.push(q);
                out.reason = "profile-seam-endpoint-normal-unresolved";
                return Ok(out);
            }
            if r.reason == "span-limit" {
                out.pending.push(q);
                return Ok(out);
            }
            let mid = q[0] + (q[1] - q[0]) * 0.5;
            if mid <= q[0] || mid >= q[1] {
                out.pending.push(q);
                out.reason = "profile-path-resolution-limit";
                return Ok(out);
            }
            out.pending.push([mid, q[1]]);
            out.pending.push([q[0], mid]);
        }
        out.aligned = Some(true);
        out.upper_sine_squared = Some(upper);
        out.reason = if pole.is_some() {
            "profile-edge-factored-limit-angles-qualified"
        } else {
            "profile-path-tangent-planes-qualified"
        };
        Ok(out)
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
