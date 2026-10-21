use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn square(lo: f64, hi: f64) -> Vec<Curve> {
    let v = [[lo, lo], [hi, lo], [hi, hi], [lo, hi], [lo, lo]];
    v.windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect()
}
fn group(loops: &[Vec<Curve>], weights: &[i32]) -> p::OwnedOriginalFilledRegions {
    let owner = Arc::new(
        p::assemble_original_filled_boundaries(loops, weights, 1e-12, Default::default()).unwrap(),
    );
    let regions = p::own_original_filled_regions(owner, Default::default()).unwrap();
    regions.reverify_source().unwrap();
    regions
}
#[test]
fn exact_hole_is_grouped_with_its_material_outer_component() {
    let r = group(&[square(0., 4.), square(1., 3.)], &[1, -1]);
    assert_eq!(
        r.regions(),
        [p::OriginalFilledRegion {
            outer: 0,
            holes: vec![1]
        }]
    );
    assert_eq!(r.left_containment(), [vec![1, 0], vec![1, 0]]);
    assert_eq!(r.right_containment(), [vec![0, 0], vec![1, -1]]);
}
#[test]
fn nested_island_owns_its_inner_hole_instead_of_the_distant_outer() {
    let r = group(
        &[
            square(0., 8.),
            square(1., 7.),
            square(2., 6.),
            square(3., 5.),
        ],
        &[1, -1, 1, -1],
    );
    assert_eq!(
        r.regions(),
        [
            p::OriginalFilledRegion {
                outer: 0,
                holes: vec![1]
            },
            p::OriginalFilledRegion {
                outer: 2,
                holes: vec![3]
            }
        ]
    );
}
#[test]
fn disconnected_and_overlapping_original_components_are_distinguished() {
    let separate = group(&[square(0., 1.), square(2., 3.)], &[1, 1]);
    assert_eq!(separate.regions().len(), 2);
    assert!(separate.regions().iter().all(|r| r.holes.is_empty()));
    let union = group(&[square(0., 2.), square(1., 3.)], &[1, 1]);
    assert_eq!(union.regions().len(), 1);
    assert!(union.regions()[0].holes.is_empty());
}
#[test]
fn automatic_curved_region_keeps_irrational_original_carrier_cuts() {
    let arch = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1.; 3],
        control_points: vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]],
        periodic: false,
    };
    let base = Curve::from_polyline(vec![vec![3., 0.], vec![-3., 0.]]).unwrap();
    let points = [[-4., 1.5], [4., 1.5], [4., 4.], [-4., 4.], [-4., 1.5]];
    let wire = points
        .windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect();
    let r = p::assemble_original_filled_regions(
        &[vec![arch, base], wire],
        &[-1, 1],
        1e-12,
        Default::default(),
        Default::default(),
    )
    .unwrap();
    r.reverify_source().unwrap();
    assert_eq!(r.regions().len(), 1);
    assert!(r.regions()[0].holes.is_empty());
    assert_eq!(
        r.owner().cycles().graph().owner().atlas().sources()[0].degree,
        2
    );
}
