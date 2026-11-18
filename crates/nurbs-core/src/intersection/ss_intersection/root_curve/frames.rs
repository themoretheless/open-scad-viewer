use super::*;
use num_traits::ToPrimitive;
fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::input("Nonfinite original root coordinate"))
}
fn outward(x: Q) -> Result<[f64; 2]> {
    let value = x
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| crate::input("Original root coordinate overflow"))?;
    let exact = q(value)?;
    Ok([
        if exact > x { value.next_down() } else { value },
        if exact < x { value.next_up() } else { value },
    ])
}
pub(super) fn uv(curve: &OriginalSsRootCurve, local: [[f64; 2]; 4]) -> Result<[[f64; 2]; 4]> {
    let spans = match &curve.carrier {
        Carrier::Deflated(f) => Some(f.source().original().original().spans()),
        Carrier::Pole(f) => Some(f.source().original().original().original().spans()),
        Carrier::Blowup(f) => Some(f.source().node().source().original().original().spans()),
        _ => None,
    };
    let Some(spans) = spans else { return Ok(local) };
    let source = curve.source();
    let mut out = [[0.; 2]; 4];
    for k in 0..4 {
        let s = if k < 2 {
            source.first()
        } else {
            source.second()
        };
        let knots = if k % 2 == 0 { &s.knots_u } else { &s.knots_v };
        let lo = q(knots[spans[k]])?;
        let width = q(knots[spans[k] + 1])? - &lo;
        out[k] = [
            outward(&lo + &width * q(local[k][0])?)?[0].max(knots[spans[k]]),
            outward(&lo + &width * q(local[k][1])?)?[1].min(knots[spans[k] + 1]),
        ];
    }
    Ok(out)
}
pub(super) fn world(
    curve: &OriginalSsRootCurve,
    uv: [[f64; 2]; 4],
    max: usize,
) -> Result<([[f64; 2]; 3], usize)> {
    let s = curve.source().first();
    let us = super::super::fiber::original_axis_sections(s, 0, uv[0])?;
    let vs = super::super::fiber::original_axis_sections(s, 1, uv[1])?;
    let mut out = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut used = 0;
    for u in us {
        for v in &vs {
            for i in s.degree_u..s.control_points.len() {
                if s.knots_u[i] == s.knots_u[i + 1] {
                    continue;
                }
                for j in s.degree_v..s.control_points[0].len() {
                    if s.knots_v[j] == s.knots_v[j + 1] {
                        continue;
                    }
                    let d = [
                        [u[0].max(s.knots_u[i]), u[1].min(s.knots_u[i + 1])],
                        [v[0].max(s.knots_v[j]), v[1].min(s.knots_v[j + 1])],
                    ];
                    if d.iter().any(|d| d[0] > d[1]) {
                        continue;
                    }
                    if used == max {
                        return Err(crate::resource(
                            "Original root world enclosure budget exhausted",
                        ));
                    }
                    used += 1;
                    let bounds = crate::surface_distance::enclosure(s, [i, j], d)?;
                    for k in 0..3 {
                        out[k][0] = out[k][0].min(bounds[k].lo);
                        out[k][1] = out[k][1].max(bounds[k].hi);
                    }
                }
            }
        }
    }
    check(
        used > 0
            && out
                .iter()
                .all(|d| d.iter().all(|x| x.is_finite()) && d[0] <= d[1]),
        "Original root world enclosure is unproved",
    )?;
    Ok((out, used))
}
pub(super) fn frame(curve: &OriginalSsRootCurve) -> Result<[[Q; 2]; 4]> {
    check(!matches!(curve.carrier,Carrier::Blowup(_)),"Nonlinear blowdown cannot use affine endpoint membership")?;
    let spans = match &curve.carrier {
        Carrier::Deflated(f) => Some(f.source().original().original().spans()),
        Carrier::Pole(f) => Some(f.source().original().original().original().spans()),
        Carrier::Blowup(f) => Some(f.source().node().source().original().original().spans()),
        _ => None,
    };
    let Some(spans) = spans else {
        return Ok(std::array::from_fn(|_| [Q::zero(), Q::one()]));
    };
    let source = curve.source();
    let mut out = std::array::from_fn(|_| [Q::zero(), Q::one()]);
    for k in 0..4 {
        let s = if k < 2 {
            source.first()
        } else {
            source.second()
        };
        let knots = if k % 2 == 0 { &s.knots_u } else { &s.knots_v };
        out[k] = [
            q(knots[spans[k]])?,
            q(knots[spans[k] + 1])? - q(knots[spans[k]])?,
        ];
    }
    Ok(out)
}
