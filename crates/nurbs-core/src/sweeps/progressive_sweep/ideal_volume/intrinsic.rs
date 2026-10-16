//! Local embedding in physical transverse coordinates of an ORIGINAL moving
//! orthonormal frame. One exact endpoint reference supplies the spectral gap;
//! independent world-coordinate frame boxes never supply inverse credit.
use super::*;

pub(super) struct Tube {
    radius: I,
    path_aligned: bool,
}
mod atlas;
mod frame_bounds;
mod plane;
impl Tube {
    pub(super) fn certify(
        &self,
        c: &cell::Context<'_, '_>,
        out: &mut IdealVolumeReport,
        max: usize,
        closed: bool,
    ) -> Result<()> {
        atlas::certify(self, c, out, max, closed)
    }
    /// These premises are recomputed by the original owner. No report from a
    /// caller, sampled carrier, rounded plane or retained BRep is accepted.
    pub(super) fn prepare(
        c: &cell::Context<'_, '_>,
        out: &mut IdealVolumeReport,
        max: usize,
        exact: u64,
        closed: bool,
    ) -> Result<Option<Self>> {
        let s = c.s;
        if c.arc.is_none()
            || c.caps[0] != c.caps[1]
            || s.affine_laws.is_some()
            || s.contact_point.is_some()
            || s.scale
                .control_points
                .iter()
                .any(|p| p.as_slice() != [1., 0., 0.])
        {
            return Ok(None);
        }
        let path = s.path;
        let p = path.degree;
        let [a, b] = path.domain();
        if p == 0
            || path.periodic
            || path.knots[..=p].iter().any(|u| *u != a)
            || path.knots[path.control_points.len()..]
                .iter()
                .any(|u| *u != b)
        {
            return Ok(None);
        }
        if out.cells == max || out.exact_work == exact {
            return Ok(None);
        }
        out.cells += 1;
        let cap = c.caps[0];
        // Integrating the tangent-rate bound across source knots/seams needs
        // its OWN connected position/forward-tangent premises. An authored
        // smooth frame does not establish any path derivative continuity.
        if c.transport.is_none() && !frame_bounds::tangent_continuity(path, closed, out, exact)? {
            return Ok(None);
        }
        if !plane::certify_frame(
            path,
            cap,
            s.frame_laws.map(|(a, _)| a),
            &mut out.exact_work,
            exact,
        )? || out.cells == max
        {
            return Ok(None);
        }
        if c.transport.is_none() {
            let stage = max.min(out.cells.saturating_add(100000));
            let Some(phase) = c
                .arc
                .as_ref()
                .unwrap()
                .phase([0., 0.], &mut out.cells, stage)?
            else {
                return Ok(None);
            };
            if frame_bounds::jets(c, [0., 0.], phase, out, max)?.is_none() {
                return Ok(None);
            }
        }
        // This global contraction proves BOTH injectivity and rank two over
        // the complete natural carrier rectangle, including one-sided knots.
        // Validity and binding of its material trims remain BRep obligations.
        let cover =
            crate::surface_injectivity::certify_contraction(cap, (max - out.cells).min(100000))?;
        out.cells += cover.spans;
        if !cover.proven {
            return Ok(None);
        }
        let origin = path.control_points[0]
            .iter()
            .copied()
            .map(I::point)
            .collect::<Vec<_>>();
        let mut radius = 0_f64;
        for q in cap.control_points.iter().flatten() {
            if out.cells == max {
                return Ok(None);
            }
            out.cells += 1;
            radius = radius.max(
                crate::distance_bounds::box_distance(
                    &q.iter().copied().map(I::point).collect::<Vec<_>>(),
                    &origin,
                )?
                .1,
            );
        }
        Ok(Some(Self {
            radius: I::point(radius),
            path_aligned: s.frame_laws.is_none()
                && (s.orientation_guide.is_some() || s.options.orientation != Orientation::Fixed),
        }))
    }

    pub(super) fn local(
        &self,
        c: &cell::Context<'_, '_>,
        pieces: &[Box3],
        out: &mut IdealVolumeReport,
        max: usize,
    ) -> Result<bool> {
        if out.cells == max {
            return Ok(false);
        }
        out.cells += 1;
        let mut width = I::point(0.);
        for b in pieces {
            width = width.add(I::point(b[2][1]).sub(I::point(b[2][0]))?)?;
        }
        // Proposal only. A whole loop must never be treated as a convex lift.
        if width.hi > 0.125 {
            return Ok(false);
        }
        let map = c.arc.as_ref().unwrap();
        let length = map.length();
        let angle = c
            .transport
            .as_ref()
            .and_then(|t| t.closing_angle)
            .map_or(0., |a| a[0].abs().max(a[1].abs()));
        let mut curvature = 0_f64;
        let mut angular = 0_f64;
        let mut singular_floor = I::point(1.);
        for b in pieces {
            let stage_max = max.min(out.cells.saturating_add(100000));
            let Some(phase) = map.phase(b[2], &mut out.cells, stage_max)? else {
                return Ok(false);
            };
            let Some(jets) = phase.position_jets else {
                return Ok(false);
            };
            // C_s=L T, hence ||T_s|| <= ||C_ss||/L_lower.
            let k = iv::norm(jets[1])?.div(length)?.hi;
            if c.transport.is_none() {
                let Some(frame) = frame_bounds::jets(c, b[2], phase, out, max)? else {
                    return Ok(false);
                };
                let omega = frame_bounds::angular(&frame)?;
                let floor = if self.path_aligned {
                    I::point(1.)
                } else {
                    frame_bounds::singular_floor(
                        iv::div(jets[0], length)?,
                        decode(frame.longitudinal.as_ref().unwrap().value)?,
                    )?
                };
                singular_floor = I::point(singular_floor.lo.min(floor.lo));
                curvature = curvature.max(k);
                angular = angular.max(omega);
                continue;
            }
            if out.cells == max {
                return Ok(false);
            }
            let twist = crate::sweeps::progressive_miter::scalar_certificate::certify_traversal(
                c.s.twist,
                b[2],
                (max - out.cells).min(100000),
            )?;
            out.cells += twist.cells;
            if twist.status != Status::Certified {
                return Ok(false);
            }
            let [a, z] = c.s.twist.domain();
            let beta = I::new(twist.first.unwrap()[0], twist.first.unwrap()[1])?
                .mul(I::point(z).sub(I::point(a))?)?;
            let omega = I::point(k)
                .add(I::point(angle))?
                .add(I::point(beta.lo.abs().max(beta.hi.abs())))?
                .hi;
            curvature = curvature.max(k);
            angular = angular.max(omega);
        }
        tube_bound_with_floor(
            width,
            self.radius,
            length,
            I::point(curvature),
            I::point(angular),
            singular_floor,
        )
    }
}

/// For G(x,y,s)=C(s)+x N(s)+y B(s), |(x,y)|<=rho, choose the
/// SAME exact columns A0=[N0,B0,T0] at one endpoint and z=L*s.
/// ||DG diag(1,1,1/L)-A0||F² is bounded by
/// 2*(Omega*d)² + (K*d+rho*Omega/L)². If below sigma_min(A0)²,
/// every averaged
/// Jacobian on the convex physical disk x temporal lift is nonsingular.
/// Thus G is injective and regular there. The independently proved regular
/// injective Q transfers this to source UV, without requiring its image convex.
/// The bound covers both seam pieces with the SAME periodic reference. For
/// path-aligned frames A0 is orthonormal and the spectral floor is one.
fn tube_bound(width: I, radius: I, length: I, curvature: I, angular: I) -> Result<bool> {
    tube_bound_with_floor(width, radius, length, curvature, angular, I::point(1.))
}
fn tube_bound_with_floor(
    width: I,
    radius: I,
    length: I,
    curvature: I,
    angular: I,
    floor: I,
) -> Result<bool> {
    if length.lo <= 0. {
        return Ok(false);
    }
    let turn = angular.mul(width)?;
    let axial = curvature
        .mul(width)?
        .add(radius.mul(angular)?.div(length)?)?;
    let squared = I::point(2.).mul(turn.mul(turn)?)?.add(axial.mul(axial)?)?;
    Ok(squared.hi < floor.lo)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transverse_tube_bound_requires_both_turn_and_focal_radius() {
        let test = |d, r, l, k, o| {
            tube_bound(
                I::point(d),
                I::point(r),
                I::point(l),
                I::point(k),
                I::point(o),
            )
            .unwrap()
        };
        assert!(test(0.01, 0.01, 10., 4., 5.));
        assert!(!test(0.5, 0.01, 10., 4., 5.));
        assert!(!test(0., 3., 10., 4., 5.));
        assert!(!test(0., 0.01, 0., 4., 5.));
        assert!(!test(0., 2., 10., 0., 5.)); // strict bound, focal equality refuses
    }
}
#[cfg(test)]
mod general_tests;
