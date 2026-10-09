use super::*;

#[test]
fn lattice_auto_recommendation_tracks_work_size() {
    assert_eq!(recommended_for_lattice(1_000, 8, 8), Acceleration::Cpu);
    let large = recommended_for_lattice(100_000, 12, 54);
    if cfg!(feature = "cuda") {
        assert_eq!(large, Acceleration::Cuda);
    } else if cfg!(feature = "gpu") {
        assert_eq!(large, Acceleration::Gpu);
    } else {
        assert_eq!(large, Acceleration::Cpu);
    }
}

#[test]
fn lattice_auto_matches_cpu_on_small_grid() {
    let mesh = crate::solid::primitives::cube([20.; 3], false).unwrap();
    let nodes = vec![[5., 5., 5.], [15., 5., 5.], [5., 15., 5.], [5., 5., 15.]];
    let edges = vec![[0, 1], [0, 2], [0, 3]];
    let reference = lattice(
        &mesh,
        nodes.clone(),
        edges.clone(),
        4.,
        0.,
        3.,
        false,
        false,
        0.,
        false,
    )
    .unwrap();
    let automatic = lattice_accelerated(
        &mesh,
        nodes,
        edges,
        4.,
        0.,
        3.,
        false,
        false,
        0.,
        false,
        Acceleration::Auto,
    )
    .unwrap();
    assert_eq!(automatic.mesh.indices.len(), reference.mesh.indices.len());
    assert!(
        (automatic.report.signed_volume_mm3 - reference.report.signed_volume_mm3).abs() < 1e-9
    );
}

#[cfg(feature = "gpu")]
#[test]
fn gpu_lattice_matches_cpu_reference() {
    let mesh = crate::solid::primitives::cube([30.; 3], false).unwrap();
    let nodes = vec![[5., 5., 5.], [25., 5., 5.], [5., 25., 5.], [5., 5., 25.]];
    let edges = vec![[0, 1], [0, 2], [0, 3]];
    let reference = lattice(
        &mesh,
        nodes.clone(),
        edges.clone(),
        2.,
        0.,
        1.,
        false,
        false,
        0.,
        false,
    )
    .unwrap();
    let accelerated = lattice_accelerated(
        &mesh,
        nodes,
        edges,
        2.,
        0.,
        1.,
        false,
        false,
        0.,
        false,
        Acceleration::Gpu,
    )
    .unwrap();
    if crate::lattice_gpu::try_gpu_available() {
        let dt =
            (accelerated.mesh.indices.len() as f64 - reference.mesh.indices.len() as f64).abs();
        assert!(
            dt <= 0.01 * reference.mesh.indices.len() as f64,
            "triangle counts diverge: {} vs {}",
            reference.mesh.indices.len(),
            accelerated.mesh.indices.len()
        );
        let dv = (accelerated.report.signed_volume_mm3 - reference.report.signed_volume_mm3)
            .abs()
            / reference.report.signed_volume_mm3;
        assert!(dv < 0.001, "volume diverges: {dv}");
    }
}

#[cfg(feature = "cuda")]
#[test]
fn cuda_lattice_matches_cpu_reference() {
    let mesh = crate::solid::primitives::cube([30.; 3], false).unwrap();
    let nodes = vec![[5., 5., 5.], [25., 5., 5.], [5., 25., 5.], [5., 5., 25.]];
    let edges = vec![[0, 1], [0, 2], [0, 3]];
    let reference = lattice(
        &mesh,
        nodes.clone(),
        edges.clone(),
        2.,
        0.,
        1.,
        false,
        false,
        0.,
        false,
    )
    .unwrap();
    let accelerated = lattice_accelerated(
        &mesh,
        nodes,
        edges,
        2.,
        0.,
        1.,
        false,
        false,
        0.,
        false,
        Acceleration::Cuda,
    )
    .unwrap();
    if crate::lattice_cuda::try_cuda_available() {
        let dt =
            (accelerated.mesh.indices.len() as f64 - reference.mesh.indices.len() as f64).abs();
        assert!(
            dt <= 0.01 * reference.mesh.indices.len() as f64,
            "triangle counts diverge: {} vs {}",
            reference.mesh.indices.len(),
            accelerated.mesh.indices.len()
        );
        let dv = (accelerated.report.signed_volume_mm3 - reference.report.signed_volume_mm3)
            .abs()
            / reference.report.signed_volume_mm3;
        assert!(dv < 0.001, "volume diverges: {dv}");
    }
}

#[test]
fn bvh_distance_and_sign_agree_with_reference() {
    let mesh = crate::solid::primitives::cube([10.; 3], false).unwrap();
    let triangles = distance_triangles(mesh.view()).unwrap();
    let bvh = Node::build(triangles);
    for i in 0..250 {
        let p = [
            ((i * 73) % 197) as f64 / 10. - 5.,
            ((i * 31) % 193) as f64 / 10. - 5.,
            ((i * 17) % 191) as f64 / 10. - 5.,
        ];
        let actual = bvh.signed_distance(p);
        let expected = crate::solid::proximity::signed_distance(&mesh, p);
        assert!(
            (actual - expected).abs() < 1e-7,
            "{p:?}: {actual} != {expected}"
        )
    }
}
