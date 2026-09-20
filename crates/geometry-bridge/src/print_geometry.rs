//! Geometric build-direction screening, not support generation or print certification.
use crate::{Error, Result};
use math_core::{cross, dot, sub};
use polygon_core::Mesh;

pub(crate) fn dispatch(mut value: value_codec::Value) -> Result<value_codec::Value> {
    use crate::{field, input, require_exact_fields, take_field};
    use value_codec::json;
    require_exact_fields(
        &value,
        &[
            "op",
            "mesh",
            "buildDirection",
            "coneDegrees",
            "planeOffsetMm",
            "planeToleranceMm",
        ],
        "Build surface request",
    )?;
    let mesh = value["mesh"]
        .as_object()
        .ok_or_else(|| input("Build surface mesh must be an object"))?;
    if mesh
        .keys()
        .any(|key| !matches!(key.as_str(), "positions" | "indices" | "uv"))
    {
        return Err(input("Build surface mesh contains unauthorized fields"));
    }
    for (name, limit) in [("positions", 2_000_000), ("indices", 300_000)] {
        let values = value["mesh"][name]
            .as_array()
            .ok_or_else(|| input(format!("Build surface {name} must be an array")))?;
        if values.len() > limit {
            return Err(input("Build surface mesh exceeds its buffer budget"));
        }
    }
    let build_direction = field(&value, "buildDirection")?;
    let cone_degrees = field(&value, "coneDegrees")?;
    let plane_offset_mm = field(&value, "planeOffsetMm")?;
    let plane_tolerance_mm = field(&value, "planeToleranceMm")?;
    let mesh = take_field(&mut value, "mesh")?;
    let report = inspect_build_surfaces(
        &mesh,
        build_direction,
        cone_degrees,
        plane_offset_mm,
        plane_tolerance_mm,
    )?;
    Ok(json!({"modelKind":"signed-triangle-build-surfaces-v1",
        "totalAreaMm2":report.total_area_mm2,"downwardAreaMm2":report.downward_area_mm2,
        "downwardTriangles":report.downward_triangles,"contactAreaMm2":report.contact_area_mm2,
        "belowPlaneTriangles":report.below_plane_triangles}))
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildSurfaceReport {
    pub total_area_mm2: f64,
    /// Downward normals within the requested cone, excluding plane-contact faces.
    pub downward_area_mm2: f64,
    pub downward_triangles: usize,
    /// Whole triangles whose vertices lie within plane tolerance.
    pub contact_area_mm2: f64,
    /// Any vertex below the plane minus tolerance, not clipped area.
    pub below_plane_triangles: usize,
}

/// Coordinates and plane offset are mm in the same frame. The normalized build
/// direction points away from the plate. Cone degrees are measured from its
/// negative direction: 0 selects horizontal downward faces, 90 any downward face.
/// Mesh winding supplies normals; self-intersection is not certified here.
pub fn inspect_build_surfaces(
    mesh: &Mesh,
    build_direction: [f64; 3],
    cone_degrees: f64,
    plane_offset_mm: f64,
    plane_tolerance_mm: f64,
) -> Result<BuildSurfaceReport> {
    let invalid = || {
        Error::new(
            "PRINT_GEOMETRY_INVALID",
            "Build surface inspection requires finite bounded geometry, a nonzero build direction and valid plane/cone settings",
        )
    };
    if mesh.indices.is_empty()
        || mesh.indices.len() > 300_000
        || mesh.positions.len() > 2_000_000
        || !mesh
            .positions
            .iter()
            .all(|x| x.is_finite() && x.abs() <= 1_000_000.)
        || !build_direction.iter().all(|x| x.is_finite())
        || !cone_degrees.is_finite()
        || !(0.0..=90.0).contains(&cone_degrees)
        || !plane_offset_mm.is_finite()
        || plane_offset_mm.abs() > 2_000_000.
        || !plane_tolerance_mm.is_finite()
        || !(0.0..=1_000_000.).contains(&plane_tolerance_mm)
    {
        return Err(invalid());
    }
    let length = build_direction[0]
        .hypot(build_direction[1])
        .hypot(build_direction[2]);
    if !length.is_finite() || length == 0. {
        return Err(invalid());
    }
    let up = build_direction.map(|x| x / length);
    let topology = mesh.inspect()?;
    if !topology.closed
        || topology.orientation_conflicts > 0
        || topology.non_manifold_edges > 0
        || topology.degenerate_triangles > 0
        || topology.signed_volume_mm3 <= 0.
    {
        return Err(Error::new(
            "PRINT_GEOMETRY_TOPOLOGY",
            "Build surface inspection requires a closed, consistently wound mesh with positive signed volume",
        ));
    }
    let cosine = cone_degrees.to_radians().cos();
    let mut report = BuildSurfaceReport {
        total_area_mm2: 0.,
        downward_area_mm2: 0.,
        downward_triangles: 0,
        contact_area_mm2: 0.,
        below_plane_triangles: 0,
    };
    for triangle in mesh.indices.chunks_exact(3) {
        let point = |i: usize| {
            [
                mesh.positions[i * 3],
                mesh.positions[i * 3 + 1],
                mesh.positions[i * 3 + 2],
            ]
        };
        let [a, b, c] = [point(triangle[0]), point(triangle[1]), point(triangle[2])];
        let normal = cross(sub(b, a), sub(c, a));
        let twice_area = normal[0].hypot(normal[1]).hypot(normal[2]);
        if twice_area == 0. || !twice_area.is_finite() {
            return Err(invalid());
        }
        let area = twice_area / 2.;
        report.total_area_mm2 += area;
        let heights = [a, b, c].map(|p| dot(p, up) - plane_offset_mm);
        if heights.iter().any(|h| *h < -plane_tolerance_mm) {
            report.below_plane_triangles += 1;
        }
        if heights.iter().all(|h| h.abs() <= plane_tolerance_mm) {
            report.contact_area_mm2 += area;
            continue;
        }
        let alignment = dot(normal.map(|value| value / twice_area), up);
        if alignment < 0. && -alignment >= cosine {
            report.downward_area_mm2 += area;
            report.downward_triangles += 1;
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn box_mesh() -> Mesh {
        Mesh {
            positions: vec![
                0., 0., 0., 2., 0., 0., 2., 3., 0., 0., 3., 0., 0., 0., 4., 2., 0., 4., 2., 3., 4.,
                0., 3., 4.,
            ],
            indices: vec![
                0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2,
                7, 6, 3, 0, 4, 3, 4, 7,
            ],
            uv: None,
        }
    }
    #[test]
    fn distinguishes_contact_from_floating_downward_faces() {
        let mesh = box_mesh();
        let seated = inspect_build_surfaces(&mesh, [0., 0., 1.], 45., 0., 0.).unwrap();
        assert_eq!(seated.total_area_mm2, 52.);
        assert_eq!(seated.contact_area_mm2, 6.);
        assert_eq!(seated.downward_area_mm2, 0.);
        assert_eq!(seated.below_plane_triangles, 0);
        let floating = inspect_build_surfaces(&mesh, [0., 0., 1.], 45., -1., 0.).unwrap();
        assert_eq!(floating.downward_area_mm2, 6.);
        assert_eq!(floating.downward_triangles, 2);
        assert_eq!(floating.contact_area_mm2, 0.);
        assert!(
            inspect_build_surfaces(&mesh, [0., 0., 1.], 45., 1., 0.)
                .unwrap()
                .below_plane_triangles
                > 0
        );
    }
    #[test]
    fn direction_scale_and_plane_translation_are_explicit() {
        let mut mesh = box_mesh();
        let x = inspect_build_surfaces(&mesh, [10., 0., 0.], 90., 0., 0.).unwrap();
        assert_eq!(x.contact_area_mm2, 12.);
        assert_eq!(x.downward_triangles, 0);
        for p in mesh.positions.chunks_exact_mut(3) {
            p[2] += 10.;
        }
        assert_eq!(
            inspect_build_surfaces(&mesh, [0., 0., 2.], 0., 10., 0.)
                .unwrap()
                .contact_area_mm2,
            6.
        );
    }
    #[test]
    fn refuses_invalid_settings_open_or_reversed_meshes() {
        let mut mesh = box_mesh();
        for direction in [[0., 0., 0.], [f64::NAN, 0., 1.]] {
            assert!(inspect_build_surfaces(&mesh, direction, 45., 0., 0.).is_err());
        }
        for cone in [-1., 91., f64::NAN] {
            assert!(inspect_build_surfaces(&mesh, [0., 0., 1.], cone, 0., 0.).is_err());
        }
        for tri in mesh.indices.chunks_exact_mut(3) {
            tri.swap(1, 2);
        }
        assert_eq!(
            inspect_build_surfaces(&mesh, [0., 0., 1.], 45., 0., 0.)
                .unwrap_err()
                .code,
            "PRINT_GEOMETRY_TOPOLOGY"
        );
        mesh.indices.truncate(3);
        assert!(inspect_build_surfaces(&mesh, [0., 0., 1.], 45., 0., 0.).is_err());
    }

    #[test]
    fn cone_uses_actual_normals_and_plane_tolerance_is_explicit() {
        let mesh = box_mesh();
        assert_eq!(
            inspect_build_surfaces(&mesh, [1., 0., 1.], 30., -1., 0.)
                .unwrap()
                .downward_area_mm2,
            0.
        );
        assert_eq!(
            inspect_build_surfaces(&mesh, [1., 0., 1.], 60., -1., 0.)
                .unwrap()
                .downward_area_mm2,
            18.
        );
        assert_eq!(
            inspect_build_surfaces(&mesh, [0., 0., -1.], 45., -4., 0.)
                .unwrap()
                .contact_area_mm2,
            6.
        );
        assert_eq!(
            inspect_build_surfaces(&mesh, [0., 0., 1.], 45., 0.01, 0.02)
                .unwrap()
                .contact_area_mm2,
            6.
        );
        for (plane, tolerance) in [(f64::NAN, 0.), (0., -1.), (0., f64::INFINITY)] {
            assert!(inspect_build_surfaces(&mesh, [0., 0., 1.], 45., plane, tolerance).is_err());
        }
    }
}
