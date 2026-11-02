//! Authored lift integers refer to literal evaluated UV scalars and exact real
//! arithmetic on the original binary64 chart ends, never a rounded quotient.
use crate::{Error, Result};
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
pub(crate) fn periodic_lifts(
    s: &nurbs_core::surface::Surface,
    p: &nurbs_core::curve::Curve,
    parameters: [f64; 2],
) -> Result<[[i32; 2]; 2]> {
    s.validate()?;
    let d = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut out = [[0; 2]; 2];
    for i in 0..2 {
        let uv = p.evaluate(parameters[i])?.point;
        if uv.len() != 2 {
            return Err(Error::new(
                "BREP_TRIM_DOMAIN",
                "Periodic lift needs a two-dimensional pcurve",
            ));
        }
        for k in 0..2 {
            if [s.periodic_u, s.periodic_v][k] {
                let q = |v| Q::from_float(v).unwrap();
                out[i][k] = ((q(uv[k]) - q(d[k][0])) / (q(d[k][1]) - q(d[k][0])))
                    .floor()
                    .to_i32()
                    .ok_or_else(|| {
                        Error::new(
                            "BREP_TRIM_PERIODIC_LIFT",
                            "Authored periodic lift is outside its integer representation",
                        )
                    })?;
            }
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn surface() -> nurbs_core::surface::Surface {
        let profile = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-1.5, -1., -0.5, 0., 0.5, 1., 1.5],
            knots_v: vec![0., 0., 1., 1.],
            control_points: profile
                .iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
                .collect(),
            weights: vec![vec![1., 1.]; 5],
            periodic_u: true,
            periodic_v: false,
        }
    }
    fn pc(u: f64) -> nurbs_core::curve::Curve {
        nurbs_core::curve::Curve::from_polyline(vec![vec![u, 0.], vec![u, 1.]]).unwrap()
    }
    #[test]
    fn a_rounded_unit_quotient_cannot_move_a_point_to_the_next_sheet() {
        let u = 1f64.next_down();
        assert_eq!(((u + 1.) / 2.).floor(), 1.);
        assert_eq!(
            periodic_lifts(&surface(), &pc(u), [0., 1.]).unwrap(),
            [[0, 0]; 2]
        );
        assert_eq!(
            periodic_lifts(&surface(), &pc(1.), [0., 1.]).unwrap(),
            [[1, 0]; 2]
        );
        assert_eq!(
            periodic_lifts(&surface(), &pc((-1f64).next_down()), [0., 1.]).unwrap(),
            [[-1, 0]; 2]
        );
    }
    #[test]
    fn signed_sheets_and_unrepresentable_integers_are_checked() {
        assert_eq!(
            periodic_lifts(&surface(), &pc(5.25), [0., 1.]).unwrap(),
            [[3, 0]; 2]
        );
        assert_eq!(
            periodic_lifts(&surface(), &pc(-5.25), [0., 1.]).unwrap(),
            [[-3, 0]; 2]
        );
        let mut s = surface();
        for k in &mut s.knots_u {
            *k *= 0.125;
        }
        assert!(periodic_lifts(&s, &pc(1e9), [0., 1.]).is_err());
    }
}
