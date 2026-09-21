//! Sketch-to-solid authoring, shared by native and WASM callers.
use crate::{Model, Result, invalid};

pub enum Profile {
    Polygon(Vec<[f64; 2]>),
    Circle { center: [f64; 2], radius: f64 },
}

pub fn extrude(
    profile: Profile,
    closed: bool,
    height: f64,
    base_z: f64,
    plane: Option<[[f64; 3]; 3]>,
) -> Result<Model> {
    if !closed
        || !height.is_finite()
        || height == 0.
        || height.abs() > 1e6
        || !base_z.is_finite()
        || base_z.abs() > 1e6
    {
        return Err(invalid(
            "A closed sketch and nonzero finite height are required",
        ));
    }
    let (model, center) = match profile {
        Profile::Circle { center, radius } => {
            if center.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
                return Err(invalid("Invalid sketch circle center"));
            }
            (crate::cylinder(radius, height.abs())?, center)
        }
        Profile::Polygon(mut points) => {
            if points.len() < 3
                || points.len() > 512
                || points
                    .iter()
                    .flatten()
                    .any(|v| !v.is_finite() || v.abs() > 1e6)
            {
                return Err(invalid("Invalid sketch polygon"));
            }
            let area: f64 = points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .map(|(p, q)| p[0] * q[1] - q[0] * p[1])
                .sum();
            if area < 0. {
                points.reverse();
            }
            (crate::extrude_polygon(&points, 0., height.abs())?, [0., 0.])
        }
    };
    let [origin, u, v] = plane.unwrap_or([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]);
    crate::transform::workplane(
        &model,
        origin,
        u,
        v,
        [center[0], center[1], base_z + height.min(0.)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clockwise_polygon_and_negative_height_are_authored_in_rust() {
        let model = extrude(
            Profile::Polygon(vec![[0., 0.], [0., 2.], [3., 2.], [3., 0.]]),
            true,
            -4.,
            1.,
            None,
        )
        .unwrap();
        model.validate().unwrap();
        assert!(
            model
                .vertices
                .iter()
                .all(|v| v.point[2] == -3. || v.point[2] == 1.)
        );
        assert!(extrude(Profile::Polygon(vec![]), true, 1., 0., None).is_err());
        assert!(
            extrude(
                Profile::Circle {
                    center: [0., 0.],
                    radius: 2.
                },
                false,
                1.,
                0.,
                None
            )
            .is_err()
        );
    }
    #[test]
    fn analytic_circle_keeps_curved_surfaces_in_workplane() {
        let model = extrude(
            Profile::Circle {
                center: [2., 3.],
                radius: 2.,
            },
            true,
            -3.,
            1.,
            Some([[10., 20., 30.], [0., 1., 0.], [0., 0., 1.]]),
        )
        .unwrap();
        assert!(model.faces.iter().any(|f| f.surface.degree_u == 2));
        assert!(
            model
                .vertices
                .iter()
                .all(|v| v.point[0] == 8. || v.point[0] == 11.)
        );
        model.validate().unwrap();
    }
}

pub fn polygon_wire(mut ring: Vec<[f64; 2]>) -> Result<Vec<nurbs_core::curve::Curve>> {
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    if ring.len() < 3 || ring.len() > 512 {
        return Err(invalid("Profile ring requires 3..512 points"));
    }
    (0..ring.len())
        .map(|i| {
            nurbs_core::curve::Curve::from_polyline(vec![
                ring[i].to_vec(),
                ring[(i + 1) % ring.len()].to_vec(),
            ])
        })
        .collect()
}

pub fn circle_wire(radius: f64) -> Result<Vec<nurbs_core::curve::Curve>> {
    if !radius.is_finite() || radius <= 0. || radius > 1e6 {
        return Err(invalid("Invalid profile circle radius"));
    }
    let cardinal = [[radius, 0.], [0., radius], [-radius, 0.], [0., -radius]];
    (0..4)
        .map(|i| {
            let a = cardinal[i];
            let b = cardinal[(i + 1) % 4];
            let curve = nurbs_core::curve::Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![a.to_vec(), vec![a[0] + b[0], a[1] + b[1]], b.to_vec()],
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            };
            curve.validate()?;
            Ok(curve)
        })
        .collect()
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    #[test]
    fn authored_wires_retain_analytic_geometry_and_closure() {
        let wire = circle_wire(3.).unwrap();
        crate::planar_trim::validate(std::slice::from_ref(&wire), 1e-7).unwrap();
        for arc in wire {
            for i in 0..=16 {
                let p = arc.evaluate(i as f64 / 16.).unwrap().point;
                assert!((p[0] * p[0] + p[1] * p[1] - 9.).abs() < 1e-12);
            }
        }
        let ring = polygon_wire(vec![[0., 0.], [2., 0.], [2., 1.], [0., 1.], [0., 0.]]).unwrap();
        assert_eq!(ring.len(), 4);
        assert!((crate::planar_trim::signed_area(&ring, 1e-7).unwrap() - 2.).abs() < 1e-12);
        assert!(circle_wire(0.).is_err());
    }
}
