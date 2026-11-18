use nurbs_core::{
    editing::exact_surface_merge::{GridRefusal, merge_surface_grid},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn grid(z: impl Fn(f64, f64) -> f64) -> Vec<Surface> {
    let mut tiles = Vec::new();
    for j in 0..2 {
        for i in 0..2 {
            let [a, b, c, d] = [i as f64, i as f64 + 1., j as f64, j as f64 + 1.];
            tiles.push(Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![a, a, b, b],
                knots_v: vec![c, c, d, d],
                control_points: [a, b]
                    .map(|u| [c, d].map(|v| vec![u, v, z(u, v)]).to_vec())
                    .to_vec(),
                weights: vec![vec![1.; 2]; 2],
                periodic_u: false,
                periodic_v: false,
            });
        }
    }
    tiles
}

#[test]
fn coplanar_grid_merges_without_any_remaining_seams() {
    let tiles = grid(|u, v| u + 2. * v);
    let r = merge_surface_grid(&tiles, 2, WORK).unwrap();
    assert_eq!(r.inserted_knots_for_alignment, 0);
    let s = r.surface.unwrap();
    assert_eq!(s.knots_u, [0., 0., 2., 2.]);
    assert_eq!(s.knots_v, [0., 0., 2., 2.]);
    assert_eq!(s.control_points[1][1], [2., 2., 6.]);
}

#[test]
fn restores_the_common_basis_when_only_one_row_needs_an_interior_knot() {
    let law = |u: f64, v: f64| (u - 1.).max(0.) * (v - 1.).max(0.);
    let tiles = grid(law);
    let r = merge_surface_grid(&tiles, 2, WORK).unwrap();
    assert_eq!(r.inserted_knots_for_alignment, 1, "{r:?}");
    let s = r.surface.unwrap_or_else(|| panic!("{:?}", r.refusal));
    assert_eq!(s.knots_u, [0., 0., 1., 2., 2.]);
    assert_eq!(s.knots_v, [0., 0., 1., 2., 2.]);
    for u in [0., 0.25, 1., 1.5, 2.] {
        for v in [0., 0.25, 1., 1.5, 2.] {
            let result = s.evaluate(u, v).unwrap().point;
            assert!((result[2] - law(u, v)).abs() < 1e-14);
        }
    }
}

#[test]
fn late_cross_row_basis_mismatch_withholds_the_entire_grid() {
    let mut tiles = grid(|u, v| u + v);
    for tile in &mut tiles[2..] {
        for knot in &mut tile.knots_u {
            *knot += 1.;
        }
    }
    let r = merge_surface_grid(&tiles, 2, WORK).unwrap();
    assert!(r.surface.is_none());
    assert_eq!(r.refusal, Some(GridRefusal::CrossRowBasis { index: 1 }));
    assert!(merge_surface_grid(&tiles[..3], 2, WORK).is_err());
    let short = merge_surface_grid(&tiles, 2, 1).unwrap();
    assert!(short.surface.is_none() && short.refusal.is_some());
}

#[test]
fn signed_zero_is_one_exact_knot_across_rows() {
    let mut tiles = grid(|u, _| (u - 1.).max(0.));
    for (index, tile) in tiles.iter_mut().enumerate() {
        for knot in &mut tile.knots_u {
            *knot -= 1.;
            if index < 2 && *knot == 0. {
                *knot = -0.;
            }
        }
    }
    let result = merge_surface_grid(&tiles, 2, WORK).unwrap();
    assert_eq!(result.inserted_knots_for_alignment, 0);
    let surface = result
        .surface
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(surface.knots_u, [-1., -1., 0., 1., 1.]);
}
