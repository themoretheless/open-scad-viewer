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
