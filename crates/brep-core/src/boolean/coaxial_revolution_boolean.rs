//! Coaxial Booleans of cylinders, frusta and cones.
//!
//! Two solids of revolution about one axis combine into a solid of revolution
//! about the same axis, and its meridian section is the planar Boolean of the
//! two sections. A cylinder's section in the (radius, axial) half-plane is a
//! rectangle, a frustum's a trapezoid and a cone's a triangle; all of them
//! are polygons with one side on the axis. The planar Boolean of polygons is
//! a polygonal region, computed by `planar_geometry::curve_boolean` from the
//! original line spans, and revolving it gives exact cylinder, cone and disk
//! patches. Every result is audited by the caller's certificate.
//!
//! Axes must coincide within the recognition band. Spheres and tori have
//! circular sections, which this polygonal Boolean does not carry; their
//! pairs belong to the sphere-mate cells.
use crate::intersections::{recognize_cone, recognize_cylinder};
use crate::sphere_mate::{cross, dot, norm, sub, unsupported};
use crate::{Model, revolve_region, transform};
use nurbs_core::{Result, curve::Curve};
use planar_geometry::curve::{BezPath, PathEl, Point};
use planar_geometry::curve_boolean::{BinaryOp, FillRule, binary_op};

/// A solid of revolution recognized as a cylinder, frustum or cone: the axis
/// through `bottom`, the radii at its two ends and its height along `axis`.
struct Revolved {
    bottom: [f64; 3],
    axis: [f64; 3],
    frame_x: [f64; 3],
    r_bottom: f64,
    r_top: f64,
    height: f64,
    error: f64,
}

fn recognize(model: &Model) -> Result<Option<Revolved>> {
    if let Some(c) = recognize_cylinder(model)? {
        let bottom = std::array::from_fn(|k| c.center[k] - c.half_height * c.axis[k]);
        return Ok(Some(Revolved {
            bottom,
            axis: c.axis,
            frame_x: c.frame[0],
            r_bottom: c.radius,
            r_top: c.radius,
            height: 2. * c.half_height,
            error: c.error,
        }));
    }
    if let Some(c) = recognize_cone(model)? {
        return Ok(Some(Revolved {
            bottom: c.bottom,
            axis: c.axis,
            frame_x: c.frame[0],
            r_bottom: c.r_bottom,
            r_top: c.r_top,
            height: c.height,
            error: c.error,
        }));
    }
    Ok(None)
}

/// Section polygon of `solid` in the (radius, axial) half-plane of the axis
/// through `origin` along `axis`, counter-clockwise (material on the left).
fn section(solid: &Revolved, origin: [f64; 3], axis: [f64; 3], snap: f64) -> Vec<[f64; 2]> {
    let top: [f64; 3] = std::array::from_fn(|k| solid.bottom[k] + solid.height * solid.axis[k]);
    let mut ends = [
        (dot(sub(solid.bottom, origin), axis), solid.r_bottom),
        (dot(sub(top, origin), axis), solid.r_top),
    ];
    if ends[0].0 > ends[1].0 {
        ends.swap(0, 1);
    }
    let [(z0, r0), (z1, r1)] = ends;
    let mut ring = vec![[0., z0]];
    if r0 > snap {
        ring.push([r0, z0]);
    }
    if r1 > snap {
        ring.push([r1, z1]);
    }
    ring.push([0., z1]);
    ring
}

fn path(ring: &[[f64; 2]]) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(Point {
        x: ring[0][0],
        y: ring[0][1],
    });
    for p in &ring[1..] {
        path.line_to(Point { x: p[0], y: p[1] });
    }
    path.close_path();
    path
}

fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}

/// Result of a coaxial Boolean: the solid, or `None` for an empty result.
pub(crate) enum Outcome {
    Solid(Model),
    Empty,
}

/// Coaxial cylinder/frustum/cone Boolean. `Ok(None)` when an operand is not
/// such a solid or the axes differ; refusals for unsupported sections.
pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Outcome>> {
    let op = match operation {
        "union" => BinaryOp::Union,
        "difference" => BinaryOp::Difference,
        "intersection" => BinaryOp::Intersection,
        _ => return Ok(None),
    };
    let (Some(sa), Some(sb)) = (recognize(a)?, recognize(b)?) else {
        return Ok(None);
    };
    let scale = sa.height.max(sb.height).max(sa.r_bottom.max(sa.r_top)).max(sb.r_bottom.max(sb.r_top))
        + norm(sub(sa.bottom, sb.bottom))
        + 1.;
    let band = sa.error + sb.error + 1e-9 * scale;
    // Coaxial: parallel axes and B's bottom on A's axis line.
    let off = sub(sb.bottom, sa.bottom);
    let along = dot(off, sa.axis);
    let perp: [f64; 3] = std::array::from_fn(|k| off[k] - along * sa.axis[k]);
    if norm(cross(sa.axis, sb.axis)) > band / scale || norm(perp) > band {
        return Ok(None);
    }
    let snap = band;
    let ring_a = section(&sa, sa.bottom, sa.axis, snap);
    let ring_b = section(&sb, sa.bottom, sa.axis, snap);
    let contours = binary_op(
        &path(&ring_a),
        &path(&ring_b),
        FillRule::NonZero,
        FillRule::NonZero,
        op,
        band.max(1e-12),
    )?;
    let mut loops: Vec<Vec<Curve>> = Vec::new();
    for contour in contours.contours() {
        let mut points: Vec<[f64; 2]> = Vec::new();
        for element in &contour.path {
            match *element {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                    // Points within the band of the axis lie on it exactly.
                    let x = if p.x.abs() <= snap { 0. } else { p.x };
                    let q = [x, p.y];
                    if points.last().is_none_or(|last| {
                        (last[0] - q[0]).abs() > snap || (last[1] - q[1]).abs() > snap
                    }) {
                        points.push(q);
                    }
                }
                PathEl::ClosePath => {}
                _ => {
                    return Err(unsupported(
                        "Coaxial revolution Boolean: the section Boolean produced a curved span",
                    ));
                }
            }
        }
        if points.len() > 1 {
            let (first, last) = (points[0], points[points.len() - 1]);
            if (first[0] - last[0]).abs() <= snap && (first[1] - last[1]).abs() <= snap {
                points.pop();
            }
        }
        if points.len() < 3 {
            continue;
        }
        if points.iter().any(|p| p[0] < 0.) {
            return Err(unsupported(
                "Coaxial revolution Boolean: section crosses the axis",
            ));
        }
        loops.push(
            (0..points.len())
                .map(|i| line(points[i], points[(i + 1) % points.len()]))
                .collect(),
        );
    }
    if loops.is_empty() {
        return Ok(Some(Outcome::Empty));
    }
    let local = revolve_region(&loops, a.tolerance_mm.max(b.tolerance_mm))?;
    // Local revolution axis is +z with radius along +x; place it on A's axis.
    let x = sa.frame_x;
    let y = cross(sa.axis, x);
    let z = sa.axis;
    let o = sa.bottom;
    let placed = transform::affine(
        &local,
        [
            [x[0], y[0], z[0], o[0]],
            [x[1], y[1], z[1], o[1]],
            [x[2], y[2], z[2], o[2]],
            [0., 0., 0., 1.],
        ],
    )?;
    placed.validate()?;
    Ok(Some(Outcome::Solid(placed)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cylinder, frustum};

    fn shifted(model: &Model, by: [f64; 3]) -> Model {
        transform::affine(
            model,
            [
                [1., 0., 0., by[0]],
                [0., 1., 0., by[1]],
                [0., 0., 1., by[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn solid(outcome: Option<Outcome>) -> Model {
        match outcome.unwrap() {
            Outcome::Solid(model) => model,
            Outcome::Empty => panic!("unexpected empty result"),
        }
    }
    fn z_range(model: &Model) -> (f64, f64) {
        model
            .vertices
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v.point[2]), hi.max(v.point[2]))
            })
    }
    fn max_radius(model: &Model) -> f64 {
        model
            .vertices
            .iter()
            .map(|v| v.point[0].hypot(v.point[1]))
            .fold(0., f64::max)
    }
    fn volume(model: &Model) -> f64 {
        crate::analysis::mass_properties(model, 1e-7, 800_000)
            .unwrap()
            .signed_volume_mm3
    }
    fn frustum_volume(r0: f64, r1: f64, h: f64) -> f64 {
        std::f64::consts::PI * h * (r0 * r0 + r0 * r1 + r1 * r1) / 3.
    }
    fn has_vertex_ring(model: &Model, radius: f64, z: f64) -> bool {
        model.vertices.iter().any(|v| {
            (v.point[0].hypot(v.point[1]) - radius).abs() < 1e-9 && (v.point[2] - z).abs() < 1e-9
        })
    }

    #[test]
    fn cone_through_a_cylinder() {
        // Cylinder r 2, z 0..6; cone r 4 at z = 1 to the apex at z = 9.
        // The cone side reaches radius 2 at z = 5.
        let c = cylinder(2., 6.).unwrap();
        let k = shifted(&frustum(4., 0., 8.).unwrap(), [0., 0., 1.]);
        let union = solid(boolean(&c, &k, "union").unwrap());
        assert_eq!(z_range(&union), (0., 9.));
        assert!((max_radius(&union) - 4.).abs() < 1e-9);
        assert!(has_vertex_ring(&union, 2., 1.), "cylinder wall meets the cone base");
        assert!(has_vertex_ring(&union, 2., 5.), "cone side crosses the wall");
        let intersection = solid(boolean(&c, &k, "intersection").unwrap());
        assert_eq!(z_range(&intersection), (1., 6.));
        assert!((max_radius(&intersection) - 2.).abs() < 1e-9);
        let difference = solid(boolean(&c, &k, "difference").unwrap());
        assert_eq!(z_range(&difference), (0., 6.));
        // Below the cone the cylinder stays whole; above z = 5 the cone is
        // inside the wall, so the difference has a conical pocket.
        assert!(has_vertex_ring(&difference, 2., 1.));
        let reverse = solid(boolean(&k, &c, "difference").unwrap());
        assert_eq!(z_range(&reverse), (1., 9.));
        // Volumes from the sections: cylinder 24 pi, cone 128 pi / 3, and
        // their common part a cylinder r 2 over z 1..5 plus the cone tip
        // from z 5 to 6 (radii 2 -> 1.5).
        let (vc, vk) = (frustum_volume(2., 2., 6.), frustum_volume(4., 0., 8.));
        let common = frustum_volume(2., 2., 4.) + frustum_volume(2., 1.5, 1.);
        for (model, expected) in [
            (&union, vc + vk - common),
            (&intersection, common),
            (&difference, vc - common),
            (&reverse, vk - common),
        ] {
            assert!((volume(model) - expected).abs() < 1e-4, "{} vs {expected}", volume(model));
        }
    }

    #[test]
    fn stacked_and_nested_frusta() {
        let a = frustum(3., 1., 4.).unwrap();
        // Same shape flipped above: they touch along the top cap of radius 1.
        let b = shifted(&frustum(1., 3., 4.).unwrap(), [0., 0., 4.]);
        let union = solid(boolean(&a, &b, "union").unwrap());
        assert_eq!(z_range(&union), (0., 8.));
        assert!(matches!(
            boolean(&a, &b, "intersection").unwrap().unwrap(),
            Outcome::Empty
        ));
        // A thin cylinder through the frustum: a tube-like difference.
        let rod = shifted(&cylinder(0.5, 10.).unwrap(), [0., 0., -3.]);
        let bored = solid(boolean(&a, &rod, "difference").unwrap());
        assert_eq!(bored.bodies.len(), 1);
        assert!(has_vertex_ring(&bored, 0.5, 0.) && has_vertex_ring(&bored, 0.5, 4.));
        bored.validate().unwrap();
    }

    #[test]
    fn non_coaxial_pairs_are_left_to_other_cells() {
        let c = cylinder(2., 6.).unwrap();
        let k = shifted(&frustum(4., 0., 8.).unwrap(), [0.5, 0., 1.]);
        assert!(boolean(&c, &k, "union").unwrap().is_none());
        assert!(boolean(&c, &c, "xor").unwrap().is_none());
    }
}
