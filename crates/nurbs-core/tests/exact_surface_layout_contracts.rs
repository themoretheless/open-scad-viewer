use nurbs_core::{
    editing::exact_surface_merge::{
        GridLayoutRefusal, infer_surface_grid_layout, merge_surface_grid,
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn tile(a: f64, b: f64, c: f64, d: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![a, a, b, b],
        knots_v: vec![c, c, d, d],
        control_points: [a, b]
            .map(|u| [c, d].map(|v| vec![u, v, u + 2. * v]).to_vec())
            .to_vec(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn shuffled_nonuniform_tiles_restore_the_order_for_exact_merging() {
    let tiles = [
        tile(0., 3., 1., 4.),
        tile(-2., -0., -1., 1.),
        tile(-2., 0., 1., 4.),
        tile(0., 3., -1., 1.),
    ];
    let audit = infer_surface_grid_layout(&tiles, WORK).unwrap();
    let layout = audit.layout.unwrap();
    assert_eq!(layout.u_breaks, [-2., 0., 3.]);
    assert_eq!(layout.v_breaks, [-1., 1., 4.]);
    assert_eq!(layout.order, [1, 3, 2, 0]);
    let ordered = layout
        .order
        .iter()
        .map(|&index| tiles[index].clone())
        .collect::<Vec<_>>();
    let result = merge_surface_grid(&ordered, 2, WORK - audit.work_used).unwrap();
    let surface = result.surface.unwrap();
    assert_eq!(surface.control_points[1][1], [3., 4., 11.]);
    assert_eq!(surface.knots_u, [-2., -2., 3., 3.]);
    assert_eq!(surface.knots_v, [-1., -1., 4., 4.]);
}

#[test]
fn sparse_occupancy_distinguishes_holes_duplicates_and_t_junctions() {
    let a = tile(0., 1., 0., 1.);
    let b = tile(1., 2., 0., 1.);
    let c = tile(0., 1., 1., 2.);
    let d = tile(1., 2., 1., 2.);
    let gap = infer_surface_grid_layout(&[a.clone(), b.clone(), d.clone()], WORK).unwrap();
    assert_eq!(gap.refusal, Some(GridLayoutRefusal::MissingCells));
    let duplicate =
        infer_surface_grid_layout(&[a.clone(), b, c.clone(), d.clone(), a], WORK).unwrap();
    assert_eq!(
        duplicate.refusal,
        Some(GridLayoutRefusal::DuplicateCell {
            first: 0,
            second: 4
        })
    );
    let junction = infer_surface_grid_layout(&[tile(0., 2., 0., 1.), c, d], WORK).unwrap();
    assert_eq!(
        junction.refusal,
        Some(GridLayoutRefusal::SpansMultipleCells { input: 0 })
    );
}

#[test]
fn layout_budget_is_atomic() {
    let tiles = [
        tile(0., 1., 0., 1.),
        tile(1., 2., 0., 1.),
        tile(0., 1., 1., 2.),
        tile(1., 2., 1., 2.),
    ];
    let full = infer_surface_grid_layout(&tiles, 16).unwrap();
    assert_eq!(full.work_used, 16);
    assert!(full.layout.is_some());
    let short = infer_surface_grid_layout(&tiles, 15).unwrap();
    assert_eq!(short.refusal, Some(GridLayoutRefusal::ResourceLimit));
    assert_eq!(short.work_used, 12);
    assert!(short.layout.is_none());
}
