use super::*;
pub(super) fn open(s:&Sweep<'_>,out:&mut OriginalSmoothVolumeReport,l:OriginalSmoothVolumeLimits)->Result<bool> {
    let reason=certify_open(s,l.order,&mut out.cells,&mut out.exact_work,l.cells,l.exact_work)?;
    if reason.is_some(){out.reason=reason;}Ok(reason.is_none())
}
/// Fresh Ck of the original pose/translation, shared by positive global owners.
/// Arc translation requires the caller's fresh positive-speed inverse clock.
pub(crate) fn certify_open(s:&Sweep<'_>,order:usize,cells:&mut usize,work:&mut u64,
    max_cells:usize,max_work:u64)->Result<Option<&'static str>> {
    let f = s.certify_original_frame_smoothness(
        order,
        (max_cells - *cells).min(100000),
        max_work - *work,
    )?;
    *cells += f.cells;
    *work += f.exact_work;
    if !f.source_frame_smoothness_certified {
        return Ok(Some(f.reason.unwrap_or("original-motion-frame-unproved")));
    }
    let raw =
        super::super::endpoint_jets::certify_knots(s.path, order, max_work - *work)?;
    *work += raw.exact_work;
    let mut translation = raw.certified;
    if !translation && s.options.spacing != Spacing::Parameter {
        // The fresh original-volume owner already established positive source
        // speed for its inverse arc clock. C0 position and C^(k-1) forward
        // unit tangent at every original knot then give Ck arc translation.
        let position = super::super::endpoint_jets::certify_position(
            s.path,
            false,
            max_work - *work,
        )?;
        *work += position.exact_work;
        if position.certified {
            let direction = super::super::endpoint_jets::certify_direction(
                s.path,
                order - 1,
                false,
                true,
                max_work - *work,
            )?;
            *work += direction.exact_work;
            translation = direction.certified;
        }
    }
    if !translation {
        return Ok(Some("original-smooth-volume-translation-unproved"));
    }
    for law in std::iter::once(s.scale).chain(s.affine_laws.into_iter().flat_map(|(a, c)| [a, c])) {
        let proof = super::super::endpoint_jets::certify_knots(
            law,
            order,
            max_work - *work,
        )?;
        *work += proof.exact_work;
        if !proof.certified {
            return Ok(Some("original-smooth-volume-law-continuity-unproved"));
        }
    }
    if let Some(reason)=super::super::original_smoothness::contact_motion::certify(
        s,order,false,cells,work,max_cells,max_work)? {
        return Ok(Some(reason));
    }
    Ok(None)
}
