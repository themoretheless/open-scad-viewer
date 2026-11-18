//! Sketch-to-solid authoring, shared by native and WASM callers.
use crate::{Model, Result, invalid};

pub enum Profile {
    Polygon(Vec<[f64; 2]>),
    Bezier(planar_geometry::path::BezierPath),
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
        Profile::Bezier(path) => {
            let mut wire = bezier_wire(path)?;
            if crate::bezier_profile::signed_area(&wire, 1e-7)? < 0. {
                wire.reverse();
                for curve in &mut wire {
                    curve.control_points.reverse();
                    curve.weights.reverse();
                }
            }
            (crate::prism::extrude(&[wire], 0., height.abs())?, [0., 0.])
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

pub use crate::sketch_wire::{bezier_wire, circle_wire, polygon_wire};

pub fn extrude_bezier_profiles(
    paths: Vec<planar_geometry::path::BezierPath>,
    height: f64,
    base_z: f64,
    plane: Option<[[f64; 3]; 3]>,
) -> Result<Model> {
    if !height.is_finite()
        || height.abs() < 1e-5
        || height.abs() > 1e6
        || !base_z.is_finite()
        || base_z.abs() > 1e6
    {
        return Err(invalid("Invalid Bézier extrusion height or offset"));
    }
    let mut wires = paths
        .into_iter()
        .map(bezier_wire)
        .collect::<Result<Vec<_>>>()?;
    let (_, depths) = crate::bezier_profile::classify_components(&wires, 1e-7)?;
    for (wire, depth) in wires.iter_mut().zip(depths) {
        if (crate::bezier_profile::signed_area(wire, 1e-7)? > 0.) != (depth % 2 == 0) {
            wire.reverse();
            for c in wire {
                c.control_points.reverse();
                c.weights.reverse();
            }
        }
    }
    let model = crate::prism::extrude(&wires, 0., height.abs())?;
    let [origin, u, v] = plane.unwrap_or([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]);
    crate::transform::workplane(&model, origin, u, v, [0., 0., base_z + height.min(0.)])
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
