//! Complete source-value cover against ONE original retained station chord.
//! Subdivision never creates replacement sections or derivative/continuity credit.
use super::*;
fn bounds(
    values: &[[[f64; 2]; 3]],
    leaf: [f64; 2],
    whole: [f64; 2],
    a: &Curve,
    b: &Curve,
) -> Result<(f64, f64)> {
    let phase = I::new(leaf[0], leaf[1])?
        .sub(I::point(whole[0]))?
        .div(I::point(whole[1]).sub(I::point(whole[0]))?)?
        .intersect(0., 1.)?;
    let mut upper: f64 = 0.;
    let mut lower: f64 = 0.;
    for (i, value) in values.iter().enumerate() {
        let mut hi = I::point(0.);
        let mut lo = I::point(0.);
        for k in 0..3 {
            let x = I::point(a.control_points[i][k]);
            let chord = x.add(phase.mul(I::point(b.control_points[i][k]).sub(x)?)?)?;
            let d = I::new(value[k][0], value[k][1])?.sub(chord)?;
            let radius = d.lo.abs().max(d.hi.abs());
            hi = hi.add(I::point(radius).mul(I::point(radius))?)?;
            let radius = if d.lo <= 0. && d.hi >= 0. {
                0.
            } else {
                d.lo.abs().min(d.hi.abs())
            };
            lo = lo.add(I::point(radius).mul(I::point(radius))?)?;
        }
        upper = upper.max(hi.hi.max(0.).sqrt().next_up());
        lower = lower.max(lo.lo.max(0.).sqrt().next_down().max(0.));
    }
    Ok((lower, upper))
}
/// The path with its authored axis and normal frame laws.
#[derive(Clone, Copy)]
pub(super) struct ChordFrame<'a> {
    pub path: &'a Curve,
    pub axis: &'a Curve,
    pub normal: &'a Curve,
}
pub(super) fn certify(
    sweep: &Sweep<'_>,
    frame: ChordFrame<'_>,
    coordinates: &[[[f64; 2]; 3]],
    interval: [f64; 2],
    [a, b]: [&Curve; 2],
    spent: &mut usize,
    max: usize,
) -> Result<Option<f64>> {
    let ChordFrame { path, axis, normal } = frame;
    let mut pending = vec![interval];
    let mut upper: f64 = 0.;
    while let Some(leaf) = pending.pop() {
        if *spent == max {
            return Ok(None);
        }
        *spent += 1;
        let value = authored_frame_certificate::certify_control_values(
            path,
            sweep.scale,
            sweep.twist,
            axis,
            normal,
            sweep.affine_laws,
            coordinates,
            leaf,
            max - *spent,
        )?;
        *spent += value.cells;
        let middle = crate::distance_bounds::midpoint(leaf[0], leaf[1]);
        if let Some(values) = value.values.filter(|_| value.status == Status::Certified) {
            let (_, bound) = bounds(&values, leaf, interval, a, b)?;
            if bound <= sweep.options.max_deviation {
                upper = upper.max(bound);
                continue;
            }
            // A source point can stop an unpromising refinement. It supplies
            // NO acceptance credit: the complete leaf value bound above is
            // still retained, even when it exceeds the requested tolerance.
            if *spent == max {
                return Ok(None);
            }
            *spent += 1;
            let point = authored_frame_certificate::certify_control_values(
                path,
                sweep.scale,
                sweep.twist,
                axis,
                normal,
                sweep.affine_laws,
                coordinates,
                [middle, middle],
                max - *spent,
            )?;
            *spent += point.cells;
            if let Some(values) = point.values.filter(|_| point.status == Status::Certified) {
                if bounds(&values, [middle, middle], interval, a, b)?.0
                    > sweep.options.max_deviation
                {
                    upper = upper.max(bound);
                    continue;
                }
            }
        }
        if !(leaf[0] < middle && middle < leaf[1]) {
            return Ok(None);
        }
        pending.push([middle, leaf[1]]);
        pending.push([leaf[0], middle]);
    }
    Ok(Some(upper))
}
