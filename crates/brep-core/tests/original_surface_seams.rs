use brep_core::profile_certificate as p;
use nurbs_core::{
    curve::Curve,
    periodic_trim::{SurfaceSeamProof, verify_surface_seam_identity},
    surface::{self, Surface},
};
use std::sync::Arc;
fn tube() -> Surface {
    let c = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    surface::extrude(&c, [0., 0., 2.]).unwrap()
}
fn dictionary() -> Arc<p::OwnedOriginalSeamDictionary> {
    let v = [[0., 0.], [4., 0.], [4., 1.], [0., 1.], [0., 0.]];
    let wire = v
        .windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect();
    let regions = Arc::new(
        p::assemble_original_filled_regions(
            &[wire],
            &[1],
            1e-12,
            Default::default(),
            Default::default(),
        )
        .unwrap(),
    );
    let chart = Arc::new(
        p::own_original_chart_vertices(
            regions.owner().cycles().graph().clone(),
            [[0., 4.], [0., 1.]],
            [true, false],
            Default::default(),
        )
        .unwrap(),
    );
    Arc::new(p::own_original_seam_dictionary(regions, chart, Default::default()).unwrap())
}
#[test]
fn complete_original_uv_pair_binds_to_actual_exact_surface_boundary_functions() {
    let surface = tube();
    let owner = p::bind_original_seams_to_surface(dictionary(), &surface).unwrap();
    owner.reverify_source().unwrap();
    assert_eq!(owner.surface(), &surface);
    assert_eq!(
        owner.proofs(),
        &[Some(SurfaceSeamProof::ClampedBoundaryIdentity), None]
    );
}
#[test]
fn exact_weight_gauge_is_allowed_but_tiny_boundary_geometry_changes_are_not() {
    let mut surface = tube();
    let n = surface.weights.len();
    for w in &mut surface.weights[n - 1] {
        *w *= 2.;
    }
    assert_eq!(
        verify_surface_seam_identity(&surface, 0).unwrap(),
        Some(SurfaceSeamProof::ClampedBoundaryIdentity)
    );
    let owner = p::bind_original_seams_to_surface(dictionary(), &surface).unwrap();
    owner.reverify_source().unwrap();
    surface.control_points[n - 1][0][0] += 2f64.powi(-40);
    assert_eq!(verify_surface_seam_identity(&surface, 0).unwrap(), None);
    assert!(p::bind_original_seams_to_surface(owner.dictionary().clone(), &surface).is_err());
}
#[test]
fn source_validation_axis_and_actual_domain_are_independent_obligations() {
    let surface = tube();
    assert!(verify_surface_seam_identity(&surface, 2).is_err());
    let mut invalid = surface.clone();
    invalid.periodic_u = true;
    assert!(verify_surface_seam_identity(&invalid, 0).is_err());
    let mut wrong = surface.clone();
    for u in &mut wrong.knots_u {
        *u *= 2.;
    }
    assert!(p::bind_original_seams_to_surface(dictionary(), &wrong).is_err());
    let mut malformed = surface;
    malformed.control_points.clear();
    assert!(verify_surface_seam_identity(&malformed, 0).is_err());
}
#[test]
fn actual_periodic_storage_identity_is_verified_from_controls_and_knots() {
    let mut surface = tube();
    surface.periodic_u = true;
    surface.knots_u = vec![-1., 0., 1., 2., 3., 4., 5.];
    assert_eq!(
        verify_surface_seam_identity(&surface, 0).unwrap(),
        Some(SurfaceSeamProof::PeriodicStorageIdentity)
    );
    let owner = p::bind_original_seams_to_surface(dictionary(), &surface).unwrap();
    owner.reverify_source().unwrap();
    assert_eq!(
        owner.proofs()[0],
        Some(SurfaceSeamProof::PeriodicStorageIdentity)
    );
    surface.knots_u[0] -= 0.125;
    assert!(p::bind_original_seams_to_surface(owner.dictionary().clone(), &surface).is_err());
}
