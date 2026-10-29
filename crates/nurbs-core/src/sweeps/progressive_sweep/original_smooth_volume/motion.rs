use super::*;
pub(super) fn open(
    s: &Sweep<'_>,
    out: &mut OriginalSmoothVolumeReport,
    l: OriginalSmoothVolumeLimits,
) -> Result<bool> {
    let f = s.certify_original_frame_smoothness(
        l.order,
        (l.cells - out.cells).min(100000),
        l.exact_work - out.exact_work,
    )?;
    out.cells += f.cells;
    out.exact_work += f.exact_work;
    if !f.source_frame_smoothness_certified {
        out.reason = f.reason;
        return Ok(false);
    }
    let raw =
        super::super::endpoint_jets::certify_knots(s.path, l.order, l.exact_work - out.exact_work)?;
    out.exact_work += raw.exact_work;
    let mut translation = raw.certified;
    if !translation && s.options.spacing != Spacing::Parameter {
        // The fresh original-volume owner already established positive source
        // speed for its inverse arc clock. C0 position and C^(k-1) forward
        // unit tangent at every original knot then give Ck arc translation.
        let position = super::super::endpoint_jets::certify_position(
            s.path,
            false,
            l.exact_work - out.exact_work,
        )?;
        out.exact_work += position.exact_work;
        if position.certified {
            let direction = super::super::endpoint_jets::certify_direction(
                s.path,
                l.order - 1,
                false,
                true,
                l.exact_work - out.exact_work,
            )?;
            out.exact_work += direction.exact_work;
            translation = direction.certified;
        }
    }
    if !translation {
        out.reason = Some("original-smooth-volume-translation-unproved");
        return Ok(false);
    }
    for law in std::iter::once(s.scale).chain(s.affine_laws.into_iter().flat_map(|(a, c)| [a, c])) {
        let proof = super::super::endpoint_jets::certify_knots(
            law,
            l.order,
            l.exact_work - out.exact_work,
        )?;
        out.exact_work += proof.exact_work;
        if !proof.certified {
            out.reason = Some("original-smooth-volume-law-continuity-unproved");
            return Ok(false);
        }
    }
    Ok(true)
}
