use brep_core::{cuboid, cylinder, operations, sphere, transform};
use geometry_bridge::brep::nurbs;

fn placed_sphere(radius: f64, at: [f64; 3]) -> brep_core::Model {
    transform::affine(
        &sphere(radius).unwrap(),
        [
            [1., 0., 0., at[0]],
            [0., 1., 0., at[1]],
            [0., 0., 1., at[2]],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}

fn aabb(points: impl Iterator<Item = [f64; 3]>) -> [f64; 6] {
    let mut out = [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in points {
        for k in 0..3 {
            out[2 * k] = out[2 * k].min(p[k]);
            out[2 * k + 1] = out[2 * k + 1].max(p[k]);
        }
    }
    out
}

/// The app pairs each displayed body's mesh with its B-rep by AABB; the
/// tessellation of every box/sphere result must respect the trims.
#[test]
fn box_sphere_results_tessellate_within_their_vertex_bounds() {
    let big = cuboid([-10., -10., -10.], [10., 10., 10.]).unwrap();
    let bar = cuboid([-10., -1.5, -1.2], [10., 1.5, 1.2]).unwrap();
    let cases = [
        ("dome", big.clone(), placed_sphere(5., [0.3, -0.7, 8.])),
        ("edge", big.clone(), placed_sphere(4., [9., 0.4, 9.])),
        ("corner", big.clone(), placed_sphere(4., [9.3, 8.9, 9.1])),
        (
            "slab",
            cuboid([-10., -10., -2.], [10., 10., 2.]).unwrap(),
            placed_sphere(5., [0.3, -0.7, 0.1]),
        ),
        ("bar", bar, placed_sphere(4., [0.3, 0.1, -0.2])),
        (
            "axial belt",
            cylinder(3., 10.).unwrap(),
            placed_sphere(4., [0., 0., 5.]),
        ),
        (
            "axial cap ring",
            cylinder(3., 10.).unwrap(),
            placed_sphere(2., [0., 0., 9.]),
        ),
        (
            "axial wall and cap",
            cylinder(3., 10.).unwrap(),
            placed_sphere(4., [0., 0., 8.]),
        ),
        (
            "axial cap seam",
            cylinder(3., 10.).unwrap(),
            placed_sphere(2., [0., 0., 10.]),
        ),
        ("face seam", big.clone(), placed_sphere(5., [10., 0., 0.])),
        ("edge poles", big.clone(), placed_sphere(4., [10., 10., 0.])),
        (
            "corner poles",
            big.clone(),
            placed_sphere(4., [10., 10., 10.]),
        ),
        ("split seam", big, placed_sphere(5., [10., 7., 0.])),
        (
            "sphere sphere",
            sphere(3.).unwrap(),
            placed_sphere(3., [4.1, 0.3, -0.2]),
        ),
        (
            "sphere bulge",
            sphere(5.).unwrap(),
            placed_sphere(1.5, [4.6, 0.7, 0.9]),
        ),
        // Tolerant fallback results (numerical SSI).
        (
            "tolerant off-axis",
            cylinder(3., 10.).unwrap(),
            placed_sphere(2., [2.5, 0.4, 5.]),
        ),
        (
            "tolerant box sphere",
            cuboid([-4., -4., -4.], [4., 4., 4.]).unwrap(),
            placed_sphere(3., [3.1, 2.7, 2.9]),
        ),
        (
            "tolerant two stubs",
            cylinder(3., 10.).unwrap(),
            placed_sphere(4., [0.2, 0.1, 5.]),
        ),
        (
            "tolerant drilled sphere",
            sphere(5.).unwrap(),
            transform::affine(
                &cylinder(1., 20.).unwrap(),
                [
                    [1., 0., 0., 0.3],
                    [0., 1., 0., 0.2],
                    [0., 0., 1., -10.],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap(),
        ),
        (
            "tolerant crossing cylinders",
            cylinder(2., 12.).unwrap(),
            transform::affine(
                &cylinder(1.2, 12.).unwrap(),
                [
                    [1., 0., 0., 0.3],
                    [0., 0., -1., 6.],
                    [0., 1., 0., 6.],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap(),
        ),
        (
            "tolerant torus box",
            cuboid([0., 0., 0.], [10., 10., 10.]).unwrap(),
            transform::affine(
                &brep_core::torus(4., 1.).unwrap(),
                [
                    [1., 0., 0., 0.3],
                    [0., 1., 0., 0.7],
                    [0., 0., 1., 0.4],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap(),
        ),
    ];
    for (name, b, s) in &cases {
        for operation in ["union", "difference", "intersection"] {
            let model = operations::boolean(b, s, operation)
                .unwrap_or_else(|e| panic!("{name} {operation}: {e:?}"));
            for segments in [2usize, 4, 8] {
                let t = nurbs(&model, segments)
                    .unwrap_or_else(|e| panic!("{name} {operation} lod {segments}: {e:?}"));
                assert!(
                    t.built.report.closed,
                    "{name} {operation} lod {segments} closed"
                );
                assert!(
                    t.built.report.signed_volume_mm3 > 0.,
                    "{name} {operation} lod {segments} volume"
                );
                // The mesh samples the B-rep: it must reach every vertex and
                // never leave the control hull (curved edges and trimmed
                // faces may legitimately extend past the vertices).
                let mesh = aabb(t.built.mesh.positions.chunks(3).map(|p| [p[0], p[1], p[2]]));
                let verts = aabb(model.vertices.iter().map(|v| v.point));
                let hull = aabb(
                    model
                        .vertices
                        .iter()
                        .map(|v| v.point)
                        .chain(model.edges.iter().flat_map(|e| {
                            e.curve.control_points.iter().map(|p| [p[0], p[1], p[2]])
                        }))
                        .chain(model.faces.iter().flat_map(|f| {
                            f.surface
                                .control_points
                                .iter()
                                .flatten()
                                .map(|p| [p[0], p[1], p[2]])
                        })),
                );
                for axis in 0..3 {
                    let (lo, hi) = (2 * axis, 2 * axis + 1);
                    assert!(
                        mesh[lo] >= hull[lo] - 1e-9 && mesh[hi] <= hull[hi] + 1e-9,
                        "{name} {operation} lod {segments}: mesh {mesh:?} leaves hull {hull:?}"
                    );
                    assert!(
                        mesh[lo] <= verts[lo] + 1e-3 && mesh[hi] >= verts[hi] - 1e-3,
                        "{name} {operation} lod {segments}: mesh {mesh:?} misses vertices {verts:?}"
                    );
                }
            }
        }
    }
}
