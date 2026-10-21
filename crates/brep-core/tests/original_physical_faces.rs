use brep_core::profile_certificate as p;
use nurbs_core::{
    curve::Curve,
    surface::{self, Surface},
};
fn plane(chart: [[f64; 2]; 2]) -> Surface {
    let [[a, b], [c, d]] = chart;
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![a, a, b, b],
        knots_v: vec![c, c, d, d],
        weights: vec![vec![1.; 2]; 2],
        control_points: vec![
            vec![vec![a, c, 0.], vec![a, d, 0.]],
            vec![vec![b, c, 0.], vec![b, d, 0.]],
        ],
        periodic_u: false,
        periodic_v: false,
    }
}
fn rectangle(lo: [f64; 2], hi: [f64; 2]) -> Vec<Curve> {
    let v = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]], lo];
    v.windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect()
}
fn tube(repeat: bool) -> Surface {
    let mut v = vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ];
    if repeat {
        v.extend([
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ]);
    }
    surface::extrude(&Curve::from_polyline(v).unwrap(), [0., 0., 1.]).unwrap()
}
#[test]
fn planar_annulus_publishes_one_physical_face_with_exact_hole_patch() {
    let surface = plane([[0., 1.], [0., 1.]]);
    let loops = [
        rectangle([0., 0.], [1., 1.]),
        rectangle([0.25, 0.25], [0.75, 0.75]),
    ];
    let f = p::assemble_original_physical_faces(
        &surface,
        &loops,
        &[1, -1],
        [false, false],
        1e-12,
        Default::default(),
    )
    .unwrap();
    f.reverify_source().unwrap();
    assert_eq!(f.surface(), &surface);
    assert_eq!(f.faces().len(), 1);
    assert_eq!(f.faces()[0].boundary_cycles.len(), 2);
    assert_eq!(f.faces()[0].euler_characteristic, 0);
    assert_eq!(f.patches()[0].holes.len(), 1);
    assert_eq!(f.edges().len(), 8);
    assert!(f.edges().iter().all(|e| e.uses.len() == 1
        && e.traces.len() == 1
        && e.vertices.iter().all(|i| *i < f.vertices().len())));
}
#[test]
fn original_cylinder_owns_shared_physical_seam_and_loop_edges() {
    let surface = tube(false);
    let f = p::assemble_original_physical_faces(
        &surface,
        &[rectangle([0., 0.], [4., 1.])],
        &[1],
        [true, false],
        1e-12,
        Default::default(),
    )
    .unwrap();
    f.reverify_source().unwrap();
    assert_eq!(f.faces().len(), 1);
    assert_eq!(f.faces()[0].boundary_cycles.len(), 2);
    assert_eq!(f.vertices().len(), 2);
    assert_eq!(f.edges().len(), 3);
    let seam = f.edges().iter().find(|e| e.uses.len() == 2).unwrap();
    assert_eq!(seam.traces.len(), 2);
    assert_ne!(seam.uses[0].reversed, seam.uses[1].reversed);
    assert!(f.edges().iter().any(|e| e.vertices[0] == e.vertices[1]));
}
#[test]
fn irrational_original_trim_cuts_remain_owned_world_composition_traces() {
    let arch = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1.; 3],
        control_points: vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]],
        periodic: false,
    };
    let base = Curve::from_polyline(vec![vec![3., 0.], vec![-3., 0.]]).unwrap();
    let loops = [vec![arch.clone(), base], rectangle([-4., 1.5], [4., 4.])];
    let f = p::assemble_original_physical_faces(
        &plane([[-4., 4.], [0., 4.]]),
        &loops,
        &[-1, 1],
        [false, false],
        1e-12,
        Default::default(),
    )
    .unwrap();
    f.reverify_source().unwrap();
    assert_eq!(f.faces().len(), 1);
    assert_eq!(f.sources()[0], arch);
    assert!(f.edges().iter().flat_map(|e| &e.traces).any(|t| {
        t.source == 0
            && t.cuts
                .iter()
                .any(|c| matches!(c, p::OriginalCarrierCut::Contact { .. }))
    }));
}
#[test]
fn separated_uv_patches_publish_one_connected_physical_face_through_shore() {
    let f = p::assemble_original_physical_faces(
        &tube(false),
        &[
            rectangle([0., 0.25], [1., 0.75]),
            rectangle([3., 0.25], [4., 0.75]),
        ],
        &[1, 1],
        [true, false],
        1e-12,
        Default::default(),
    )
    .unwrap();
    f.reverify_source().unwrap();
    assert_eq!(f.faces().len(), 1);
    assert_eq!(f.faces()[0].patches.len(), 2);
    assert_eq!(f.faces()[0].boundary_cycles.len(), 1);
    assert_eq!(f.faces()[0].euler_characteristic, 1);
}
#[test]
fn global_double_cover_cannot_publish_physical_faces() {
    let limits = p::OriginalPhysicalFaceLimits {
        pairs: p::OriginalAtlasPairLimits {
            max_pair_depth: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(
        p::assemble_original_physical_faces(
            &tube(true),
            &[rectangle([0., 0.], [8., 1.])],
            &[1],
            [true, false],
            1e-12,
            limits
        )
        .is_err()
    );
}
#[test]
fn zero_material_multiplicity_publishes_no_artificial_physical_geometry() {
    let surface = plane([[0., 1.], [0., 1.]]);
    let f = p::assemble_original_physical_faces(
        &surface,
        &[rectangle([0., 0.], [1., 1.])],
        &[0],
        [false, false],
        1e-12,
        Default::default(),
    )
    .unwrap();
    f.reverify_source().unwrap();
    assert!(f.faces().is_empty());
    assert!(f.patches().is_empty());
    assert!(f.vertices().is_empty());
    assert!(f.edges().is_empty());
}
