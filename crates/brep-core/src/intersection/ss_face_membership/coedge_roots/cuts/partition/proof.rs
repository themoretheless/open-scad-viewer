use super::*;
pub(super) fn agreement(
    c: &Curve,
    pc: &Curve,
    s: &Surface,
    reversed: bool,
    tolerance: f64,
    cells: &mut usize,
    max: usize,
) -> Result<()> {
    if *cells == max {
        return Err(unproven(
            "Shared actual cut correspondence budget exhausted",
        ));
    }
    let report = nurbs_core::curve_surface_agreement::verify(
        c,
        pc,
        s,
        reversed,
        tolerance,
        (max - *cells).min(100000),
    )?;
    spend(cells, max, report.cells)?;
    if report.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
        return Err(unproven(
            "Actual cut original-reference/lift proof remains unresolved",
        ));
    }
    Ok(())
}
pub(super) fn carrier(c: &Curve) -> Surface {
    Surface {
        degree_u: c.degree,
        degree_v: 1,
        knots_u: c.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: c
            .control_points
            .iter()
            .map(|p| vec![p.clone(), p.clone()])
            .collect(),
        weights: c.weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
pub(super) fn embed_uv(c: &Curve) -> Curve {
    let mut c = c.clone();
    for p in &mut c.control_points {
        p.push(0.);
    }
    c
}
pub(super) fn unit_clock(domain: [f64; 2], band: [f64; 2], reverse: bool) -> Result<[f64; 2]> {
    let d = I::point(domain[1]).sub(I::point(domain[0]))?;
    let t = I::new(band[0], band[1])?
        .sub(I::point(domain[0]))?
        .div(d)?
        .intersect(0., 1.)?;
    let t = if reverse {
        I::point(1.).sub(t)?.intersect(0., 1.)?
    } else {
        t
    };
    Ok([t.lo, t.hi])
}
