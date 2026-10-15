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
    pub parameter_topology_preserved: bool,
    pub cells: usize,
    pub exact_work: u64,
    pub boxes: usize,
    pub monotonicity_lower: Option<f64>,
    pub projection: [[f64; 3]; 3],
    pub reason: Option<&'static str>,
}
mod charts;
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
        check(
            max_cells <= 100000
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
        if self.source_is_closed()? {
            out.reason = Some("ideal-volume-closed-quotient-required");
            return Ok(out);
        }
        if s.options.spacing != Spacing::Parameter {
            out.reason = Some("ideal-volume-arc-jets-unproved");
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
        let smooth = s.certify_original_frame_smoothness(1, max_cells, max_exact_work)?;
        out.cells += smooth.cells;
        out.exact_work += smooth.exact_work;
        if !smooth.source_frame_smoothness_certified {
            out.reason = smooth.reason;
            return Ok(out);
        }
        let zero = constant_vector_law([0.; 3])?;
        let Some(initial) =
            super::profile_domain::original_frame(s, &zero, [0., 0.], max_cells - out.cells, true)?
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
            max_cells - out.cells,
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
            let report = super::contact_anchor::certify(s, max_cells - out.cells)?;
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
        let mut pending = vec![[[0., 1.]; 3]];
        let mut lower = f64::INFINITY;
        while let Some(b) = pending.pop() {
            if out.cells == max_cells {
                return Ok(out);
            }
            out.cells += 1;
            out.boxes += 1;
            let Some(a) = cap_jets(caps[0], [b[0], b[1]], &mut out.cells, max_cells)? else {
                return Ok(out);
            };
            let Some(z) = cap_jets(caps[1], [b[0], b[1]], &mut out.cells, max_cells)? else {
                return Ok(out);
            };
            let t = I::new(b[2][0], b[2][1])?;
            let one = I::point(1.).sub(t)?;
            let mut q = [[I::point(0.); 3]; 3];
            for j in 0..3 {
                q[j] = local(iv::add(iv::scale(a[j], one)?, iv::scale(z[j], t)?)?, basis)?;
            }
            // q[0] is an affine position, q[1..] are linear differential vectors.
            q[0] = iv::sub(q[0], local(start, basis)?)?;
            let qt = local(iv::sub(z[0], a[0])?, basis)?;
            let fit = if let Some(anchor) = anchor {
                let fit = if let Some(offset) = &offset {
                    frame::certify_contact_fit(
                        &zero,
                        offset,
                        s.scale,
                        s.affine_laws,
                        anchor,
                        b[2],
                        max_cells - out.cells,
                    )?
                } else {
                    frame::certify_contact_fit(
                        s.path,
                        s.orientation_guide.unwrap(),
                        s.scale,
                        s.affine_laws,
                        anchor,
                        b[2],
                        max_cells - out.cells,
                    )?
                };
                out.cells += fit.cells;
                if fit.status != Status::Certified {
                    return Ok(out);
                }
                let value = fit.value.unwrap();
                if value[0] <= 0. {
                    out.reason = Some("ideal-volume-contact-positivity-unproved");
                    return Ok(out);
                }
                Some([
                    I::new(value[0], value[1])?,
                    I::new(fit.first.unwrap()[0], fit.first.unwrap()[1])?,
                    I::new(fit.second.unwrap()[0], fit.second.unwrap()[1])?,
                ])
            } else {
                None
            };
            let Some(mut f) = original_jets(s, b[2], max_cells - out.cells, offset.as_ref())?
            else {
                out.reason = Some("ideal-volume-original-frame-jets-unproved");
                return Ok(out);
            };
            let actual = frame::control_trajectories_with_fit(
                s.path,
                s.scale,
                s.affine_laws,
                &[encode(q[0])],
                b[2],
                max_cells - out.cells,
                f.clone(),
                fit,
            )?;
            out.cells += actual.cells;
            let Some(actual) = actual.jets else {
                return Ok(out);
            };
            // Reuse the SAME original frame enclosure without charging its
            // source work twice. UV derivatives have no positional translation
            // or center, so do not subtract independent wide position boxes.
            f.cells = 0;
            let affine = s.affine_laws.map(|(a, _)| (a, &zero));
            let linear = frame::control_trajectories_with_fit(
                &zero,
                s.scale,
                affine,
                &[encode(q[1]), encode(q[2]), encode(qt)],
                b[2],
                max_cells - out.cells,
                f,
                fit,
            )?;
            out.cells += linear.cells;
            let Some(linear) = linear.jets else {
                return Ok(out);
            };
            let columns = [
                decode(linear[0].value)?,
                decode(linear[1].value)?,
                iv::add(decode(actual[0].first)?, decode(linear[2].value)?)?,
            ];
            let bound = margin(columns, projection)?;
            if bound > 0. {
                lower = lower.min(bound);
                continue;
            }
            // Deterministic longest normalized side. Children cover the parent
            // including their shared boundary; every failed parent is charged.
            let axis = (0..3)
                .max_by(|&i, &j| (b[i][1] - b[i][0]).total_cmp(&(b[j][1] - b[j][0])))
                .unwrap();
            let mid = crate::distance_bounds::midpoint(b[axis][0], b[axis][1]);
            if !(b[axis][0] < mid && mid < b[axis][1]) {
                out.reason = Some("ideal-volume-precision-limit");
                return Ok(out);
            }
            let mut left = b;
            let mut right = b;
            left[axis][1] = mid;
            right[axis][0] = mid;
            pending.push(right);
            pending.push(left);
        }
        out.embedding_certified = true;
        out.regularity_certified = true;
        out.parameter_topology_preserved = true;
        out.monotonicity_lower = Some(lower);
        out.reason = None;
        Ok(out)
    }
}
