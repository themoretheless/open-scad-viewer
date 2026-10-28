use super::*;

#[test]
fn snap_grid_and_intersection() {
    let g = snap_to_grid([1.2, 3.8], 1.).unwrap();
    assert_eq!(g, [1., 4.]);
    let a = BezierPath::from_polyline(&[[0., 0.], [4., 4.]], false).unwrap();
    let b = BezierPath::from_polyline(&[[0., 4.], [4., 0.]], false).unwrap();
    let hit = find_snap([2.05, 2.0], 0.5, None, &[a, b]).unwrap().unwrap();
    assert_eq!(hit.kind, SnapKind::Intersection);
    assert!(dist(hit.point, [2., 2.]) < 1e-6);
}

#[test]
fn align_and_distribute() {
    let boxes = [
        BBox {
            min: [0., 0.],
            max: [1., 1.],
        },
        BBox {
            min: [5., 2.],
            max: [6., 3.],
        },
        BBox {
            min: [2., 4.],
            max: [3., 5.],
        },
    ];
    let ref_bb = boxes.iter().copied().reduce(BBox::union).unwrap();
    let d = align_boxes(&boxes, ref_bb, Some(HAlign::Left), None);
    assert!((boxes[0].min[0] + d[0][0] - ref_bb.min[0]).abs() < 1e-9);
    let dist = distribute_centers(&boxes, true).unwrap();
    assert_eq!(dist.len(), 3);
    let edges = distribute_objects(&boxes, true, DistributeAnchor::LeftOrTop).unwrap();
    assert_eq!(edges.len(), 3);
    let page = BBox {
        min: [0., 0.],
        max: [20., 20.],
    };
    let key = align_boxes_relative(
        &boxes,
        RelativeTo::KeyObject,
        Some(0),
        None,
        None,
        Some(HAlign::Left),
        None,
    )
    .unwrap();
    assert!((boxes[1].min[0] + key[1][0] - boxes[0].min[0]).abs() < 1e-9);
    let packed = distribute_spacing(&boxes, true, 1.).unwrap();
    assert_eq!(packed.len(), 3);
    let _ = page;
}

#[test]
fn angle_snap_and_guides() {
    let p = snap_to_45([0., 0.], [10., 1.]).unwrap();
    assert!((p[1]).abs() < 1e-9);
    let moving = BBox {
        min: [0.1, 0.],
        max: [1.1, 1.],
    };
    let target = BBox {
        min: [0., 4.],
        max: [2., 5.],
    };
    let g = drag_align_guides(moving, &[target], 0.2).unwrap();
    assert!((g.delta[0] + 0.1).abs() < 1e-9);
    assert!(g.guides.iter().any(|l| l.vertical));
}

#[test]
fn scissors_and_knife() {
    let path = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
    let parts = scissors_cut(&path, [5., 0.1], 0.5).unwrap();
    assert_eq!(parts.len(), 2);
    let knife = knife_cut(
        &BezierPath::from_polyline(&[[0., 0.], [10., 0.], [10., 10.]], false).unwrap(),
        [5., -1.],
        [5., 1.],
    )
    .unwrap();
    assert!(knife.len() >= 2);
}

#[test]
fn measure_segment() {
    let m = measure([0., 0.], [3., 4.]).unwrap();
    assert!((m.distance - 5.).abs() < 1e-12);
    assert!((m.angle_deg - 53.130102).abs() < 1e-4);
}
