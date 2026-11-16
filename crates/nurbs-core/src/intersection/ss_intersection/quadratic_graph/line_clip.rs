//! Choose active rectangle constraints with exact fraction comparisons. Do not
//! divide inactive bounds: their non-dyadic clocks need not describe an endpoint.
use super::*;
#[derive(Clone, Copy)]
struct Bound {
    numerator: f64,
    denominator: f64,
    axis: usize,
    boundary: f64,
}
fn compare(a: Bound, b: Bound, w: &mut Work) -> Result<std::cmp::Ordering> {
    let left = w.mul(a.numerator, b.denominator)?;
    let right = w.mul(b.numerator, a.denominator)?;
    Ok(left.partial_cmp(&right).unwrap())
}
fn point(anchor: [f64; 2], direction: [f64; 2], bound: Bound, w: &mut Work) -> Result<[f64; 2]> {
    let a = bound.axis;
    let b = 1 - a;
    let delta = w.add(bound.boundary, -anchor[a])?;
    let product = w.mul(delta, direction[b])?;
    let offset = w.div(product, direction[a])?;
    let mut point = anchor;
    point[a] = bound.boundary;
    point[b] = w.add(anchor[b], offset)?;
    check(
        point.iter().all(|x| (0. ..=1.).contains(x)),
        "Clipped exact line leaves its original rectangle",
    )?;
    Ok(point)
}
pub(super) fn clip(
    anchor: [f64; 2],
    direction: [f64; 2],
    w: &mut Work,
) -> Result<Option<[[f64; 2]; 2]>> {
    check(
        direction != [0.; 2],
        "Degenerate original graph factor direction",
    )?;
    let mut low: Option<Bound> = None;
    let mut high: Option<Bound> = None;
    for axis in 0..2 {
        let d = direction[axis];
        if d == 0. {
            if !(0. ..=1.).contains(&anchor[axis]) {
                return Ok(None);
            }
            continue;
        }
        let boundaries = if d > 0. { [0., 1.] } else { [1., 0.] };
        let mut bounds = Vec::with_capacity(2);
        for boundary in boundaries {
            let delta = w.add(boundary, -anchor[axis])?;
            bounds.push(Bound {
                numerator: if d > 0. { delta } else { -delta },
                denominator: d.abs(),
                axis,
                boundary,
            });
        }
        if low.is_none() || compare(bounds[0], low.unwrap(), w)?.is_gt() {
            low = Some(bounds[0])
        }
        if high.is_none() || compare(bounds[1], high.unwrap(), w)?.is_lt() {
            high = Some(bounds[1])
        }
    }
    let (low, high) = (low.unwrap(), high.unwrap());
    if compare(low, high, w)?.is_gt() {
        return Ok(None);
    }
    Ok(Some([
        point(anchor, direction, low, w)?,
        point(anchor, direction, high, w)?,
    ]))
}
pub(super) fn vector(data: &Data, diagonal: [f64; 2], w: &mut Work) -> Result<[f64; 2]> {
    let mut out = [0.; 2];
    for axis in 0..2 {
        let a = w.mul(data.basis[axis][0], diagonal[0])?;
        let b = w.mul(data.basis[axis][1], diagonal[1])?;
        out[axis] = w.add(a, b)?;
    }
    Ok(out)
}

pub(super) fn transverse(data: &mut Data, slope: [f64; 2], constant: f64, w: &mut Work) -> Result<()> {
    let mut points = Vec::new();
    for axis in 0..2 {
        let varying = 1 - axis;
        for side in [0., 1.] {
            let start = if side == 0. { constant } else { w.add(constant, slope[axis])? };
            let end = w.add(start, slope[varying])?;
            let mut point = [0.; 2];
            point[axis] = side;
            if start == 0. {
                points.push(point);
            }
            if end == 0. {
                point[varying] = 1.;
                points.push(point);
            }
            if start != 0. && end != 0. && start.is_sign_negative() != end.is_sign_negative() {
                point[varying] = w.div(-start, slope[varying])?;
                points.push(point);
            }
        }
    }
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    match points.as_slice() {
        [] => {}
        [point] => {
            data.kind = QuadraticGraphSSKind::IsolatedBoundaryPoint;
            data.vertices.push(OriginalGraphSSVertex {
                point: OriginalGraphSSPoint::Normalized(*point),
                singular: false,
            });
        }
        &[first, second] => {
            data.kind = QuadraticGraphSSKind::TransverseLine;
            segment(data, [first, second], 1);
        }
        _ => return Err(crate::input("Affine gap has inconsistent rectangle ports")),
    }
    Ok(())
}
pub(super) fn double(data: &mut Data, w: &mut Work) -> Result<()> {
    let axis = usize::from(data.diagonal[0] == 0.);
    let mut diagonal = [0.; 2];
    diagonal[1 - axis] = 1.;
    let direction = vector(data, diagonal, w)?;
    if let Some(points) = clip(data.center, direction, w)? {
        if points[0] == points[1] {
            data.kind = QuadraticGraphSSKind::IsolatedTangentPoint;
            data.vertices.push(OriginalGraphSSVertex {
                point: OriginalGraphSSPoint::Normalized(points[0]),
                singular: true,
            });
        } else {
            data.kind = QuadraticGraphSSKind::TangentDoubleLine;
            segment(data, points, 2);
            for vertex in &mut data.vertices {
                vertex.singular = true;
            }
        }
    }
    Ok(())
}
