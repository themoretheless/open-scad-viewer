use nurbs_core::{
    shared_tessellation::{self, Seam, Side},
    surface::Surface,
};
fn patch(x: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![x, 0., 0.], vec![x, 1., 0.]],
            vec![vec![x + 1., 0., 0.], vec![x + 1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn seam(reversed: bool) -> Seam {
    Seam {
        patches: [0, 1],
        sides: [Side::U1, Side::U0],
        reversed,
    }
}
#[test]
fn different_knot_layouts_share_identical_indexed_seam_vertices() {
    let left = patch(0.);
    let mut right = patch(1.);
    right.knots_v = vec![0., 0., 0.3, 1., 1.];
    for row in &mut right.control_points {
        let mut middle = row[0].clone();
        middle[1] = 0.3;
        row.insert(1, middle);
    }
    right.weights = vec![vec![1.; 3]; 2];
    let input = vec![left, right];
    let before = input.clone();
    let r = shared_tessellation::tessellate(&input, &[seam(false)], 8, 0.3, 10000, 10000).unwrap();
    assert!(r.within_tolerance);
    assert!(r.cells <= 10000 && r.agreement_cells <= 10000);
    let mesh = r.mesh.unwrap();
    assert_eq!(mesh.points.len(), 153);
    let vertices = |patch| {
        mesh.triangles
            .iter()
            .filter(|t| t.patch == patch)
            .flat_map(|t| t.vertices)
            .collect::<std::collections::BTreeSet<_>>()
    };
    let a = vertices(0);
    let b = vertices(1);
    let shared: Vec<_> = a.intersection(&b).collect();
    assert_eq!(shared.len(), 9);
    for &&id in &shared {
        assert_eq!(mesh.points[id][0], 1.);
    }
    assert_eq!(input, before);
}
#[test]
fn reversed_correspondence_is_welded_without_cracks() {
    let left = patch(0.);
    let mut right = patch(1.);
    for row in &mut right.control_points {
        row.reverse();
    }
    let r = shared_tessellation::tessellate(&[left, right], &[seam(true)], 4, 0.5, 10000, 10000)
        .unwrap();
    assert!(r.within_tolerance);
    assert_eq!(r.mesh.unwrap().points.len(), 45);
}
#[test]
fn mismatched_seam_or_insufficient_geometric_accuracy_never_passes() {
    let input = vec![patch(0.), patch(1.1)];
    let r = shared_tessellation::tessellate(&input, &[seam(false)], 4, 1e-5, 10000, 10000).unwrap();
    assert!(!r.within_tolerance);
    assert!(r.mesh.is_none());
    let input = vec![patch(0.), patch(1.)];
    let coarse =
        shared_tessellation::tessellate(&input, &[seam(false)], 1, 1e-5, 10000, 10000).unwrap();
    assert!(!coarse.within_tolerance);
    assert!(coarse.mesh.is_some());
    assert!(shared_tessellation::tessellate(&input, &[seam(false)], 8, 0.1, 1, 10000).is_err());
    assert!(shared_tessellation::tessellate(&input, &[seam(false)], 0, 0.1, 10000, 10000).is_err());
}
