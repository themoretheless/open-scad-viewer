use nurbs_core::{
    surface::Surface,
    surface_tessellation,
    tessellation::shading::{self, NormalPolicy, NormalSource, Orientation},
};

fn plane() -> Surface {
    Surface {
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
    }
}

fn curved_panel() -> Surface {
    let mut surface = plane();
    surface.control_points[1][1][2] = 1.;
    surface
}

fn triangular_pole() -> Surface {
    // A C0 knot has no unique two-sided analytic normal on its shared mesh
    // line, while the adjacent geometric triangles remain nondegenerate.
    Surface {
        knots_u: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        ..plane()
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[test]
fn analytic_normals_and_winding_follow_face_orientation() {
    let surface = curved_panel();
    let report = surface_tessellation::tessellate(&surface, 100., 100).unwrap();
    let natural = shading::shade(
        &surface,
        &report,
        Orientation::Natural,
        NormalPolicy::RefuseDegenerate,
    )
    .unwrap();
    assert!(
        natural
            .normals
            .iter()
            .all(|n| n.source == NormalSource::Analytic)
    );
    assert!(natural.normals.iter().any(|n| n.vector[0].abs() > 1e-6));
    let reversed = shading::shade(
        &surface,
        &report,
        Orientation::Reversed,
        NormalPolicy::RefuseDegenerate,
    )
    .unwrap();
    for (a, b) in natural.normals.iter().zip(&reversed.normals) {
        assert_eq!(a.source, NormalSource::Analytic);
        assert_eq!(b.source, NormalSource::Analytic);
        for axis in 0..3 {
            assert!((a.vector[axis] + b.vector[axis]).abs() < 1e-12);
        }
    }
    for (normal, reversed) in natural.triangles.iter().zip(&reversed.triangles) {
        assert_eq!(
            normal.vertices,
            [
                reversed.vertices[0],
                reversed.vertices[2],
                reversed.vertices[1]
            ]
        );
    }
    for shaded in [&natural, &reversed] {
        for triangle in &shaded.triangles {
            let [a, b, c] = triangle.vertices.map(|i| shaded.points[i]);
            let geometric = cross(
                std::array::from_fn(|k| b[k] - a[k]),
                std::array::from_fn(|k| c[k] - a[k]),
            );
            let sign = geometric[2];
            match shaded.normals[0].vector[2] >= 0. {
                true => assert!(sign > 0.),
                false => assert!(sign < 0.),
            }
        }
    }
}

#[test]
fn singular_vertices_have_typed_refusal_or_explicit_mesh_fallback() {
    let surface = triangular_pole();
    let report = surface_tessellation::tessellate(&surface, 100., 100).unwrap();
    let refusal = shading::shade(
        &surface,
        &report,
        Orientation::Natural,
        NormalPolicy::RefuseDegenerate,
    )
    .unwrap_err();
    assert_eq!(refusal.code, "NURBS_ANALYTIC_NORMAL_UNAVAILABLE");
    let shaded = shading::shade(
        &surface,
        &report,
        Orientation::Natural,
        NormalPolicy::MeshFallback,
    )
    .unwrap();
    assert!(
        shaded
            .normals
            .iter()
            .any(|n| n.source == NormalSource::MeshFallback)
    );
    assert!(
        shaded
            .normals
            .iter()
            .any(|n| n.source == NormalSource::Analytic)
    );
    for normal in shaded.normals {
        let length = normal.vector.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((length - 1.).abs() < 1e-12);
    }
}

#[test]
fn fully_collapsed_surface_cannot_fake_a_fallback_normal() {
    let mut surface = plane();
    for point in surface.control_points.iter_mut().flatten() {
        *point = vec![0.; 3];
    }
    let report = surface_tessellation::tessellate(&surface, 100., 100).unwrap();
    let error = shading::shade(
        &surface,
        &report,
        Orientation::Reversed,
        NormalPolicy::MeshFallback,
    )
    .unwrap_err();
    assert_eq!(error.code, "NURBS_MESH_NORMAL_UNAVAILABLE");
}

#[test]
fn collapsed_cone_row_uses_analytic_limit_without_winding_flip() {
    let cone = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![0., 1., 0.], vec![-1., 0., 0.]],
            vec![vec![0., 0., 1.]; 3],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let report = surface_tessellation::tessellate(&cone, 100., 100).unwrap();
    let shaded = shading::shade(
        &cone,
        &report,
        Orientation::Natural,
        NormalPolicy::RefuseDegenerate,
    )
    .unwrap();
    assert!(
        shaded
            .normals
            .iter()
            .all(|n| n.source == NormalSource::Analytic)
    );
    assert!(shaded.normals.iter().any(|n| n.vector[2] > 0.));
    let reversed = shading::shade(
        &cone,
        &report,
        Orientation::Reversed,
        NormalPolicy::RefuseDegenerate,
    )
    .unwrap();
    for (a, b) in shaded.normals.iter().zip(&reversed.normals) {
        for k in 0..3 {
            assert!((a.vector[k] + b.vector[k]).abs() < 1e-12);
        }
    }
}
