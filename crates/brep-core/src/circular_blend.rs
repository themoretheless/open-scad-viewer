//! Exact support geometry for a convex plane/cylinder rim blend.
//! This authors open patches and contact rails, not a sewn solid or end transitions.
use crate::{Result, invalid};
use nurbs_core::{curve::Curve, surface::Surface};

pub struct CircularBlendSpan {
    pub centers: Curve,
    pub plane_contact: Curve,
    pub cylinder_contact: Curve,
    pub surface: Surface,
}

fn arc(radius: f64, z: f64, start: f64, sweep: f64) -> Curve {
    let middle = start + sweep / 2.;
    let weight = (sweep / 2.).cos();
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![radius * start.cos(), radius * start.sin(), z],
            vec![
                radius * middle.cos() / weight,
                radius * middle.sin() / weight,
                z,
            ],
            vec![
                radius * (start + sweep).cos(),
                radius * (start + sweep).sin(),
                z,
            ],
        ],
        weights: vec![1., weight, 1.],
        periodic: false,
    }
}

/// Local cylinder Z axis, top plane z=height, positive convex radius.
/// Each span covers at most pi/2, including reversed and seam-crossing arcs.
pub fn plane_cylinder_rim(
    outer_radius: f64,
    height: f64,
    radius: f64,
    start: f64,
    sweep: f64,
) -> Result<Vec<CircularBlendSpan>> {
    if [outer_radius, height, radius, start, sweep]
        .iter()
        .any(|v| !v.is_finite())
        || radius <= 0.
        || outer_radius <= radius
        || height <= radius
        || sweep.abs() < 1e-12
        || sweep.abs() > std::f64::consts::TAU
    {
        return Err(invalid(
            "Circular blend requires a fitting radius and a finite nonzero arc of at most one turn",
        ));
    }
    let count = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize;
    let delta = sweep / count as f64;
    let mut spans = Vec::with_capacity(count);
    for i in 0..count {
        let angle = start + i as f64 * delta;
        let unit = arc(1., 0., angle, delta);
        let centers = arc(outer_radius - radius, height - radius, angle, delta);
        let plane_contact = arc(outer_radius - radius, height, angle, delta);
        let cylinder_contact = arc(outer_radius, height - radius, angle, delta);
        // Meridian controls describe an exact quarter circle centered at
        // (outer_radius-radius,height-radius). Tensor product with the angular
        // circle gives the torus patch, with strictly positive product weights.
        let meridian = [
            (outer_radius - radius, height),
            (outer_radius, height),
            (outer_radius, height - radius),
        ];
        let meridian_weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.];
        let surface = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: unit.knots.clone(),
            knots_v: unit.knots.clone(),
            control_points: unit
                .control_points
                .iter()
                .map(|p| {
                    meridian
                        .iter()
                        .map(|&(r, z)| vec![r * p[0], r * p[1], z])
                        .collect()
                })
                .collect(),
            weights: unit
                .weights
                .iter()
                .map(|w| meridian_weights.iter().map(|v| w * v).collect())
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        centers.validate()?;
        plane_contact.validate()?;
        cylinder_contact.validate()?;
        surface.validate()?;
        spans.push(CircularBlendSpan {
            centers,
            plane_contact,
            cylinder_contact,
            surface,
        });
    }
    Ok(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn torus_support_has_exact_radius_and_contact_boundaries() {
        for sweep in [0.31, 2.2, -2.2, std::f64::consts::TAU] {
            let spans = plane_cylinder_rim(20., 6., 1.25, 5.9, sweep).unwrap();
            for span in &spans {
                for i in 0..=20 {
                    let u = i as f64 / 20.;
                    let center = span.centers.evaluate(u).unwrap().point;
                    for j in 0..=20 {
                        let point = span.surface.evaluate(u, j as f64 / 20.).unwrap().point;
                        let distance = (point[0] - center[0])
                            .hypot(point[1] - center[1])
                            .hypot(point[2] - center[2]);
                        assert!((distance - 1.25).abs() < 1e-11);
                    }
                    for (v, rail) in [(0., &span.plane_contact), (1., &span.cylinder_contact)] {
                        let evaluation = span.surface.evaluate(u, v).unwrap();
                        let point = evaluation.point;
                        let normal = evaluation.unit_normal().unwrap();
                        let target = if v == 0. {
                            [0., 0., 1.]
                        } else {
                            [point[0] / 20., point[1] / 20., 0.]
                        };
                        let alignment: f64 = (0..3).map(|k| normal[k] * target[k]).sum();
                        assert!((alignment.abs() - 1.).abs() < 1e-11);
                        let expected = rail.evaluate(u).unwrap().point;
                        for k in 0..3 {
                            assert!((point[k] - expected[k]).abs() < 1e-11);
                        }
                    }
                }
            }
            for pair in spans.windows(2) {
                for v in [0., 0.3, 1.] {
                    let a = pair[0].surface.evaluate(1., v).unwrap().point;
                    let b = pair[1].surface.evaluate(0., v).unwrap().point;
                    assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-11));
                }
            }
        }
    }
    #[test]
    fn impossible_and_nonfinite_support_refuses() {
        for radius in [0., -1., 20., f64::NAN] {
            assert!(plane_cylinder_rim(20., 6., radius, 0., 1.).is_err());
        }
        assert!(plane_cylinder_rim(20., 6., 1., 0., 7.).is_err());
    }
}
