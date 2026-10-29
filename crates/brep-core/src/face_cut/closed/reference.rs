//! Compare every authored orientation directly with its immutable saved curve.
use super::*;
use num_rational::BigRational as Q;
pub(super) fn exact_reversal(candidate: &Curve, source: &Curve) -> bool {
    if candidate.degree != source.degree
        || candidate.domain() != source.domain()
        || candidate.knots.len() != source.knots.len()
        || candidate.control_points.len() != source.control_points.len()
        || candidate.weights.len() != source.weights.len()
        || candidate.periodic != source.periodic
        || !candidate
            .control_points
            .iter()
            .eq(source.control_points.iter().rev())
        || !candidate.weights.iter().eq(source.weights.iter().rev())
    {
        return false;
    }
    let [a, b] = source.domain();
    let center = Q::from_float(a).unwrap() + Q::from_float(b).unwrap();
    candidate
        .knots
        .iter()
        .zip(source.knots.iter().rev())
        .all(|(&new, &old)| {
            if old == a {
                new == b
            } else if old == b {
                new == a
            } else {
                Q::from_float(new).unwrap() == &center - Q::from_float(old).unwrap()
            }
        })
}
fn embed(c: &Curve) -> Curve {
    let mut c = c.clone();
    if c.control_points[0].len() == 2 {
        for p in &mut c.control_points {
            p.push(0.);
        }
    }
    c
}
/// True proves exact saved-clock identity. False proves only the explicit
/// tolerance. A rounded reverse is never accepted from its surface lift alone.
pub(super) fn reference(
    candidate: &Curve,
    source: &Curve,
    reverse: bool,
    tolerance: f64,
    w: &mut Work,
    l: ClosedCutLimits,
) -> Result<bool> {
    if (!reverse && candidate == source) || (reverse && exact_reversal(candidate, source)) {
        return Ok(true);
    }
    let original = embed(source);
    let actual = embed(candidate);
    let [a, b] = source.domain();
    let support = nurbs_core::surface::Surface {
        degree_u: original.degree,
        degree_v: 1,
        knots_u: original.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: original
            .control_points
            .iter()
            .map(|p| vec![p.clone(), p.clone()])
            .collect(),
        weights: original.weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let pc = if reverse {
        crate::line(vec![b, 0.], vec![a, 0.])
    } else {
        crate::line(vec![a, 0.], vec![b, 0.])
    };
    lift(&actual, &pc, &support, false, tolerance, w, l)?;
    Ok(false)
}
