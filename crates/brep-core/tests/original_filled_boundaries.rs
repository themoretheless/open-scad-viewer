use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn q(a: i64, b: i64) -> [String; 2] {
    [a.to_string(), b.to_string()]
}
fn square(lo: f64, hi: f64) -> Vec<Curve> {
    let p = [[lo, lo], [hi, lo], [hi, hi], [lo, hi], [lo, lo]];
    p.windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect()
}
fn setup(
    loops: &[Vec<Curve>],
) -> (
    Arc<p::OwnedOriginalCarrierCycles>,
    Vec<p::OwnedOriginalCarrierSides>,
) {
    let sources = loops.iter().flatten().cloned().collect::<Vec<_>>();
    let a = Arc::new(
        p::assemble_original_contact_atlas(&sources, 1e-12, p::ContactAtlasLimits::default())
            .unwrap(),
    );
    let owner = Arc::new(p::own_original_carriers(a).unwrap());
    let graph = Arc::new(p::own_original_carrier_graph(owner.clone()).unwrap());
    let cycles = Arc::new(
        p::own_original_carrier_cycles(graph, p::ContactPortOrderLimits::default()).unwrap(),
    );
    let sides = (0..owner.carriers().len())
        .map(|carrier| {
            [8, 1, 3, 5, 7, 9, 11, 13, 15]
                .iter()
                .find_map(|&t| {
                    p::own_original_carrier_sides(
                        owner.clone(),
                        loops,
                        carrier,
                        q(t, 16),
                        q(1, 4096),
                        192,
                    )
                    .ok()
                })
                .expect("no test side witness")
        })
        .collect();
    (cycles, sides)
}
#[test]
fn nested_nonzero_interiors_remove_only_internal_original_edges() {
    let (cycles, sides) = setup(&[square(0., 4.), square(1., 3.)]);
    let region = p::own_original_filled_boundaries(cycles.clone(), sides.clone(), &[1, 1]).unwrap();
    region.reverify_source().unwrap();
    assert_eq!(region.boundaries().len(), 1);
    assert_eq!(region.retained().len(), 4);
    assert!(region.retained().iter().all(|h| *h / 2 < 4));
    let hole = p::own_original_filled_boundaries(cycles.clone(), sides.clone(), &[1, -1]).unwrap();
    hole.reverify_source().unwrap();
    assert_eq!(hole.boundaries().len(), 2);
    assert_eq!(hole.retained().len(), 8);
    assert!(
        hole.retained()
            .iter()
            .filter(|h| **h / 2 >= 4)
            .all(|h| h % 2 == 1)
    );
    let empty = p::own_original_filled_boundaries(cycles, sides, &[0, 0]).unwrap();
    assert!(empty.boundaries().is_empty());
    assert!(empty.successor().iter().all(Option::is_none));
}
#[test]
fn overlapping_original_squares_bypass_internal_edges_at_crossings() {
    let (cycles, sides) = setup(&[square(0., 2.), square(1., 3.)]);
    let region = p::own_original_filled_boundaries(cycles.clone(), sides.clone(), &[1, 1]).unwrap();
    region.reverify_source().unwrap();
    assert_eq!(region.boundaries().len(), 1);
    assert_eq!(region.retained().len(), 8);
    let mut all = region
        .boundaries()
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    all.sort();
    assert_eq!(all, region.retained());
    let difference = p::own_original_filled_boundaries(cycles, sides, &[1, -1]).unwrap();
    difference.reverify_source().unwrap();
    assert_eq!(difference.boundaries().len(), 2);
    assert_eq!(difference.retained().len(), 12);
}
#[test]
fn incomplete_duplicate_and_unbound_side_receipts_never_qualify() {
    let loops = vec![square(0., 1.)];
    let (cycles, sides) = setup(&loops);
    assert!(p::own_original_filled_boundaries(cycles.clone(), sides[..3].to_vec(), &[1]).is_err());
    let mut duplicate = sides.clone();
    duplicate[1] = duplicate[0].clone();
    assert!(p::own_original_filled_boundaries(cycles.clone(), duplicate, &[1]).is_err());
    let (_, other) = setup(&loops);
    assert!(p::own_original_filled_boundaries(cycles.clone(), other, &[1]).is_err());
    assert!(p::own_original_filled_boundaries(cycles.clone(), sides.clone(), &[]).is_err());
    assert!(p::own_original_filled_boundaries(cycles, sides, &[1_000_001]).is_err());
}
