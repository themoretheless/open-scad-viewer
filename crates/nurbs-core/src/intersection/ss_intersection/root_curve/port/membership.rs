use super::*;
pub(super) fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::input("Nonfinite original endpoint coordinate"))
}
pub(in super::super) fn bind(
    point: &OriginalSsRootPoint,
    target: &OriginalSsRootCurve,
    phase: [i32; 4],
    limits: OriginalSsCurveLimits,
) -> Result<[Q; 2]> {
    let bounds = point.enclose(limits)?;
    bind_bounds(&point.curve, &bounds, point.clock_bounds()?, target, phase)
}
fn native_period(source: &FiberSources, coordinate: usize) -> Result<Option<Q>> {
    let surface = if coordinate < 2 { source.first() } else { source.second() };
    let axis = coordinate % 2;
    let (knots, degree, count, periodic) = if axis == 0 {
        (&surface.knots_u, surface.degree_u, surface.control_points.len(), surface.periodic_u)
    } else {
        (&surface.knots_v, surface.degree_v, surface.control_points[0].len(), surface.periodic_v)
    };
    if !periodic || crate::periodic_trim::verify_surface_seam_identity(surface, axis)?.is_none() {
        return Ok(None);
    }
    Ok(Some(q(knots[count])? - q(knots[degree])?))
}

pub(super) fn infer_phase(
    point: &OriginalSsRootPoint,
    target: &OriginalSsRootCurve,
    limits: OriginalSsCurveLimits,
) -> Result<[i32; 4]> {
    let source = point.curve.source();
    let other = target.source();
    check(
        source.first() == other.first() && source.second() == other.second(),
        "Original SS endpoint carriers or ordered roles differ",
    )?;
    let image = point.enclose(limits)?.uv;
    let fixed = point.curve.free_axis().index();
    let clock = point.clock_bounds()?;
    let frame = frames::frame(&point.curve)?;
    let target_frame = frames::frame(target)?;
    let domain = target.coordinate_domain();
    let mut phase = [0; 4];
    for k in 0..4 {
        check(target_frame[k][1] > Q::zero(), "Original SS endpoint frame is not increasing")?;
        let bounds = if k == fixed {
            clock.each_ref().map(|value| &frame[k][0] + &frame[k][1] * value)
        } else {
            [q(image[k][0])?, q(image[k][1])?]
        };
        let target_bounds = [
            &target_frame[k][0] + &target_frame[k][1] * q(domain[k][0])?,
            &target_frame[k][0] + &target_frame[k][1] * q(domain[k][1])?,
        ];
        let period = native_period(other, k)?;
        let mut found = None;
        for offset in -2..=2 {
            let shift = match &period {
                Some(period) => period * Q::from_integer(offset.into()),
                None if offset == 0 => Q::zero(),
                None => continue,
            };
            if target_bounds[0] <= &bounds[0] + &shift && &bounds[1] + &shift <= target_bounds[1] {
                check(found.is_none(), "Original SS endpoint has ambiguous native phase")?;
                found = Some(offset);
            }
        }
        phase[k] = found.ok_or_else(|| crate::input("No proved native phase for original SS endpoint"))?;
    }
    Ok(phase)
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
        let shift = if phase[k] == 0 {
            Q::zero()
        } else {
            native_period(b, k)?
                .ok_or_else(|| crate::input("Original SS endpoint phase has no exact native seam identity"))?
                * Q::from_integer(phase[k].into())
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
