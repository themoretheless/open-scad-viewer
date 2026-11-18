use super::*;
#[test]
fn identical_coordinates_do_not_replace_shared_edge_ownership() {
    let m = crate::cuboid([0.; 3], [10.; 3]).unwrap();
    let pair = [0, 2];
    let mut work = Work {
        used: 0,
        max: 100000,
        exhausted: false,
    };
    assert!(certify(&m, pair, &mut work));
    let mut changed = m.clone();
    let shared = edges(&m, pair[0])
        .intersection(&edges(&m, pair[1]))
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(shared.len(), 1);
    let copy = changed.edges.len();
    let duplicated = changed.edges[shared[0]].clone();
    changed.edges.push(duplicated);
    let wire = changed.faces[pair[0]].outer;
    for c in &mut changed.loops[wire].coedges {
        if c.edge == shared[0] {
            c.edge = copy;
        }
    }
    let mut work = Work {
        used: 0,
        max: 100000,
        exhausted: false,
    };
    assert!(!certify(&changed, pair, &mut work));
}
#[test]
fn an_internal_zero_grid_line_is_not_a_natural_boundary() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 2., 2.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [1., 0., 1.]
            .into_iter()
            .enumerate()
            .map(|(i, z)| vec![vec![i as f64, 0., z], vec![i as f64, 1., z]])
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    s.validate().unwrap();
    let mut work = Work {
        used: 0,
        max: 1000,
        exhausted: false,
    };
    assert!(!interior_strict(&s, 2, 0., &mut work));
}
#[test]
fn coordinate_contact_work_exhaustion_cannot_certify() {
    let m = crate::cuboid([0.; 3], [10.; 3]).unwrap();
    let mut work = Work {
        used: 0,
        max: 0,
        exhausted: false,
    };
    assert!(!certify(&m, [0, 2], &mut work));
    assert_eq!(work.used, 0);
    assert!(work.exhausted);
}
