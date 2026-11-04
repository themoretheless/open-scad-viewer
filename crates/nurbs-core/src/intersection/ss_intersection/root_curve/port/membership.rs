use super::*;
pub(super) fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::input("Nonfinite original endpoint coordinate"))
}
pub(super) fn bind(
    point: &OriginalSsRootPoint,
    target: &OriginalSsRootCurve,
    phase: [i32; 4],
    limits: OriginalSsCurveLimits,
) -> Result<[Q; 2]> {
    let bounds = point.enclose(limits)?;
    let exact = q(point.parameter)?;
    bind_bounds(&point.curve, &bounds, [exact.clone(), exact], target, phase)
}
pub(crate) fn bind_bounds(
    source: &OriginalSsRootCurve,
    bounds: &OriginalSsCurveBounds,
    clock: [Q; 2],
    target: &OriginalSsRootCurve,
    phase: [i32; 4],
) -> Result<[Q; 2]> {
    let a = source.source();
    let b = target.source();
    check(
        a.first() == b.first() && a.second() == b.second(),
        "Original SS endpoint carriers or ordered roles differ",
    )?;
    let frame = frames::frame(source)?;
    let target_frame = frames::frame(target)?;
    check(
        frame.iter().chain(&target_frame).all(|x| x[1] > Q::zero()),
        "Original SS endpoint frame is not increasing",
    )?;
    let domain = target.coordinate_domain();
    let mut target_box: [[Q; 2]; 4] =
        std::array::from_fn(|k| [q(bounds.uv[k][0]).unwrap(), q(bounds.uv[k][1]).unwrap()]);
    let fixed = source.free_axis().index();
    target_box[fixed] = [
        &frame[fixed][0] + &frame[fixed][1] * &clock[0],
        &frame[fixed][0] + &frame[fixed][1] * &clock[1],
    ];
    let free = target.free_axis().index();
    for k in 0..4 {
        let surface = if k < 2 { b.first() } else { b.second() };
        let (knots, p, n, periodic) = if k % 2 == 0 {
            (
                &surface.knots_u,
                surface.degree_u,
                surface.control_points.len(),
                surface.periodic_u,
            )
        } else {
            (
                &surface.knots_v,
                surface.degree_v,
                surface.control_points[0].len(),
                surface.periodic_v,
            )
        };
        let shift = if phase[k] == 0 {
            Q::zero()
        } else {
            check(
                periodic
                    && crate::periodic_trim::verify_surface_seam_identity(surface, k % 2)?
                        .is_some(),
                "Original SS endpoint phase has no exact native seam identity",
            )?;
            (q(knots[n])? - q(knots[p])?) * Q::from_integer(phase[k].into())
        };
        let lo = (&target_box[k][0] + &shift - &target_frame[k][0]) / &target_frame[k][1];
        let hi = (&target_box[k][1] + shift - &target_frame[k][0]) / &target_frame[k][1];
        check(
            q(domain[k][0])? <= lo && hi <= q(domain[k][1])?,
            "Original root selector is not proved inside the target SS graph",
        )?;
        target_box[k] = [lo, hi];
    }
    if let Carrier::Pole(f) = &target.carrier {
        let coordinate = 2 * f.source().pole().support() + f.source().pole().axis();
        let value = Q::from_integer(f.source().pole().shore().into());
        check(
            target_box[coordinate][1] < value || value < target_box[coordinate][0],
            "Pole aliases need exact collapsed-point endpoint authority",
        )?;
    }
    Ok(target_box[free].clone())
}
