//! Original positive affine transverse coordinates, not sampled pose bounds.
use super::*;
use crate::sweeps::progressive_miter::{scalar_certificate, vector_certificate};

/// The exact plane premise gives q_z=0. At any station, positive uniform and
/// transverse axis scales give a regular injective coordinate change
/// (u,v,s) -> (a(s)q_x+c_x(s), b(s)q_y+c_y(s),s). Its image is contained in
/// one physical transverse disk. Extending the frame map over that disk does
/// not require the original carrier image or its trimmed subset to be convex.
pub(super) fn radius(
    s: &Sweep<'_>,
    carrier: f64,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<Option<f64>> {
    radius_impl(s, carrier, out, max, true)
}
/// Complete 3D section ball. No zero axial-coordinate premise is used.
pub(super) fn full_radius(
    s: &Sweep<'_>,
    carrier: f64,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<Option<f64>> {
    radius_impl(s, carrier, out, max, false)
}
fn radius_impl(
    s: &Sweep<'_>,
    carrier: f64,
    out: &mut IdealVolumeReport,
    max: usize,
    transverse: bool,
) -> Result<Option<f64>> {
    // Preserve the absent identity's existing zero-work route.
    if s.affine_laws.is_none()
        && s.scale
            .control_points
            .iter()
            .all(|p| p.as_slice() == [1., 0., 0.])
    {
        return Ok(Some(carrier));
    }
    let uniform =
        scalar_certificate::certify_traversal(s.scale, [0., 1.], (max - out.cells).min(100000))?;
    out.cells += uniform.cells;
    let Some(uniform) = uniform
        .value
        .filter(|v| uniform.status == Status::Certified && v[0] > 0.)
    else {
        return Ok(None);
    };
    let mut axis = 1_f64;
    let mut transverse_center = 0_f64;
    let mut axial_center = 0_f64;
    if let Some((axes, center)) = s.affine_laws {
        let a = vector_certificate::certify_values_traversal(
            axes,
            [0., 1.],
            (max - out.cells).min(100000),
            true,
        )?;
        out.cells += a.cells;
        let Some(a) = a.value.filter(|a| a.iter().all(|v| v[0] > 0.)) else {
            return Ok(None);
        };
        // The original carrier's exact axial coordinate is zero. Axis scale
        // in that coordinate is nevertheless freshly positive as required by
        // the source contract; it cannot hide a transverse rank loss.
        axis = a[0][1].max(a[1][1]);
        if !transverse {
            axis = axis.max(a[2][1]);
        }
        let center = vector_certificate::certify_values_traversal(
            center,
            [0., 1.],
            (max - out.cells).min(100000),
            false,
        )?;
        out.cells += center.cells;
        let Some(center) = center.value else {
            return Ok(None);
        };
        let center = decode(center)?;
        transverse_center = iv::norm([center[0], center[1], I::point(0.)])?.hi;
        axial_center = center[2].lo.abs().max(center[2].hi.abs());
        if !transverse {
            transverse_center = iv::norm(center)?.hi;
            axial_center = 0.;
        }
    }
    let transverse = I::point(uniform[1])
        .mul(I::point(axis))?
        .mul(I::point(carrier))?
        .add(I::point(transverse_center))?;
    // This also bounds the rotation coefficient on the ENTIRE extended disk,
    // with independently enclosed c_z(s), not only on actual section points.
    Ok(Some(
        iv::norm([transverse, I::point(axial_center), I::point(0.)])?.hi,
    ))
}

#[derive(Clone, Copy)]
pub(super) struct FullCoefficients {
    pub value: V3,
    pub first: V3,
    pub center_first: V3,
    pub center: V3,
}
/// Original diagonal pose coefficients and station derivatives in all 3 axes.
pub(super) fn full_coefficients(
    s: &Sweep<'_>,
    station: [f64; 2],
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<Option<FullCoefficients>> {
    if out.cells == max {
        return Ok(None);
    }
    let u = scalar_certificate::certify_traversal(s.scale, station, (max - out.cells).min(100000))?;
    out.cells += u.cells;
    if u.status != Status::Certified {
        return Ok(None);
    }
    let uv = I::new(u.value.unwrap()[0], u.value.unwrap()[1])?;
    if uv.lo <= 0. {
        return Ok(None);
    }
    let width = |c: &Curve| {
        let [a, b] = c.domain();
        I::point(b).sub(I::point(a))
    };
    let ud = I::new(u.first.unwrap()[0], u.first.unwrap()[1])?.mul(width(s.scale)?)?;
    let mut av = [I::point(1.); 3];
    let mut ad = [I::point(0.); 3];
    let mut cd = ad;
    let mut cv = ad;
    if let Some((axes, center)) = s.affine_laws {
        let a = vector_certificate::certify_traversal(
            axes,
            station,
            (max - out.cells).min(100000),
            true,
        )?;
        out.cells += a.cells;
        if a.status != Status::Certified {
            return Ok(None);
        }
        av = decode(a.value.unwrap())?;
        ad = iv::scale(decode(a.first.unwrap())?, width(axes)?)?;
        let c = vector_certificate::certify_traversal(
            center,
            station,
            (max - out.cells).min(100000),
            false,
        )?;
        out.cells += c.cells;
        if c.status != Status::Certified {
            return Ok(None);
        }
        cd = iv::scale(decode(c.first.unwrap())?, width(center)?)?;
        cv = decode(c.value.unwrap())?;
    }
    let mut value = av;
    let mut first = ad;
    for k in 0..3 {
        value[k] = uv.mul(av[k])?;
        first[k] = ud.mul(av[k])?.add(uv.mul(ad[k])?)?;
    }
    Ok(Some(FullCoefficients {
        value,
        first,
        center_first: cd,
        center: cv,
    }))
}

/// Only axial center motion remains after the physical transverse coordinate
/// change. Center, axis and uniform-scale transverse derivatives are absorbed
/// by that change; omitting c_z' would incorrectly admit a rank-two body.
pub(super) fn axial_rate(
    s: &Sweep<'_>,
    station: [f64; 2],
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<Option<f64>> {
    let Some((_, center)) = s.affine_laws else {
        return Ok(Some(0.));
    };
    let proof = vector_certificate::certify_traversal(
        center,
        station,
        (max - out.cells).min(100000),
        false,
    )?;
    out.cells += proof.cells;
    let Some(first) = proof.first.filter(|_| proof.status == Status::Certified) else {
        return Ok(None);
    };
    let [a, b] = center.domain();
    let rate = I::new(first[2][0], first[2][1])?.mul(I::point(b).sub(I::point(a))?)?;
    Ok(Some(rate.lo.abs().max(rate.hi.abs())))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_section_ball_covers_axial_scale_and_center_without_a_plane_premise() {
        let profiles = [crate::primitives::line([0., 0., 1.], [0.25, 0., 0.5]).unwrap()];
        let path = crate::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
        let scale = constant_vector_law([2., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let axes = constant_vector_law([1., 1., 4.]).unwrap();
        let center = constant_vector_law([0., 0., 2.]).unwrap();
        let sweep = MultiSweep::new(
            &profiles,
            &path,
            &scale,
            &twist,
            Options {
                normal: [1., 0., 0.],
                orientation: Orientation::Fixed,
                spacing: Spacing::Parameter,
                initial_sections: 3,
                max_sections: 9,
                max_deviation: 0.05,
            },
        )
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
        let fresh = || IdealVolumeReport {
            embedding_certified: false,
            regularity_certified: false,
            parameter_topology_preserved: false,
            cells: 0,
            exact_work: 0,
            boxes: 0,
            monotonicity_lower: None,
            projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            reason: None,
        };
        let mut report = fresh();
        let radius = full_radius(&sweep.sweeps[0], 1., &mut report, 100000)
            .unwrap()
            .unwrap();
        // The actual axial offset of q=(0,0,1) is 2*4*1+2=10.
        // A transverse-only bound using the first two scales misses this point.
        assert!(radius >= 10. && radius < 10.000000001);
        let exact = report.cells;
        assert!(exact > 0);
        let mut rerun = fresh();
        assert_eq!(
            full_radius(&sweep.sweeps[0], 1., &mut rerun, exact).unwrap(),
            Some(radius)
        );
        for limit in [0, exact - 1] {
            let mut stopped = fresh();
            assert!(
                full_radius(&sweep.sweeps[0], 1., &mut stopped, limit)
                    .unwrap()
                    .is_none()
            );
            assert!(stopped.cells <= limit);
            assert!(
                !stopped.embedding_certified
                    && !stopped.regularity_certified
                    && !stopped.parameter_topology_preserved
            );
        }
    }
}
