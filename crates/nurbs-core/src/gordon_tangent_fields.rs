//! Candidate rational boundary derivative fields for Cartesian tangent auditing.
//! Extraction/composition rounds in binary64; these curves are not exact certificates.
use super::denominators::product;
use crate::{
    Result, check,
    curve::Curve,
    surface::{Axis, Surface},
};

/// Rational quotient rule: dP/dv = (X_v W-X W_v)/W².
/// Returns one Bezier curve per U span, with the authored U domain.
pub(super) fn boundary_v_derivative(surface: &Surface, end: bool) -> Result<Vec<Curve>> {
    surface.validate()?;
    check(
        !surface.periodic_u && !surface.periodic_v,
        "Boundary tangent extraction requires open surface covers",
    )?;
    let va = surface.knots_v[surface.degree_v];
    let vb = surface.knots_v[surface.control_points[0].len()];
    let clamped = surface.edit_axis(Axis::V, |c| c.trim(va, vb))?;
    let degree = clamped.degree_u;
    if 2 * degree > 25 {
        return Err(crate::resource(
            "Boundary derivative field exceeds degree 25",
        ));
    }
    let mut breaks: Vec<_> = clamped
        .knots_u
        .iter()
        .copied()
        .filter(|u| {
            *u >= clamped.knots_u[degree] && *u <= clamped.knots_u[clamped.control_points.len()]
        })
        .collect();
    breaks.dedup();
    let mut result = Vec::new();
    for span in breaks.windows(2) {
        let patch = clamped.edit_axis(Axis::U, |c| c.trim(span[0], span[1]))?;
        check(
            patch.control_points.len() == degree + 1,
            "Tangent extraction did not isolate a U Bezier span",
        )?;
        let nv = patch.control_points[0].len();
        let (owner, neighbor, extent, sign) = if end {
            (nv - 1, nv - 2, vb - patch.knots_v[nv - 1], -1.)
        } else {
            (0, 1, patch.knots_v[patch.degree_v + 1] - va, 1.)
        };
        let factor = sign * patch.degree_v as f64 / extent;
        let scale = patch.weights.iter().flatten().copied().fold(0., f64::max);
        let w: Vec<_> = patch.weights.iter().map(|row| row[owner] / scale).collect();
        let dw: Vec<_> = patch
            .weights
            .iter()
            .map(|row| factor * (row[neighbor] / scale - row[owner] / scale))
            .collect();
        let weights = product(&w, &w);
        let mut controls = vec![vec![0.; 3]; weights.len()];
        for k in 0..3 {
            let x: Vec<_> = (0..=degree)
                .map(|i| patch.control_points[i][owner][k] * w[i])
                .collect();
            let dx: Vec<_> = (0..=degree)
                .map(|i| {
                    factor
                        * (patch.control_points[i][neighbor][k] * patch.weights[i][neighbor]
                            / scale
                            - x[i])
                })
                .collect();
            let a = product(&dx, &w);
            let b = product(&x, &dw);
            for i in 0..weights.len() {
                controls[i][k] = (a[i] - b[i]) / weights[i];
            }
        }
        let mut knots = vec![span[0]; 2 * degree + 1];
        knots.extend(vec![span[1]; 2 * degree + 1]);
        let field = Curve {
            degree: 2 * degree,
            knots,
            control_points: controls,
            weights,
            periodic: false,
        };
        field.validate()?;
        result.push(field);
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_boundary_derivative_includes_weight_derivative_and_authored_scale() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![2., 2., 7., 7.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 1.]],
                vec![vec![1., 0., 0.], vec![1., 0., 1.]],
            ],
            weights: vec![vec![1., 2.], vec![3., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        for candidate in [
            surface.clone(),
            surface.edit_axis(Axis::U, |c| c.insert(0.3, 1)).unwrap(),
        ] {
            for end in [false, true] {
                let fields = boundary_v_derivative(&candidate, end).unwrap();
                assert_eq!(fields.len(), candidate.control_points.len() - 1);
                let certificate =
                    super::super::tangent_audit::verify(&surface, &fields, end, 1e-10, 1000)
                        .unwrap();
                assert_eq!(certificate["accepted"], true);
                #[cfg(feature = "transport")]
                {
                    let json_certificate=crate::transport::dispatch(value_codec::json!({
                        "op":"surface_boundary_v_tangent_certify","surface":surface,"targets":fields,
                        "end":end,"tolerance":1e-10,"maxCells":1000})).unwrap();
                    assert_eq!(json_certificate["accepted"], true);
                    assert_eq!(json_certificate["exact"], false);
                }

                if fields.len() == 1 {
                    let mut distorted = fields[0].elevate(3).unwrap();
                    distorted.control_points[1][0] += 0.01 / distorted.weights[1];
                    distorted.control_points[2][0] -= 0.01 / distorted.weights[2];
                    for u in [0., 0.5, 1.] {
                        let original = fields[0].evaluate(u).unwrap().point;
                        let altered = distorted.evaluate(u).unwrap().point;
                        assert!((original[0] - altered[0]).abs() < 1e-12);
                    }
                    let rejected = super::super::tangent_audit::verify(
                        &surface,
                        &[distorted],
                        end,
                        1e-10,
                        100,
                    )
                    .unwrap();
                    assert_eq!(rejected["accepted"], false);
                    assert_eq!(rejected["errorUpper"], value_codec::Value::Null);
                }

                if fields.len() > 1 {
                    let incomplete =
                        super::super::tangent_audit::verify(&candidate, &fields, end, 1e-10, 1)
                            .unwrap();
                    assert_eq!(incomplete["accepted"], false);
                    assert_eq!(incomplete["errorUpper"], value_codec::Value::Null);
                }

                for i in 0..=1000 {
                    let u = i as f64 / 1000.;
                    let expected = surface
                        .evaluate(u, if end { 7. } else { 2. })
                        .unwrap()
                        .first_derivatives()
                        .unwrap()
                        .1;
                    let field = fields
                        .iter()
                        .find(|c| c.domain()[0] <= u && c.domain()[1] >= u)
                        .unwrap();
                    let actual = field.evaluate(u).unwrap().point;
                    for k in 0..3 {
                        assert!((actual[k] - expected[k]).abs() < 1e-12);
                    }
                }
            }
        }
        assert!(
            surface
                .evaluate(0.5, 2.)
                .unwrap()
                .first_derivatives()
                .unwrap()
                .1[0]
                .abs()
                > 0.01
        );
    }
}
