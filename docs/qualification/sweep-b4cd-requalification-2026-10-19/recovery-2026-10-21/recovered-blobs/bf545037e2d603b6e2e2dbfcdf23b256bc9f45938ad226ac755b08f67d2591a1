//! Whole-curve error between a retained rational spatial boundary and the
//! bilinear cap composed with its rational UV curve. Domain/embedding are separate.
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I, surface::Surface};
#[derive(Clone, Debug)]
pub struct Report {
    pub within_budget: bool,
    pub error_upper: Option<f64>,
    pub products: usize,
    pub reason: Option<&'static str>,
}
/// Matching positive rational bases define R_i(t) summing to one. Expanding
/// the bilinear composition minus the source in symmetric R_i*R_j products
/// makes the full error a convex combination of outward coefficient boxes.
/// All products must be covered before any upper bound is returned.
pub fn inspect(
    s: &Surface,
    world: &Curve,
    uv: &Curve,
    tolerance: f64,
    max_products: usize,
) -> Result<Report> {
    s.validate()?;
    world.validate()?;
    uv.validate()?;
    check(
        tolerance.is_finite() && tolerance >= 0. && max_products <= 100000,
        "Invalid cap boundary budget",
    )?;
    check(
        s.degree_u == 1
            && s.degree_v == 1
            && s.knots_u.len() == 4
            && s.knots_u[0] == s.knots_u[1]
            && s.knots_u[2] == s.knots_u[3]
            && s.knots_v.len() == 4
            && s.knots_v[0] == s.knots_v[1]
            && s.knots_v[2] == s.knots_v[3]
            && s.control_points.len() == 2
            && s.control_points
                .iter()
                .all(|r| r.len() == 2 && r.iter().all(|p| p.len() == 3))
            && s.weights.iter().flatten().all(|w| *w == 1.)
            && !s.periodic_u
            && !s.periodic_v,
        "Cap certificate requires a clamped bilinear nonrational surface",
    )?;
    let domain = [[s.knots_u[1], s.knots_u[2]], [s.knots_v[1], s.knots_v[2]]];
    check(
        world.degree == uv.degree
            && world.knots == uv.knots
            && world.weights == uv.weights
            && world.periodic == uv.periodic
            && world.control_points.len() == uv.control_points.len()
            && world.control_points.iter().all(|p| p.len() == 3)
            && uv.control_points.iter().all(|p| {
                p.len() == 2 && (0..2).all(|k| p[k] >= domain[k][0] && p[k] <= domain[k][1])
            }),
        "Cap boundary requires matching rational bases and bounded UV controls",
    )?;
    let n = world.control_points.len();
    let mut out = Report {
        within_budget: false,
        error_upper: None,
        products: 0,
        reason: Some("cap-product-budget-exhausted"),
    };
    if n.checked_mul(n).is_none_or(|count| count > max_products) {
        return Ok(out);
    }
    let bound = (|| -> Result<f64> {
        let normalized = uv
            .control_points
            .iter()
            .map(|p| {
                (0..2)
                    .map(|k| {
                        I::point(p[k])
                            .sub(I::point(domain[k][0]))?
                            .div(I::point(domain[k][1]).sub(I::point(domain[k][0]))?)
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let mut radii = [0_f64; 3];
        for k in 0..3 {
            let a = I::point(s.control_points[0][0][k]);
            let b = I::point(s.control_points[1][0][k]).sub(a)?;
            let c = I::point(s.control_points[0][1][k]).sub(a)?;
            let d = I::point(s.control_points[1][1][k]).sub(a)?.sub(b)?.sub(c)?;
            for i in 0..n {
                for j in 0..n {
                    let u = normalized[i][0].add(normalized[j][0])?.mul(I::point(0.5))?;
                    let v = normalized[i][1].add(normalized[j][1])?.mul(I::point(0.5))?;
                    let mixed = normalized[i][0]
                        .mul(normalized[j][1])?
                        .add(normalized[j][0].mul(normalized[i][1])?)?
                        .mul(I::point(0.5))?;
                    let source = I::point(world.control_points[i][k])
                        .add(I::point(world.control_points[j][k]))?
                        .mul(I::point(0.5))?;
                    let error = a
                        .add(b.mul(u)?)?
                        .add(c.mul(v)?)?
                        .add(d.mul(mixed)?)?
                        .sub(source)?;
                    radii[k] = radii[k].max(error.lo.abs().max(error.hi.abs()));
                }
            }
        }
        let mut square = I::point(0.);
        for radius in radii {
            square = square.add(I::point(radius).mul(I::point(radius))?)?;
        }
        let upper = square.hi.sqrt().next_up();
        crate::numeric(upper.is_finite(), "Cap norm overflow")?;
        Ok(upper)
    })();
    out.products = n * n;
    match bound {
        Ok(upper) => {
            out.error_upper = Some(upper);
            out.within_budget = upper <= tolerance;
            out.reason = if out.within_budget {
                None
            } else {
                Some("cap-boundary-error-exceeds-budget")
            };
        }
        Err(_) => out.reason = Some("cap-boundary-enclosure-unresolved"),
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_composition_and_refusals_cover_whole_boundary() {
        let s = Surface {
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
        };
        let world = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.1, 0.2, 0.], vec![0.5, 0.8, 0.], vec![0.9, 0.2, 0.]],
            weights: vec![1., 2., 1.],
            periodic: false,
        };
        let mut uv = world.clone();
        uv.control_points = world
            .control_points
            .iter()
            .map(|p| p[..2].to_vec())
            .collect();
        assert!(inspect(&s, &world, &uv, 1e-12, 9).unwrap().within_budget);
        let mut natural = s.clone();
        natural.knots_u = vec![-1., -1., 1., 1.];
        natural.knots_v = vec![-1., -1., 1., 1.];
        natural.control_points = vec![
            vec![vec![-1., -1., 0.], vec![-1., 1., 0.]],
            vec![vec![1., -1., 0.], vec![1., 1., 0.]],
        ];
        assert!(
            inspect(&natural, &world, &uv, 1e-12, 9)
                .unwrap()
                .within_budget
        );
        #[cfg(feature = "transport")]
        {
            let request = value_codec::json!({"op":"sweep_cap_boundary_audit","surface":s,
            "world":world,"uv":uv,"tolerance":1e-12,"maxProducts":9});
            let r = crate::transport::dispatch(request.clone()).unwrap();
            assert_eq!(r["withinBudget"], value_codec::json!(true));
            assert_eq!(r["capGeometryCertified"], value_codec::json!(false));
            assert_eq!(r["globalEmbeddingCertified"], value_codec::json!(false));
            let mut exhausted = request;
            exhausted["maxProducts"] = value_codec::json!(0);
            let r = crate::transport::dispatch(exhausted).unwrap();
            assert_eq!(r["errorUpper"], value_codec::Value::Null);
            assert_eq!(
                r["reason"],
                value_codec::json!("cap-product-budget-exhausted")
            );
        }

        assert!(
            inspect(&s, &world, &uv, 1e-12, 8)
                .unwrap()
                .error_upper
                .is_none()
        );
        let mut warped = s.clone();
        warped.control_points[1][1][2] = 0.1;
        let report = inspect(&warped, &world, &uv, 0.001, 9).unwrap();
        assert!(!report.within_budget && report.error_upper.unwrap() > 0.02);
        uv.control_points[1][0] += 0.1;
        assert!(!inspect(&s, &world, &uv, 0.01, 9).unwrap().within_budget);
        uv.weights[1] = 3.;
        assert!(inspect(&s, &world, &uv, 1., 9).is_err());
    }
}
