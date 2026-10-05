//! Full-band implicit derivatives of a certified offset-center branch.
//! Tangent regularity is separate from envelope, trim and solid admission.
use crate::{
    Result, check,
    distance_bounds::Interval as I,
    surface::Surface,
    surface_offset::{self, ContactBand},
};
pub struct Report {
    pub contact: ContactBand,
    pub derivatives: Option<[[f64; 2]; 3]>,
    pub tangent: Option<[[f64; 2]; 3]>,
    pub speed: Option<[f64; 2]>,
    pub regular: bool,
    pub reason: &'static str,
}
impl Report {
    pub fn to_value(&self) -> value_codec::Value {
        use value_codec::json;
        let (status, witness) = match &self.contact {
            ContactBand::Excluded => ("excluded", value_codec::Value::Null),
            ContactBand::Unresolved => ("unresolved", value_codec::Value::Null),
            ContactBand::ContinuousBranch(w) => (
                "continuous-branch",
                json!({"firstUV":w.first_uv,"secondUV":w.second_uv,"centerIntervalMm":w.point,"contractionUpper":w.contraction_upper}),
            ),
        };
        json!({"method":"interval-offset-center-tangent","scope":"constant-offset-contact-band",
            "contactStatus":status,"contactWitness":witness,"parameterDerivativeIntervals":self.derivatives,
            "centerTangentIntervalsMm":self.tangent,"speedIntervalMm":self.speed,"centerRegularityProven":self.regular,
            "reason":self.reason,"wholeCurveComplete":false,"envelopeRegularityProven":false,
            "trimMembershipProven":false,"topologyAuthority":false})
    }
}
pub(crate) fn differentiable(s: &Surface, d: [[f64; 2]; 2], order: usize) -> bool {
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let periodic = [s.periodic_u, s.periodic_v];
    for axis in 0..2 {
        let start = knots[axis][degrees[axis]];
        let end = knots[axis][counts[axis]];
        if periodic[axis] && (d[axis][0] == start || d[axis][1] == end) {
            return false;
        }
        let mut i = 0;
        while i < knots[axis].len() {
            let t = knots[axis][i];
            let mut next = i + 1;
            while next < knots[axis].len() && knots[axis][next] == t {
                next += 1
            }
            if t > start
                && t < end
                && t >= d[axis][0]
                && t <= d[axis][1]
                && degrees[axis].saturating_sub(next - i) < order
            {
                return false;
            }
            i = next;
        }
    }
    true
}
fn dot(a: [I; 3], b: [I; 3]) -> Result<I> {
    let mut r = I::point(0.);
    for k in 0..3 {
        r = r.add(a[k].mul(b[k])?)?
    }
    Ok(r)
}
fn cross(a: [I; 3], b: [I; 3]) -> Result<[I; 3]> {
    let mut r = [I::point(0.); 3];
    for k in 0..3 {
        r[k] = a[(k + 1) % 3]
            .mul(b[(k + 2) % 3])?
            .sub(a[(k + 2) % 3].mul(b[(k + 1) % 3])?)?
    }
    Ok(r)
}
fn divide_scalar(a: I, b: f64) -> Result<I> {
    I::new((a.lo / b).next_down(), (a.hi / b).next_up())
}
pub(crate) fn speed(t: [I; 3]) -> Result<I> {
    let scale = t
        .iter()
        .flat_map(|x| [x.lo.abs(), x.hi.abs()])
        .fold(0_f64, f64::max);
    if scale == 0. {
        return Ok(I::point(0.));
    }
    let mut sum = I::point(0.);
    for x in t {
        let x = divide_scalar(x, scale)?;
        let lo = if x.lo > 0. {
            x.lo
        } else if x.hi < 0. {
            -x.hi
        } else {
            0.
        };
        let hi = x.lo.abs().max(x.hi.abs());
        sum = sum.add(I::new((lo * lo).next_down().max(0.), (hi * hi).next_up())?)?
    }
    let root = I::new(
        sum.lo.max(0.).sqrt().next_down().max(0.),
        sum.hi.sqrt().next_up(),
    )?
    .mul(I::point(scale))?;
    I::new(root.lo.max(0.), root.hi)
}
/// Constant signed offsets only. The three free parameter derivatives solve
/// J y' = -F_t; an interval determinant separated from zero bounds the full
/// inverse. Bounds cover the complete certified parameter band, not samples.
pub fn certify(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    fixed_axis: usize,
    fixed_interval: [f64; 2],
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<Report> {
    let contact = surface_offset::certify_contact_band(
        surfaces,
        distances,
        fixed_axis,
        fixed_interval,
        first_other,
        second,
        max_spans,
    )?;
    let mut out = Report {
        contact,
        derivatives: None,
        tangent: None,
        speed: None,
        regular: false,
        reason: "contact-band-unresolved",
    };
    let domains = match &out.contact {
        ContactBand::Excluded => {
            out.reason = "offset-carriers-excluded";
            return Ok(out);
        }
        ContactBand::Unresolved => return Ok(out),
        ContactBand::ContinuousBranch(w) => [w.first_uv, w.second_uv],
    };
    for side in 0..2 {
        if !differentiable(
            surfaces[side],
            domains[side],
            if distances[side] == 0. { 1 } else { 2 },
        ) {
            out.reason = "source-jet-continuity-unproven";
            return Ok(out);
        }
    }
    let a = surface_offset::jacobian_bounds(surfaces[0], domains[0], distances[0], max_spans)?;
    let b = surface_offset::jacobian_bounds(surfaces[1], domains[1], distances[1], max_spans)?;
    let (Some(a), Some(b)) = (a.derivatives, b.derivatives) else {
        out.reason = "offset-jacobian-unresolved";
        return Ok(out);
    };
    let mut aa = [[I::point(0.); 3]; 2];
    let mut bb = aa;
    for axis in 0..2 {
        for k in 0..3 {
            aa[axis][k] = I::new(a[axis][k][0], a[axis][k][1])?;
            bb[axis][k] = I::new(b[axis][k][0], b[axis][k][1])?
        }
    }
    let free = 1 - fixed_axis;
    let mut columns = [aa[free], bb[0], bb[1]];
    let mut rhs = aa[fixed_axis];
    for k in 0..3 {
        for col in [1, 2] {
            columns[col][k] = I::point(0.).sub(columns[col][k])?
        }
        rhs[k] = I::point(0.).sub(rhs[k])?;
        let scale = columns
            .iter()
            .map(|c| c[k])
            .chain(std::iter::once(rhs[k]))
            .flat_map(|x| [x.lo.abs(), x.hi.abs()])
            .fold(0_f64, f64::max);
        if scale == 0. {
            out.reason = "implicit-jacobian-unresolved";
            return Ok(out);
        }
        for col in &mut columns {
            col[k] = divide_scalar(col[k], scale)?
        }
        rhs[k] = divide_scalar(rhs[k], scale)?;
    }
    let det = dot(columns[0], cross(columns[1], columns[2])?)?;
    if det.contains(0.) {
        out.reason = "implicit-jacobian-unresolved";
        return Ok(out);
    }
    let mut derivative = [I::point(0.); 3];
    for k in 0..3 {
        let mut replaced = columns;
        replaced[k] = rhs;
        derivative[k] = dot(replaced[0], cross(replaced[1], replaced[2])?)?.div_signed(det)?
    }
    let mut tangent = [I::point(0.); 3];
    for k in 0..3 {
        let first = aa[fixed_axis][k].add(aa[free][k].mul(derivative[0])?)?;
        let second = bb[0][k]
            .mul(derivative[1])?
            .add(bb[1][k].mul(derivative[2])?)?;
        tangent[k] = I::new(first.lo.max(second.lo), first.hi.min(second.hi))?;
    }
    let speed = speed(tangent)?;
    check(speed.lo >= 0., "Center tangent speed must be nonnegative")?;
    out.regular = speed.lo > 0.;
    out.derivatives = Some(derivative.map(|i| [i.lo, i.hi]));
    out.tangent = Some(tangent.map(|i| [i.lo, i.hi]));
    out.speed = Some([speed.lo, speed.hi]);
    out.reason = if out.regular {
        "regular-center-tangent"
    } else {
        "center-regularity-unproven"
    };
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn assert_contains(r: [f64; 2], x: f64) {
        assert!(r[0] <= x && x <= r[1], "{r:?} misses {x}")
    }
    #[test]
    fn moving_plane_contacts_have_full_band_derivatives() {
        let a = plane();
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                p[1] = 0.5;
                p[2] = z
            }
        }
        let r = certify(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
        )
        .unwrap();
        assert!(r.regular, "{}", r.reason);
        for (i, v) in [0., 1., 0.].into_iter().enumerate() {
            assert_contains(r.derivatives.unwrap()[i], v)
        }
        for (i, v) in [1., 0., 0.].into_iter().enumerate() {
            assert_contains(r.tangent.unwrap()[i], v)
        }
        assert_contains(r.speed.unwrap(), 1.);
        let r = certify(
            [&a, &a],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.25, 0.35]],
            2,
        )
        .unwrap();
        assert!(!r.regular);
        assert!(r.tangent.is_none());
    }
    #[test]
    fn curved_rational_cylinder_plane_branch_has_uniform_tangent_bounds_in_rotated_frames() {
        let a = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![[3., 0.], [3., 3.], [0., 3.]]
                .into_iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
                .collect(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
                .into_iter()
                .map(|w| vec![w; 2])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                let y = 4. * p[0];
                let z = 5. * p[1];
                *p = vec![2. - z, y, z];
            }
        }
        let x = 2. + 0.2 * 2_f64.sqrt() - 0.5;
        let y = (3.2_f64.powi(2) - x * x).sqrt();
        let mut u = 0.5;
        for _ in 0..8 {
            let e = surface_offset::evaluate(&a, [u, 0.1], 0.2).unwrap();
            u -= (e.point[0] - x) / e.du[0];
        }
        let bu = y / 4.;
        let bv = (0.5 - 0.2 / 2_f64.sqrt()) / 5.;
        for rotated in [false, true] {
            let mut aa = a.clone();
            let mut bb = b.clone();
            if rotated {
                for s in [&mut aa, &mut bb] {
                    for row in &mut s.control_points {
                        for p in row {
                            *p = vec![p[2] + 17., p[0] - 9., p[1] + 23.];
                        }
                    }
                }
            }
            let r = certify(
                [&aa, &bb],
                [0.2, 0.2],
                1,
                [0.099999, 0.100001],
                [u - 1e-4, u + 1e-4],
                [[bu - 1e-4, bu + 1e-4], [bv - 1e-4, bv + 1e-4]],
                2,
            )
            .unwrap();
            assert!(r.regular, "{}", r.reason);
            for t in [0.099999, 0.1, 0.100001] {
                let x = 2. + 0.2 * 2_f64.sqrt() - 5. * t;
                let y = (3.2_f64.powi(2) - x * x).sqrt();
                let tangent = if rotated {
                    [5., -5., 5. * x / y]
                } else {
                    [-5., 5. * x / y, 5.]
                };
                for k in 0..3 {
                    assert_contains(r.tangent.unwrap()[k], tangent[k]);
                }
                assert_contains(
                    r.speed.unwrap(),
                    tangent.iter().map(|x| x * x).sum::<f64>().sqrt(),
                );
                assert_contains(r.derivatives.unwrap()[1], 5. * x / y / 4.);
                assert_contains(r.derivatives.unwrap()[2], 1.);
            }
        }
    }
    #[test]
    fn a_unique_contact_band_does_not_prove_a_regular_centerline() {
        let a = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![[3., 0.], [3., 3.], [0., 3.]]
                .into_iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
                .collect(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
                .into_iter()
                .map(|w| vec![w; 2])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                *p = vec![p[0] - 0.5, p[1] - 0.5, 1.65]
            }
        }
        let r = certify(
            [&a, &b],
            [-3., 0.2],
            0,
            [0.299999, 0.300001],
            [0.369, 0.371],
            [[0.49, 0.51], [0.49, 0.51]],
            2,
        )
        .unwrap();
        assert!(
            matches!(r.contact, ContactBand::ContinuousBranch(_)),
            "{}",
            r.reason
        );
        assert!(!r.regular);
        assert_eq!(r.reason, "center-regularity-unproven");
        assert_contains(r.speed.unwrap(), 0.);
    }
    #[test]
    fn periodic_start_seams_cannot_admit_an_undefined_offset_normal() {
        let mut a = plane();
        a.control_points.push(a.control_points[0].clone());
        a.weights.push(vec![1.; 2]);
        a.knots_u = vec![-1., 0., 1., 2., 3.];
        a.periodic_u = true;
        a.validate().unwrap();
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                *p = vec![p[0] - 0.5, 0.5, z];
            }
        }
        assert!(surface_offset::evaluate(&a, [0., 0.3], 0.2).is_err());
        let r = certify(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0., 1e-6],
            [0.25, 0.35],
            [[0.49, 0.51], [0.15, 0.25]],
            2,
        )
        .unwrap();
        assert!(!r.regular);
        assert!(matches!(r.contact, ContactBand::Unresolved));
    }
    #[test]
    fn source_continuity_is_required_for_implicit_tangents() {
        let mut s = plane();
        s.control_points
            .push(vec![vec![2., 0., 0.], vec![2., 1., 0.]]);
        s.weights.push(vec![1.; 2]);
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        assert!(!differentiable(&s, [[0.4, 0.6], [0.2, 0.3]], 1));
        assert!(differentiable(&s, [[0.1, 0.4], [0.2, 0.3]], 2));
    }
}
