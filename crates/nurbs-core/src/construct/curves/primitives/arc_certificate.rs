//! Inclusive original circle-arc family error in homogeneous coordinates.
use crate::{
    Result, check,
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
};
#[derive(Clone, Debug)]
pub struct ArcCertificate {
    pub error_upper: f64,
    pub homogeneous_difference_upper: f64,
    pub weight_difference_upper: f64,
    pub rounding_included: bool,
}
/// Actual positive rational quadratic versus every nominal XY circle arc in
/// independently justified radius/start/end boxes (radians). Source domain must
/// be normalized and clamped. The arc's nominal center is exact input data.
/// Includes saved poles, weights and supplied parameter uncertainty; sampling
/// and platform trigonometric values grant no authority. Not an embedding proof.
pub fn certify_circle_arc_radians(
    c: &Curve,
    center: [f64; 3],
    radius: [f64; 2],
    start: [f64; 2],
    end: [f64; 2],
) -> Result<ArcCertificate> {
    c.validate()?;
    check(
        center.iter().all(|x| x.is_finite())
            && matches!(c.control_points[0].len(), 2 | 3)
            && c.degree == 2
            && c.control_points.len() == 3
            && c.knots == [0., 0., 0., 1., 1., 1.]
            && !c.periodic,
        "Circle arc certificate requires a normalized clamped 2D/3D rational quadratic",
    )?;
    let r = I::new(radius[0], radius[1])?;
    let a = I::new(start[0], start[1])?;
    let b = I::new(end[0], end[1])?;
    check(
        r.lo > 0. && (a.hi < b.lo || b.hi < a.lo),
        "Nominal radius must be positive and endpoint angle boxes separated",
    )?;
    let half = b.sub(a)?.div(I::point(2.))?;
    let mid = a.add(b)?.div(I::point(2.))?;
    let (_, w) = crate::involute::certified_trig(half)?;
    check(
        w.lo > 0. && half.lo > -3. && half.hi < 3.,
        "Circle arc family must remain shorter than a half turn",
    )?;
    let ideal_weights = [I::point(1.), w, I::point(1.)];
    let mut difference = 0_f64;
    let mut weight_difference = 0_f64;
    for (i, angle) in [a, mid, b].into_iter().enumerate() {
        let (sin, cos) = crate::involute::certified_trig(angle)?;
        let ideal = [r.mul(cos)?, r.mul(sin)?, I::point(0.)];
        let stored: Vec<_> = (0..3)
            .map(|k| {
                let coordinate = c.control_points[i].get(k).copied().unwrap_or(0.);
                I::point(coordinate)
                    .sub(I::point(center[k]))?
                    .mul(I::point(c.weights[i]))
            })
            .collect::<Result<_>>()?;
        difference = difference.max(box_distance(&stored, &ideal)?.1);
        let d = I::point(c.weights[i]).sub(ideal_weights[i])?;
        weight_difference = weight_difference.max(d.lo.abs().max(d.hi.abs()));
    }
    let actual_min = c.weights.iter().copied().fold(f64::INFINITY, f64::min);
    let ideal_min = w.lo.min(1.);
    let a = I::point(actual_min);
    let denominator = a.mul(I::point(ideal_min))?;
    let error = I::point(difference)
        .div(a)?
        .add(r.mul(I::point(weight_difference))?.div(denominator)?)?;
    Ok(ArcCertificate {
        error_upper: error.hi,
        homogeneous_difference_upper: difference,
        weight_difference_upper: weight_difference,
        rounding_included: true,
    })
}

#[derive(Clone, Debug)]
pub struct RadialSegmentCertificate {
    pub error_upper: f64,
    pub rounding_included: bool,
}
/// Actual clamped constant-weight segment versus every nominal radial segment
/// at one angle (radians), with independently justified endpoint-radius boxes.
/// A maximum endpoint discrepancy bounds the full linear interpolation.
pub fn certify_radial_segment_radians(
    c: &Curve,
    center: [f64; 3],
    from: [f64; 2],
    to: [f64; 2],
    angle: [f64; 2],
) -> Result<RadialSegmentCertificate> {
    c.validate()?;
    check(
        center.iter().all(|x| x.is_finite())
            && matches!(c.control_points[0].len(), 2 | 3)
            && c.degree == 1
            && c.control_points.len() == 2
            && c.knots == [0., 0., 1., 1.]
            && !c.periodic
            && c.weights[0] == c.weights[1],
        "Radial segment certificate requires a normalized constant-weight 2D/3D line",
    )?;
    let angle = I::new(angle[0], angle[1])?;
    let (sin, cos) = crate::involute::certified_trig(angle)?;
    let mut maximum = 0_f64;
    for (i, radius) in [from, to].into_iter().enumerate() {
        let r = I::new(radius[0], radius[1])?;
        check(
            r.lo > 0.,
            "Radial segment nominal radii must remain positive",
        )?;
        let ideal = [
            I::point(center[0]).add(r.mul(cos)?)?,
            I::point(center[1]).add(r.mul(sin)?)?,
            I::point(center[2]),
        ];
        let stored: Vec<_> = (0..3)
            .map(|k| I::point(c.control_points[i].get(k).copied().unwrap_or(0.)))
            .collect();
        maximum = maximum.max(box_distance(&stored, &ideal)?.1);
    }
    Ok(RadialSegmentCertificate {
        error_upper: maximum,
        rounding_included: true,
    })
}
