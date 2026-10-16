//! Local embedding in physical transverse coordinates of an ORIGINAL Bishop
//! tube. The reference frame is ONE exact (existential) orthonormal frame at a
//! chart endpoint, not independent world-coordinate boxes for its columns.
use super::*;

pub(super) struct Tube {
    radius: I,
}
mod atlas;
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
    ) -> Result<Option<Self>> {
        let s = c.s;
        if c.arc.is_none()
            || c.transport.is_none()
            || c.caps[0] != c.caps[1]
            || s.affine_laws.is_some()
            || s.frame_laws.is_some()
            || s.orientation_guide.is_some()
            || s.contact_point.is_some()
            || s.options.orientation != Orientation::RotationMinimizing
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
        if !plane::certify(path, cap, &mut out.exact_work, exact)? || out.cells == max {
            return Ok(None);
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
        let transport = c.transport.as_ref().unwrap();
        let angle = transport
            .closing_angle
            .map_or(0., |a| a[0].abs().max(a[1].abs()));
        let mut curvature = 0_f64;
        let mut angular = 0_f64;
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
        tube_bound(
            width,
            self.radius,
            length,
            I::point(curvature),
            I::point(angular),
        )
    }
}

/// For G(x,y,s)=C(s)+x N(s)+y B(s), |(x,y)|<=rho, choose the
/// SAME exact frame R0 at one endpoint and change coordinate z=L*s.
/// ||R0^T DG diag(1,1,1/L)-I||F² is bounded by
/// 2*(Omega*d)² + (K*d+rho*Omega/L)². If <1, every averaged
/// Jacobian on the convex physical disk x temporal lift is nonsingular.
/// Thus G is injective and regular there. The independently proved regular
/// injective Q transfers this to source UV, without requiring its image convex.
/// The bound covers both seam pieces with the SAME periodic frame R0.
fn tube_bound(width: I, radius: I, length: I, curvature: I, angular: I) -> Result<bool> {
    if length.lo <= 0. {
        return Ok(false);
    }
    let turn = angular.mul(width)?;
    let axial = curvature
        .mul(width)?
        .add(radius.mul(angular)?.div(length)?)?;
    let squared = I::point(2.).mul(turn.mul(turn)?)?.add(axial.mul(axial)?)?;
    Ok(squared.hi < 1.)
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
