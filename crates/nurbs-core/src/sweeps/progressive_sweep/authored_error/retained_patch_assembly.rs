//! Retained patch assembly: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn patch_error_from_section(
    sweep: &Sweep<'_>,
    count: usize,
    max_products: usize,
    section: SectionInterpolationReport,
) -> Result<PatchErrorReport> {
    let mut out = PatchErrorReport {
        status: Status::Unresolved,
        cells: section.cells,
        products: 0,
        section_error_upper: section.error_upper,
        refinement_error_upper: None,
        original_section_endpoint_error_upper: section.endpoint_displacement_upper,
        retained_endpoint_error_upper: None,
        endpoint_contour_error_upper: None,
        decomposition_error_upper: None,
        error_upper: None,
        within_budget: false,
        patches: None,
        reason: section.reason,
    };
    if section.status != Status::Certified {
        return Ok(out);
    }
    let sections = sweep.sections(count)?.0;
    let rows = sections
        .iter()
        .map(profile_parts)
        .collect::<Result<Vec<_>>>()?;
    let mut decomposition = 0_f64;
    if sweep.profile.control_points.len() > 32 {
        for (source, row) in sections.iter().zip(&rows) {
            let proof = crate::curve_decomposition_certificate::inspect_partition(
                source, row, max_products - out.products,
            )?;
            out.products += proof.products;
            let Some(error) = proof.error_upper else {
                out.reason = proof.reason;
                return Ok(out);
            };
            decomposition = decomposition.max(error);
        }
    }
    // Constant rational U weights across stations make V interpolation a
    // convex interpolation of the corresponding decomposed section curves.
    let first = &rows[0];
    if rows.iter().any(|row| {
        row.len() != first.len()
            || row.iter().zip(first).any(|(a, b)| {
                a.degree != b.degree
                    || a.knots != b.knots
                    || a.weights != b.weights
                    || a.periodic != b.periodic
            })
    }) {
        out.reason = Some("decomposed-station-basis-correspondence-unproved");
        return Ok(out);
    }
    let error = I::point(section.error_upper.unwrap())
        .add(I::point(decomposition))?
        .hi;
    let retained = patches(&sections)?;
    out.status = Status::Certified;
    out.error_upper = Some(error);
    out.refinement_error_upper=section.refinement_error_upper
        .map(|upper|I::point(upper).add(I::point(decomposition)).map(|bound|bound.hi)).transpose()?;
    out.decomposition_error_upper = Some(decomposition);
    out.retained_endpoint_error_upper = section.endpoint_displacement_upper
        .map(|endpoint| I::point(endpoint).add(I::point(decomposition)).map(|bound|bound.hi))
        .transpose()?;
    out.endpoint_contour_error_upper=section.cap_endpoint_displacement_upper
        .or_else(||section.endpoint_displacement_upper.map(|e|[e;2]))
        .map(|ends|->Result<[f64;2]>{Ok([I::point(ends[0]).add(I::point(decomposition))?.hi,I::point(ends[1]).add(I::point(decomposition))?.hi])}).transpose()?;
    out.within_budget = error <= sweep.options.max_deviation;
    out.patches = Some(retained);
    out.reason = None;
    Ok(out)
}

pub(crate) fn original_line(path:&Curve)->bool{
    path.degree==1&&!path.periodic&&path.control_points.len()==2&&path.weights.len()==2
        &&(0..3).any(|k|path.control_points[0][k]!=path.control_points[1][k])
}

/// An original rational path with constant coordinate has tangent orthogonal
/// to that coordinate axis. An axial seed normal is therefore a constant
/// Bishop normal (N'=0); regularity is proved by the transport certificate.
/// This is coefficient identity, not sampled planarity or straightness.
pub(crate) fn original_planar_rmf(path:&Curve,normal:[f64;3])->bool {
    !path.periodic && !path.control_points.is_empty() && normal.iter().all(|v|v.is_finite())
        && (0..3).any(|axis|normal[axis]!=0. && (0..3).all(|k|k==axis||normal[k]==0.)
            && path.control_points.iter().all(|p|p.len()==3&&p[axis]==path.control_points[0][axis]))
        // Closed sources additionally require constructor-owned exact normal
        // and zero-correction evidence before retained error admission.
}
