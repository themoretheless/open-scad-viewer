//! Original closed boundary differential compatibility. Global volume,
//! material/trim ownership and resulting surface regularity are separate gates.
use super::*;
use cad_predicates::{
    AuthoredScalar, BezierIdentity, Limits, PredicateContext, SourceArena, ToleranceContext,
    rational_bezier_geometric_endpoint_identity,
};

#[derive(Clone, Copy)]
pub struct OriginalBoundarySmoothnessLimits {
    pub order: usize,
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
}
#[derive(Debug)]
pub struct OriginalBoundarySmoothnessReport {
    pub order: usize,
    /// Full Gk needs the owner's original global volume and trim binding.
    pub differential_compatibility_certified: bool,
    pub frame_certified: bool,
    pub laws_certified: bool,
    pub profiles_certified: bool,
    pub inspected_profile_joins: usize,
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
    pub reason: Option<&'static str>,
}
fn periodic_law(c: &Curve, order: usize, spent: &mut u64, max: u64) -> Result<bool> {
    let knots = super::endpoint_jets::certify_knots(c, order, max - *spent)?;
    *spent += knots.exact_work;
    if !knots.certified {
        return Ok(false);
    }
    let seam = super::endpoint_jets::certify(c, order, max - *spent)?;
    *spent += seam.exact_work;
    Ok(seam.certified)
}
fn profile_join(a: &Curve, b: &Curve, order: usize, max: u64) -> Result<(bool, u64)> {
    // Exact decomposition owns these original coefficients. Reverse only the
    // reference's storage so both endpoint arrays point into their own sides.
    let values = a
        .control_points
        .iter()
        .zip(&a.weights)
        .rev()
        .chain(b.control_points.iter().zip(&b.weights))
        .flat_map(|(p, w)| [p[0], p[1], p[2], *w])
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let source = SourceArena::authored("original-profile-geometric-jets", 1, values)
        .map_err(|e| crate::input(e.to_string()))?;
    let n = a.control_points.len();
    let m = b.control_points.len();
    let controls = (0..n + m)
        .map(|i| std::array::from_fn(|k| source.leaf(4 * i + k).unwrap()))
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: max,
            ..Limits::default()
        },
        None,
    );
    let proof = rational_bezier_geometric_endpoint_identity(
        &mut ctx,
        &controls[..n],
        &controls[n..],
        order,
    )
    .map_err(|e| crate::input(e.to_string()))?;
    Ok((proof.outcome == BezierIdentity::Equal, proof.work_used))
}
impl MultiSweep<'_> {
    /// Sufficient original Gk differential compatibility on every source
    /// profile join and the closed traversal seam. Path, scale, affine laws
    /// and frame all require their OWN periodic Ck. A native body owner must
    /// additionally prove the original filled volume, valid trims and exact
    /// profile/carrier binding before issuing a regular-boundary Gk guarantee.
    /// No retained model, sampled seam or caller proof flag supplies credit.
    pub fn certify_original_closed_boundary_smoothness(
        &self,
        loop_sizes: &[usize],
        limits: OriginalBoundarySmoothnessLimits,
    ) -> Result<OriginalBoundarySmoothnessReport> {
        check(
            matches!(limits.order, 1 | 2)
                && limits.cells <= 100000
                && limits.exact_work <= 1000000
                && limits.products <= 1000000
                && !loop_sizes.is_empty()
                && loop_sizes.len() <= 64
                && loop_sizes.iter().all(|&n| n > 0 && n <= 64)
                && loop_sizes.iter().sum::<usize>() == self.sweeps.len(),
            "Invalid original closed boundary smoothness limits or partition",
        )?;
        let s = &self.sweeps[0];
        let mut out = OriginalBoundarySmoothnessReport {
            order: limits.order,
            differential_compatibility_certified: false,
            frame_certified: false,
            laws_certified: false,
            profiles_certified: false,
            inspected_profile_joins: 0,
            cells: 0,
            exact_work: 0,
            products: 0,
            reason: Some("original-boundary-smoothness-unproved"),
        };
        if s.contact_point.is_some() || !self.source_is_closed()? {
            out.reason = Some("original-boundary-closed-motion-mode-unproved");
            return Ok(out);
        }
        // Translation is evaluated in the selected station coordinate.
        // In arc units, position closure and C^(k-1) forward unit tangent
        // prove Ck translation; raw speed derivatives need not be periodic.
        // Scalar/affine station laws still require their original Cartesian Ck.
        let mut translation=periodic_law(s.path,limits.order,&mut out.exact_work,limits.exact_work)?;
        if !translation && s.options.spacing!=Spacing::Parameter {
            translation=true;
            for endpoints in [false,true] {
                let position=super::endpoint_jets::certify_position(s.path,endpoints,limits.exact_work-out.exact_work)?;
                out.exact_work+=position.exact_work;
                if !position.certified {translation=false;break;}
                let tangent=super::endpoint_jets::certify_direction(s.path,limits.order-1,endpoints,true,limits.exact_work-out.exact_work)?;
                out.exact_work+=tangent.exact_work;
                if !tangent.certified {translation=false;break;}
            }
        }
        if !translation {out.reason=Some("original-boundary-periodic-law-jets-unproved");return Ok(out);}
        for law in [s.scale, s.twist]
            .into_iter()
            .chain(s.affine_laws.into_iter().flat_map(|(a, c)| [a, c]))
        {
            if !periodic_law(law, limits.order, &mut out.exact_work, limits.exact_work)? {
                out.reason = Some("original-boundary-periodic-law-jets-unproved");
                return Ok(out);
            }
        }
        if s.options.spacing != Spacing::Parameter {
            let regular = crate::curve_regularity::inspect(s.path, limits.cells - out.cells)?;
            out.cells += regular.cells;
            if !regular.spanwise_regular {
                out.reason = Some("original-boundary-arc-clock-speed-unproved");
                return Ok(out);
            }
        }
        out.laws_certified = true;
        let (certified, cells, work, reason) = if s.frame_laws.is_some() {
            let p = s.certify_closed_authored_frame_smoothness(
                limits.order,
                limits.cells - out.cells,
                limits.exact_work - out.exact_work,
            )?;
            (
                p.closed_source_frame_smoothness_certified,
                p.cells,
                p.exact_work,
                p.reason,
            )
        } else if s.orientation_guide.is_some() {
            let p = s.certify_closed_guided_frame_smoothness(
                limits.order,
                limits.cells - out.cells,
                limits.exact_work - out.exact_work,
            )?;
            (
                p.closed_source_frame_smoothness_certified,
                p.cells,
                p.exact_work,
                p.reason,
            )
        } else {
            let p = s.certify_closed_path_frame_smoothness(
                limits.order,
                limits.cells - out.cells,
                limits.exact_work - out.exact_work,
            )?;
            (
                p.closed_source_frame_smoothness_certified,
                p.cells,
                p.exact_work,
                p.reason,
            )
        };
        out.cells += cells;
        out.exact_work += work;
        out.frame_certified = certified;
        if !certified {
            out.reason = reason;
            return Ok(out);
        }
        let mut offset = 0;
        for &count in loop_sizes {
            let mut pieces = Vec::new();
            for sweep in &self.sweeps[offset..offset + count] {
                let profile = sweep.profile;
                let regular = crate::curve_regularity::inspect(profile, limits.cells - out.cells)?;
                out.cells += regular.cells;
                if !regular.spanwise_regular {
                    out.reason = Some("original-profile-speed-unproved");
                    return Ok(out);
                }
                let rows = (profile.degree..profile.control_points.len())
                    .filter(|&i| profile.knots[i] < profile.knots[i + 1])
                    .count()
                    * (profile.degree + 1);
                if rows > limits.products - out.products {
                    out.reason = Some("original-profile-extraction-work-limit");
                    return Ok(out);
                }
                out.products += rows;
                let Some(spans) = crate::retained_wall_coefficients::exact_bezier_controls(
                    profile,
                    rows.min(4096),
                ) else {
                    out.reason = Some("original-profile-exact-extraction-unproved");
                    return Ok(out);
                };
                if spans.iter().any(|c| c.degree > 32) {
                    out.reason = Some("original-profile-jet-degree-unproved");
                    return Ok(out);
                }
                pieces.extend(spans);
            }
            offset += count;
            for i in 0..pieces.len() {
                let (certified, work) = profile_join(
                    &pieces[i],
                    &pieces[(i + 1) % pieces.len()],
                    limits.order,
                    limits.exact_work - out.exact_work,
                )?;
                out.exact_work += work;
                out.inspected_profile_joins += 1;
                if !certified {
                    out.reason = Some("original-profile-geometric-join-unproved");
                    return Ok(out);
                }
            }
        }
        out.profiles_certified = true;
        out.differential_compatibility_certified = true;
        out.reason = None;
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
