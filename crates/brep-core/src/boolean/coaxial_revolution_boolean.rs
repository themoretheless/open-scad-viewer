//! Coaxial Booleans of cylinders, frusta and cones.
//!
//! Two solids of revolution about one axis combine into a solid of revolution
//! about the same axis, and its meridian section is the planar Boolean of the
//! two sections. A cylinder's section in the (radius, axial) half-plane is a
//! rectangle, a frustum's a trapezoid and a cone's a triangle; all of them
//! are polygons with one side on the axis. Their Boolean is computed exactly
//! by `revolution_section`, and revolving it gives exact cylinder, cone and
//! disk patches. Every result is audited by the caller's certificate.
//!
//! Axes must coincide within the recognition band. Tori (disk sections) and
//! spheres (half disks) are included; the section Boolean of
//! `revolution_section` keeps their arcs exact.
use crate::intersections::sphere_sphere;
use crate::intersections::{recognize_cone, recognize_cylinder, recognize_torus};
use crate::revolution_section::{self, Op, Section};
use crate::sphere_mate::{cross, dot, norm, sub, unsupported};
use crate::{Model, revolve_region, transform};
use nurbs_core::Result;

/// A canonical solid of revolution: its axis (when it has one) and enough to
/// write its meridian section in any frame along that axis.
enum Revolved {
    /// Cylinder, frustum or cone: radii at the two ends of the axis segment.
    Ruled {
        bottom: [f64; 3],
        axis: [f64; 3],
        frame_x: [f64; 3],
        r_bottom: f64,
        r_top: f64,
        height: f64,
        error: f64,
    },
    Torus {
        center: [f64; 3],
        axis: [f64; 3],
        frame_x: [f64; 3],
        major: f64,
        minor: f64,
        error: f64,
    },
    /// A sphere has every axis through its centre.
    Sphere {
        center: [f64; 3],
        radius: f64,
        error: f64,
    },
}

impl Revolved {
    fn recognize(model: &Model) -> Result<Option<Self>> {
        if let Some(c) = recognize_cylinder(model)? {
            return Ok(Some(Self::Ruled {
                bottom: std::array::from_fn(|k| c.center[k] - c.half_height * c.axis[k]),
                axis: c.axis,
                frame_x: c.frame[0],
                r_bottom: c.radius,
                r_top: c.radius,
                height: 2. * c.half_height,
                error: c.error,
            }));
        }
        if let Some(c) = recognize_cone(model)? {
            return Ok(Some(Self::Ruled {
                bottom: c.bottom,
                axis: c.axis,
                frame_x: c.frame[0],
                r_bottom: c.r_bottom,
                r_top: c.r_top,
                height: c.height,
                error: c.error,
            }));
        }
        if let Some(t) = recognize_torus(model)? {
            return Ok(Some(Self::Torus {
                center: t.center,
                axis: t.axis,
                frame_x: t.frame[0],
                major: t.major,
                minor: t.minor,
                error: t.error,
            }));
        }
        if let Some(s) = sphere_sphere::recognize(model)? {
            return Ok(Some(Self::Sphere {
                center: s.center,
                radius: s.radius,
                error: s.error,
            }));
        }
        Ok(None)
    }

    /// (point on axis, unit axis, radial frame direction), unless a sphere.
    fn axis(&self) -> Option<([f64; 3], [f64; 3], [f64; 3])> {
        match *self {
            Self::Ruled {
                bottom,
                axis,
                frame_x,
                ..
            } => Some((bottom, axis, frame_x)),
            Self::Torus {
                center,
                axis,
                frame_x,
                ..
            } => Some((center, axis, frame_x)),
            Self::Sphere { .. } => None,
        }
    }

    fn anchor(&self) -> [f64; 3] {
        match *self {
            Self::Ruled { bottom, .. } => bottom,
            Self::Torus { center, .. } | Self::Sphere { center, .. } => center,
        }
    }

    fn error(&self) -> f64 {
        match *self {
            Self::Ruled { error, .. } | Self::Torus { error, .. } | Self::Sphere { error, .. } => {
                error
            }
        }
    }

    fn extent(&self) -> f64 {
        match *self {
            Self::Ruled {
                r_bottom,
                r_top,
                height,
                ..
            } => r_bottom.max(r_top).max(height),
            Self::Torus { major, minor, .. } => major + minor,
            Self::Sphere { radius, .. } => radius,
        }
    }

    /// Meridian section in the half-plane of the axis through `origin`.
    fn section(&self, origin: [f64; 3], axis: [f64; 3], snap: f64) -> Section {
        let z = |p: [f64; 3]| dot(sub(p, origin), axis);
        match *self {
            Self::Ruled {
                bottom,
                axis: own,
                r_bottom,
                r_top,
                height,
                ..
            } => {
                let top: [f64; 3] = std::array::from_fn(|k| bottom[k] + height * own[k]);
                let mut ends = [(z(bottom), r_bottom), (z(top), r_top)];
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
                Section::Polygon(ring)
            }
            Self::Torus {
                center,
                major,
                minor,
                ..
            } => Section::Disk {
                center: [major, z(center)],
                radius: minor,
            },
            Self::Sphere { center, radius, .. } => Section::HalfDisk {
                center_z: z(center),
                radius,
            },
        }
    }
}

/// Result of a coaxial Boolean: the solid, or `None` for an empty result.
pub(crate) enum Outcome {
    Solid(Model),
    Empty,
}

/// Coaxial Boolean of two canonical solids of revolution (cylinder, frustum,
/// cone, torus, sphere; not two spheres). `Ok(None)` when an operand is not
/// such a solid or the axes differ.
pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Outcome>> {
    let op = match operation {
        "union" => Op::Union,
        "difference" => Op::Difference,
        "intersection" => Op::Intersection,
        _ => return Ok(None),
    };
    let (Some(sa), Some(sb)) = (Revolved::recognize(a)?, Revolved::recognize(b)?) else {
        return Ok(None);
    };
    let Some((origin, axis, frame_x)) = sa.axis().or_else(|| sb.axis()) else {
        return Ok(None);
    };
    let scale = sa.extent().max(sb.extent()) + norm(sub(sa.anchor(), sb.anchor())) + 1.;
    let band = sa.error() + sb.error() + 1e-9 * scale;
    // Coaxial: every axis parallel to the reference and every anchor on it.
    for solid in [&sa, &sb] {
        if let Some((_, own, _)) = solid.axis()
            && norm(cross(axis, own)) > band / scale
        {
            return Ok(None);
        }
        let off = sub(solid.anchor(), origin);
        let along = dot(off, axis);
        let perp: [f64; 3] = std::array::from_fn(|k| off[k] - along * axis[k]);
        if norm(perp) > band {
            return Ok(None);
        }
    }
    let snap = band;
    let loops = revolution_section::boolean(
        &sa.section(origin, axis, snap),
        &sb.section(origin, axis, snap),
        op,
        band.max(1e-12),
    )?;
    let mut curves = Vec::new();
    for spans in &loops {
        if revolution_section::area(spans).abs() <= band * scale {
            continue;
        }
        let loop_curves = revolution_section::curves(spans, snap);
        if loop_curves
            .iter()
            .flat_map(|c| &c.control_points)
            .any(|p| p[0] < 0.)
        {
            return Err(unsupported(
                "Coaxial revolution Boolean: section crosses the axis",
            ));
        }
        curves.push(loop_curves);
    }
    if curves.is_empty() {
        return Ok(Some(Outcome::Empty));
    }
    let local = revolve_region(&curves, a.tolerance_mm.max(b.tolerance_mm))?;
    // Local revolution axis is +z with radius along +x; place it on the axis.
    let x = frame_x;
    let y = cross(axis, x);
    let placed = transform::affine(
        &local,
        [
            [x[0], y[0], axis[0], origin[0]],
            [x[1], y[1], axis[1], origin[1]],
            [x[2], y[2], axis[2], origin[2]],
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

    /// Inclusion-exclusion of volumes: |A u B| + |A n B| = |A| + |B| and
    /// |A - B| + |A n B| = |A|.
    fn check_volumes(a: &Model, b: &Model) {
        let measure = |outcome: Option<Outcome>| match outcome.unwrap() {
            Outcome::Solid(model) => {
                model.validate().unwrap();
                volume(&model)
            }
            Outcome::Empty => 0.,
        };
        let (va, vb) = (volume(a), volume(b));
        let union = measure(boolean(a, b, "union").unwrap());
        let inter = measure(boolean(a, b, "intersection").unwrap());
        let diff = measure(boolean(a, b, "difference").unwrap());
        assert!(inter > 1e-3, "the pair should overlap");
        assert!((union + inter - va - vb).abs() < 1e-3 * (va + vb), "{union} {inter} {va} {vb}");
        assert!((diff + inter - va).abs() < 1e-3 * va, "{diff} {inter} {va}");
    }

    #[test]
    fn tori_with_cylinders_cones_spheres_and_tori() {
        let torus = crate::torus(3., 1.).unwrap();
        let cylinder = shifted(&cylinder(2.5, 2.).unwrap(), [0., 0., -1.]);
        let cone = shifted(&frustum(5., 0., 5.).unwrap(), [0., 0., -1.]);
        let ball = crate::sphere(3.).unwrap();
        let lifted = shifted(&torus, [0., 0., 1.]);
        for other in [&cylinder, &cone, &ball, &lifted] {
            check_volumes(&torus, other);
            check_volumes(other, &torus);
        }
        // Torus volume by Pappus: 2 pi^2 R r^2.
        let pappus = 2. * std::f64::consts::PI.powi(2) * 3.;
        assert!((volume(&torus) - pappus).abs() < 1e-3);
    }

    #[test]
    fn torus_inside_a_cylinder_leaves_a_cavity() {
        let torus = crate::torus(3., 1.).unwrap();
        let drum = shifted(&cylinder(5., 4.).unwrap(), [0., 0., -2.]);
        let hollow = solid(boolean(&drum, &torus, "difference").unwrap());
        assert_eq!(hollow.bodies.len(), 1);
        assert_eq!(hollow.bodies[0].inner_shells.len(), 1);
        let expected = volume(&drum) - volume(&torus);
        assert!((volume(&hollow) - expected).abs() < 1e-3);
    }

    #[test]
    fn non_coaxial_pairs_are_left_to_other_cells() {
        let c = cylinder(2., 6.).unwrap();
        let k = shifted(&frustum(4., 0., 8.).unwrap(), [0.5, 0., 1.]);
        assert!(boolean(&c, &k, "union").unwrap().is_none());
        assert!(boolean(&c, &c, "xor").unwrap().is_none());
    }
}
