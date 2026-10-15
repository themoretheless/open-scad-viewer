//! Original filled-carrier homotopy, before section approximation or rounding.
//!
//! A SINGLE constant projection P must make sym(P DF) uniformly positive
//! definite on the entire convex UV x traversal box. Integrating along the
//! segment between any two parameters proves strong monotonicity, hence global
//! injectivity and nonsingular one-sided charts. Restriction to ANY valid trim
//! region preserves its topology, including holes. Trim validity/ownership is
//! deliberately a separate BRep obligation. Local Jacobian signs alone do not
//! supply this theorem, nor can different projections be used in different cells.
use super::*;
use crate::sweeps::progressive_miter::{
    authored_frame_certificate as frame, scalar_certificate::Status,
};
use crate::{distance_bounds::Interval as I, numerics::interval_vec3 as iv};
type Box3 = [[f64; 2]; 3];
type V3 = [I; 3];

#[derive(Clone, Debug)]
pub struct IdealVolumeReport {
    pub embedding_certified: bool,
    pub regularity_certified: bool,
    /// Relative to the declared UV trim region, whose validity is not inferred.
    /// The traversal factor is [0,1] for open sources, S1 for proved quotients.
    pub parameter_topology_preserved: bool,
    pub cells: usize,
    pub exact_work: u64,
    pub boxes: usize,
    pub monotonicity_lower: Option<f64>,
    pub projection: [[f64; 3]; 3],
    pub reason: Option<&'static str>,
}
mod arc;
mod arc_frames;
mod atlas;
mod cell;
mod charts;
mod closed;
mod pose;
use charts::*;
impl MultiSweep<'_> {
    /// Certify the ORIGINAL filled volume F(u,v,t)=pose(t)[(1-t) C0+t C1].
    /// Both cap carriers use normalized UV; the BRep owner must prove that their
    /// trims describe the SAME valid region and bind every boundary to profiles.
    /// Closed paths require quotient charts and are intentionally not admitted
    /// by a convex-box theorem. An exhausted budget never returns partial credit.
    pub fn certify_ideal_volume(
        &self,
        caps: [&Surface; 2],
        projection: [[f64; 3]; 3],
        max_cells: usize,
        max_exact_work: u64,
    ) -> Result<IdealVolumeReport> {
        self.certify_ideal_volume_impl(caps, projection, max_cells, max_exact_work, false)
    }
    /// Exhaustive original-map pair cover. Each pair is either strictly
    /// separated in world space or belongs to ONE convex parameter hull whose
    /// entire interval Jacobian matrix is regular, or whose full subdivision
    /// shares ONE proved strong-monotonicity metric. This is a sufficient global
    /// embedding theorem even when no single strong-monotonicity metric exists.
    /// All pair visits, failed trials and source restrictions share the ledger.
    /// `monotonicity_lower` stays None; this method does not prove monotonicity.
    /// Closed traversals require exact native endpoint ownership and ONE
    /// literal traversal-independent carrier. Wrapped charts then certify the
    /// UV x S1 quotient, not injectivity of the duplicated endpoint cube.
    /// Invalid/unowned trim regions are not admitted.
    /// The explicit aggregate budget may be up to 1,000,000 cells. Individual
    /// source stages retain their 100,000-cell ceiling; no implicit work is added.
    pub fn certify_ideal_volume_atlas(
        &self,
        caps: [&Surface; 2],
        projection: [[f64; 3]; 3],
        max_cells: usize,
        max_exact_work: u64,
    ) -> Result<IdealVolumeReport> {
        self.certify_ideal_volume_impl(caps, projection, max_cells, max_exact_work, true)
    }
    fn certify_ideal_volume_impl(
        &self,
        caps: [&Surface; 2],
        projection: [[f64; 3]; 3],
        max_cells: usize,
        max_exact_work: u64,
        atlas: bool,
    ) -> Result<IdealVolumeReport> {
        check(
            max_cells <= if atlas { 1000000 } else { 100000 }
                && max_exact_work <= 1000000
                && projection.iter().flatten().all(|x| x.is_finite()),
            "Invalid ideal volume limits or projection",
        )?;
        for cap in caps {
            cap.validate()?;
            check(
                cap.control_points.iter().flatten().all(|p| p.len() == 3),
                "Ideal cap requires XYZ controls",
            )?;
        }
        let mut out = IdealVolumeReport {
            embedding_certified: false,
            regularity_certified: false,
            parameter_topology_preserved: false,
            cells: 0,
            exact_work: 0,
            boxes: 0,
            monotonicity_lower: None,
            projection,
            reason: Some("ideal-volume-work-limit"),
        };
        let s = &self.sweeps[0];
        let closed = self.source_is_closed()?;
        if closed && !atlas {
            out.reason = Some("ideal-volume-closed-quotient-required");
            return Ok(out);
        }
        if !surface_c0(caps[0])
            || !surface_c0(caps[1])
            || [s.path, s.scale, s.twist].into_iter().any(|c| !curve_c0(c))
            || s.affine_laws
                .is_some_and(|(a, c)| !curve_c0(a) || !curve_c0(c))
        {
            out.reason = Some("ideal-volume-source-continuity-unproved");
            return Ok(out);
        }
        // This proves frame continuity independently of cell subdivision. The
        // scalar and positional C0 premises above suffice for the piecewise
        // segment integral; the stronger frame C1 gate is conservative.
        if closed {
            if !closed::premises(s, caps, &mut out, max_cells.min(100000), max_exact_work)? {
                return Ok(out);
            }
        } else {
            let smooth =
                s.certify_original_frame_smoothness(1, max_cells.min(100000), max_exact_work)?;
            out.cells += smooth.cells;
            out.exact_work += smooth.exact_work;
            if !smooth.source_frame_smoothness_certified {
                out.reason = smooth.reason;
                return Ok(out);
            }
        }
        let zero = constant_vector_law([0.; 3])?;
        let Some(initial) = super::profile_domain::original_frame(
            s,
            &zero,
            [0., 0.],
            (max_cells - out.cells).min(100000),
            true,
        )?
        else {
            out.reason = Some("ideal-volume-initial-frame-unproved");
            return Ok(out);
        };
        out.cells += initial.cells;
        if initial.status != Status::Certified {
            out.reason = Some("ideal-volume-initial-frame-unproved");
            return Ok(out);
        }
        let basis = [
            decode(initial.transverse.unwrap())?,
            decode(initial.binormal.unwrap())?,
            decode(initial.longitudinal.unwrap())?,
        ];
        let start = crate::sweeps::progressive_miter::vector_certificate::certify_values_traversal(
            s.path,
            [0., 0.],
            (max_cells - out.cells).min(100000),
            false,
        )?;
        out.cells += start.cells;
        let Some(start) = start.value else {
            out.reason = Some("ideal-volume-path-start-unproved");
            return Ok(out);
        };
        let start = decode(start)?;
        let offset = if let Some(g) = s.orientation_guide {
            exact_offset(s.path, g, &mut out.cells, max_cells)?
        } else {
            None
        };
        let anchor = if s.contact_point.is_some() {
            let report = super::contact_anchor::certify(s, (max_cells - out.cells).min(100000))?;
            out.cells += report.cells;
            let Some(q) = report
                .coordinates
                .filter(|_| report.status == Status::Certified)
            else {
                out.reason = report.reason;
                return Ok(out);
            };
            Some(q[0])
        } else {
            None
        };
        let transport = if s.frame_laws.is_none()
            && s.orientation_guide.is_none()
            && s.options.orientation == Orientation::RotationMinimizing
        {
            if let Some([steps, _, _]) = s.spatial_rmf_error_limits {
                let report = super::rmf_transport::certify_original_rmf_transport(
                    s.path,
                    s.options.normal,
                    steps,
                    (max_cells - out.cells).min(100000),
                    max_exact_work - out.exact_work,
                    closed,
                )?;
                out.cells += report.cells;
                out.exact_work += report.exact_work;
                if report.status != Status::Certified {
                    out.reason = report.reason;
                    return Ok(out);
                }
                Some(report)
            } else {
                None
            }
        } else {
            None
        };
        let arc = if s.options.spacing != Spacing::Parameter {
            let Some(map) = arc::ArcMap::build(s.path, &mut out.cells, max_cells)? else {
                out.reason = Some("ideal-volume-original-arc-map-unproved");
                return Ok(out);
            };
            Some(map)
        } else {
            None
        };
        let mut shared_guide = false;
        let guide_arc = if arc.is_some() {
            if let Some(guide) = s.orientation_guide {
                let identity =
                    super::arc_guide::certify(s.path, guide, (max_cells - out.cells).min(100000))?;
                out.cells += identity.cells;
                shared_guide = identity.scale_upper.is_some();
                if shared_guide {
                    None
                } else {
                    let Some(map) = arc::ArcMap::build(guide, &mut out.cells, max_cells)? else {
                        out.reason = Some("ideal-volume-guide-arc-map-unproved");
                        return Ok(out);
                    };
                    Some(map)
                }
            } else {
                None
            }
        } else {
            None
        };
        let context = cell::Context {
            s,
            caps,
            basis,
            start,
            zero,
            offset,
            anchor,
            transport,
            arc,
            guide_arc,
            shared_guide,
            poses: atlas.then(Default::default),
        };
        if atlas {
            atlas::certify(&context, projection, &mut out, max_cells, closed)?;
            return Ok(out);
        }
        let mut pending = vec![([[0., 1.]; 3], None)];
        let mut lower = f64::INFINITY;
        while let Some((b, cached)) = pending.pop() {
            let bound = if let Some(bound) = cached {
                bound
            } else {
                context.bound(b, projection, &mut out, max_cells)?
            };
            match bound {
                cell::Bound::Lower(value) if value > 0. => {
                    lower = lower.min(value);
                    continue;
                }
                cell::Bound::Unsupported(reason) => {
                    out.reason = Some(reason);
                    return Ok(out);
                }
                _ => {}
            }
            if out.cells == max_cells {
                return Ok(out);
            }
            if caps[0] != caps[1] {
                let axis = (0..3)
                    .max_by(|&i, &j| (b[i][1] - b[i][0]).total_cmp(&(b[j][1] - b[j][0])))
                    .unwrap();
                let Some(children) = split_axis(b, axis) else {
                    out.reason = Some("ideal-volume-precision-limit");
                    return Ok(out);
                };
                pending.push((children[1], None));
                pending.push((children[0], None));
                continue;
            }
            // Native lookahead chooses the split that most improves the worst
            // child bound. Every attempted child is charged, even discarded
            // axes. Cache the chosen pair, so no proof is repeated for credit.
            let mut axes = [0, 1, 2];
            axes.sort_by(|&i, &j| (b[j][1] - b[j][0]).total_cmp(&(b[i][1] - b[i][0])));
            let mut best = None;
            let mut best_score = f64::NEG_INFINITY;
            for axis in axes {
                let Some(children) = split_axis(b, axis) else {
                    continue;
                };
                let left = context.bound(children[0], projection, &mut out, max_cells)?;
                let right = context.bound(children[1], projection, &mut out, max_cells)?;
                for bound in [left, right] {
                    if let cell::Bound::Unsupported(reason) = bound {
                        out.reason = Some(reason);
                        return Ok(out);
                    }
                }
                let score = match (left, right) {
                    (cell::Bound::Lower(a), cell::Bound::Lower(b)) => a.min(b),
                    _ => f64::NEG_INFINITY,
                };
                if best.is_none() || score > best_score {
                    best = Some((children, [left, right]));
                    best_score = score;
                }
                if score > 0. {
                    break;
                }
                if out.cells == max_cells {
                    return Ok(out);
                }
            }
            let Some((children, bounds)) = best else {
                out.reason = Some("ideal-volume-precision-limit");
                return Ok(out);
            };
            pending.push((children[1], Some(bounds[1])));
            pending.push((children[0], Some(bounds[0])));
        }
        out.embedding_certified = true;
        out.regularity_certified = true;
        out.parameter_topology_preserved = true;
        out.monotonicity_lower = Some(lower);
        out.reason = None;
        Ok(out)
    }
}
