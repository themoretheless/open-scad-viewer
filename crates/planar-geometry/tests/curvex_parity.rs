//! Behavioral regressions derived from Curvex's owned path/editor operations.
use planar_geometry::{
    corners,
    edit::{self, SnapGeometry, SnapKind},
    path::{BezierPath, HandleLink, HandleSide, PathSegment},
    scissors,
};

#[test]
fn round_corners_uses_cubic_anchors_and_pins_open_endpoints() {
    let curved = BezierPath::open(
        [0., 0.],
        vec![
            PathSegment::Cubic {
                c1: [0., 20.],
                c2: [10., 20.],
                to: [10., 0.],
            },
            PathSegment::Line { to: [10., 10.] },
        ],
    )
    .unwrap();
    let anchors = BezierPath::from_polyline(&[[0., 0.], [10., 0.], [10., 10.]], false).unwrap();
    let rounded = corners::round_corners(&curved, 2.).unwrap();
    assert_eq!(rounded, corners::round_corners(&anchors, 2.).unwrap());
    assert_eq!(rounded.start, curved.start);
    assert_eq!(rounded.segments.last().unwrap().end(), [10., 10.]);
    assert_eq!(rounded.segments.len(), 3);
}

fn compound_area(paths: &[BezierPath]) -> f64 {
    let rings = paths.iter().map(|p| p.to_ring(0.02).unwrap()).collect();
    let mesh = planar_geometry::tessellation::tessellate_rings(
        &rings,
        planar_geometry::tessellation::FillRule::EvenOdd,
    )
    .unwrap();
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|triangle| {
            let [a, b, c] = [0, 1, 2].map(|i| mesh.positions[triangle[i] as usize]);
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
        })
        .sum()
}

#[test]
fn compound_scissors_hits_holes_and_keeps_survivors_grouped() {
    let outer = BezierPath::from_rect([0., 0.], [20., 20.]).unwrap();
    let hole = BezierPath::from_rect([2., 2.], [8., 8.]).unwrap();
    let island = BezierPath::from_rect([4., 4.], [6., 6.]).unwrap();
    let contours = vec![outer.clone(), hole.clone(), island.clone()];
    let hit = scissors::hit_test_compound(&contours, [2., 5.], 0.1)
        .unwrap()
        .unwrap();
    assert_eq!(hit.ring, 1);
    let pieces = scissors::cut_compound_at(&contours, hit).unwrap();
    assert_eq!(pieces[0], vec![outer, island]);
    assert_eq!(pieces.len(), 2);
    assert!(!pieces[1][0].closed);
    assert_eq!(pieces[1][0].start, [2., 5.]);
    assert_eq!(pieces[1][0].segments.last().unwrap().end(), [2., 5.]);
    let hits = scissors::knife_hits_compound(&contours, [1., 5.], [9., 5.]).unwrap();
    assert_eq!(
        hits.iter().map(|h| h.ring).collect::<Vec<_>>(),
        vec![1, 1, 2, 2]
    );
    let pieces = scissors::cut_compound_at_many(&contours, &hits).unwrap();
    assert_eq!(pieces[0], vec![contours[0].clone()]);
    assert_eq!(pieces.len(), 5);
    assert!(pieces[1..].iter().all(|p| p.len() == 1 && !p[0].closed));
}

#[test]
fn compound_knife_assigns_untouched_holes_to_the_containing_side() {
    let outer = BezierPath::from_rect([0., 0.], [20., 20.]).unwrap();
    let hole = BezierPath::from_rect([2., 2.], [8., 8.]).unwrap();
    let pieces =
        scissors::knife_split_compound(&[outer, hole.clone()], [10., -1.], [10., 21.]).unwrap();
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces.iter().filter(|p| p.contains(&hole)).count(), 1);
    let mut areas = pieces.iter().map(|p| compound_area(p)).collect::<Vec<_>>();
    areas.sort_by(f64::total_cmp);
    assert_eq!(areas, vec![164., 200.]);
}

#[test]
fn compound_knife_hole_only_cut_stays_one_compound_with_same_fill() {
    let outer = BezierPath::from_rect([0., 0.], [20., 20.]).unwrap();
    let hole = BezierPath::from_rect([2., 2.], [8., 8.]).unwrap();
    let pieces =
        scissors::knife_split_compound(&[outer.clone(), hole], [1., 5.], [9., 5.]).unwrap();
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].len(), 3);
    assert!(pieces[0].contains(&outer));
    assert!((compound_area(&pieces[0]) - 364.).abs() < 1e-10);
    assert!(pieces[0].iter().all(|p| p.closed));
}

#[test]
fn semantic_snap_has_real_anchors_perpendicular_feet_and_kind_priority() {
    let path = BezierPath::open(
        [0., 0.],
        vec![PathSegment::Cubic {
            c1: [0., 20.],
            c2: [10., 20.],
            to: [10., 0.],
        }],
    )
    .unwrap();
    let flat = path.flatten().unwrap();
    let edge = flat.windows(2).find(|e| e[1][0] == 5.).unwrap();
    let foot = [
        (edge[0][0] + edge[1][0]) * 0.5,
        (edge[0][1] + edge[1][1]) * 0.5,
    ];
    let hit = edit::find_geometry_snap(foot, 0.01, None, &[SnapGeometry::Path(path)])
        .unwrap()
        .unwrap();
    assert_eq!(hit.kind, SnapKind::Perpendicular);
    assert!((hit.point[1] - foot[1]).abs() < 1e-12);
    // Curvex prioritizes the line midpoint over a closer endpoint/foot.
    let hit = edit::find_geometry_snap(
        [4.8, 0.],
        0.3,
        None,
        &[
            SnapGeometry::Line {
                start: [0., 0.],
                end: [10., 0.],
            },
            SnapGeometry::Line {
                start: [4.8, 0.],
                end: [4.8, 10.],
            },
        ],
    )
    .unwrap()
    .unwrap();
    assert_eq!(hit.kind, SnapKind::Midpoint);
    assert_eq!(hit.point, [5., 0.]);
}

#[test]
fn semantic_snap_preserves_primitive_centers_and_closed_vertices() {
    let hit = edit::find_geometry_snap(
        [2., 3.],
        0.1,
        None,
        &[SnapGeometry::Ellipse {
            center: [2., 3.],
            radii: [10., 5.],
        }],
    )
    .unwrap()
    .unwrap();
    assert_eq!(hit.kind, SnapKind::Center);
    let path = BezierPath::from_rect([0., 0.], [10., 10.]).unwrap();
    let hit = edit::find_geometry_snap([0., 0.], 0.1, None, &[SnapGeometry::Path(path)])
        .unwrap()
        .unwrap();
    assert_eq!(hit.kind, SnapKind::Vertex);
}

#[test]
fn joins_preserve_custom_curves_and_ignore_connector_for_welded_endpoints() {
    use planar_geometry::path::join_paths_with_bridge;
    let head = BezierPath::from_polyline(&[[0., 0.], [2., 0.]], false).unwrap();
    let tail = BezierPath::from_polyline(&[[6., 0.], [8., 0.]], false).unwrap();
    let bridge = PathSegment::Cubic {
        c1: [3., 4.],
        c2: [5., 4.],
        to: [6., 0.],
    };
    let joined = join_paths_with_bridge(&head, false, &tail, false, &[bridge], 1e-6).unwrap();
    assert_eq!(joined.segments.len(), 3);
    assert_eq!(joined.segments[1], bridge);
    let near = BezierPath::from_polyline(&[[2.8, 0.8], [8., 0.]], false).unwrap();
    let welded = join_paths_with_bridge(&head, false, &near, false, &[bridge], 1.).unwrap();
    assert_eq!(welded.segments.len(), 2);
}

#[test]
fn open_corner_styles_match_signed_radii_and_pin_endpoints() {
    use corners::CornerStyle;
    let points = [[0., 0.], [10., 0.], [10., 10.]];
    let signed = corners::rounded_open_polyline(&points, &[99., -2., 99.], &[]).unwrap();
    let explicit = corners::rounded_open_polyline(
        &points,
        &[0., 2., 0.],
        &[CornerStyle::Round, CornerStyle::Chamfer],
    )
    .unwrap();
    assert_eq!(signed, explicit);
    assert_eq!(
        signed.flatten().unwrap(),
        vec![[0., 0.], [8., 0.], [10., 2.], [10., 10.]]
    );
    let inverted = corners::rounded_open_polyline(
        &points,
        &[0., 2., 0.],
        &[CornerStyle::Round, CornerStyle::Inverted],
    )
    .unwrap();
    assert_eq!(inverted.start, points[0]);
    assert_eq!(inverted.segments.last().unwrap().end(), points[2]);
    match inverted.segments[1] {
        PathSegment::Cubic { c1, c2, .. } => {
            assert!((c1[0] - 8.).abs() < 1e-12 && c1[1] > 1.);
            assert!(c2[0] < 9. && (c2[1] - 2.).abs() < 1e-12);
        }
        _ => panic!("inverted corner must remain a curve"),
    }
}

#[test]
fn closed_simplify_removes_collinear_seam_without_changing_square() {
    let path = BezierPath::from_polyline(
        &[[5., 0.], [10., 0.], [10., 10.], [0., 10.], [0., 0.]],
        true,
    )
    .unwrap();
    let reduced = path.simplify(0.1).unwrap();
    assert!(reduced.closed);
    assert_eq!(reduced.anchor_count(), 4);
    let mut corners = reduced.to_ring(0.25).unwrap();
    corners.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(corners, vec![[0., 0.], [0., 10.], [10., 0.], [10., 10.]]);
}

#[test]
fn handle_link_validates_index_and_keeps_single_endpoint_handle() {
    let path = BezierPath::open(
        [0., 0.],
        vec![PathSegment::Cubic {
            c1: [1., 2.],
            c2: [3., 2.],
            to: [4., 0.],
        }],
    )
    .unwrap();
    let invalid = std::panic::catch_unwind(|| {
        path.apply_handle_link(99, HandleSide::Out, HandleLink::Mirrored)
    });
    assert!(
        invalid.is_ok(),
        "out-of-range node must return an error, not panic"
    );
    assert!(invalid.unwrap().is_err());
    assert_eq!(
        path.apply_handle_link(0, HandleSide::Out, HandleLink::Mirrored)
            .unwrap(),
        path
    );
    assert_eq!(
        path.apply_handle_link(1, HandleSide::In, HandleLink::Symmetric)
            .unwrap(),
        path
    );
}

#[test]
fn open_parallel_offset_preserves_translated_interior_coordinates() {
    let path = BezierPath::from_polyline(&[[10., 20.], [20., 20.], [30., 20.]], false).unwrap();
    let offset = path.offset(2., "miter", 8).unwrap();
    assert_eq!(
        offset[0].flatten().unwrap(),
        vec![[10., 22.], [20., 22.], [30., 22.]]
    );
}

#[test]
fn tangent_join_extends_forward_rays_and_rejects_backward_intersections() {
    use planar_geometry::path::{close_path_at_tangents, join_paths_at_tangents};
    let head = BezierPath::from_polyline(&[[0., 0.], [2., 0.]], false).unwrap();
    let tail = BezierPath::from_polyline(&[[4., 2.], [4., 4.]], false).unwrap();
    let joined = join_paths_at_tangents(&head, false, &tail, false, 1e-6).unwrap();
    assert_eq!(joined.segments.len(), 4);
    assert!(matches!(
        joined.segments[1],
        PathSegment::Cubic { to: [4., 0.], .. }
    ));
    assert!(matches!(
        joined.segments[2],
        PathSegment::Cubic { to: [4., 2.], .. }
    ));
    let behind = BezierPath::from_polyline(&[[1., 2.], [1., 4.]], false).unwrap();
    let joined = join_paths_at_tangents(&head, false, &behind, false, 1e-6).unwrap();
    assert_eq!(joined.segments.len(), 3);
    assert!(matches!(joined.segments[1], PathSegment::Line { .. }));
    let open = BezierPath::from_polyline(&[[0., 2.], [0., 4.], [2., 4.], [2., 0.]], false).unwrap();
    let closed = close_path_at_tangents(&open, 1e-6).unwrap();
    assert!(closed.closed);
    assert_eq!(closed.start, closed.segments.last().unwrap().end());
}

#[test]
fn angular_snap_projects_only_inside_tolerance() {
    let (point, angle) = edit::snap_to_rays([0., 0.], [10., 0.1], 0., 8, 3.)
        .unwrap()
        .unwrap();
    assert_eq!(point, [10., 0.]);
    assert_eq!(angle, 0.);
    assert!(
        edit::snap_to_rays([0., 0.], [10., 2.], 0., 8, 3.)
            .unwrap()
            .is_none()
    );
    assert!(
        edit::snap_to_rays([0., 0.], [1., 0.], 0., 8, 3.)
            .unwrap()
            .is_none()
    );
    let (point, angle) =
        edit::snap_to_rays([0., 0.], [9., 10.], std::f64::consts::FRAC_PI_4, 4, 4.)
            .unwrap()
            .unwrap();
    assert!((point[0] - 9.5).abs() < 1e-12 && (point[1] - 9.5).abs() < 1e-12);
    assert_eq!(angle, std::f64::consts::FRAC_PI_4);
}

#[test]
fn alignment_guides_only_describe_the_returned_translation() {
    let moving = edit::BBox {
        min: [0., 0.],
        max: [10., 10.],
    };
    let targets = [
        edit::BBox {
            min: [1., 20.],
            max: [11., 30.],
        },
        edit::BBox {
            min: [-1., 20.],
            max: [9., 30.],
        },
    ];
    let alignment = edit::drag_align_guides(moving, &targets, 1.1).unwrap();
    assert!(!alignment.guides.is_empty());
    for guide in alignment.guides {
        let axis = usize::from(!guide.vertical);
        let actual = [moving.min[axis], moving.center()[axis], moving.max[axis]];
        assert!(
            actual
                .iter()
                .any(|p| (p + alignment.delta[axis] - guide.position).abs() < 1e-12)
        );
    }
}

#[test]
fn semantic_drag_snap_ties_and_distance_marks_match_visible_neighbors() {
    let moving = edit::BBox {
        min: [0., 0.],
        max: [10., 10.],
    };
    let first = edit::BBox {
        min: [1., 20.],
        max: [11., 30.],
    };
    let second = edit::BBox {
        min: [-1., 20.],
        max: [9., 30.],
    };
    let snapped = edit::find_drag_snap(moving, &[first, second], 1.1).unwrap();
    assert_eq!(snapped.delta, [1., 0.]);
    assert_eq!(snapped.guides.len(), 1);
    assert_eq!(snapped.guides[0].target, first);
    let marks = edit::compute_distance_marks(
        moving,
        &[
            edit::BBox {
                min: [-10., 2.],
                max: [-2., 8.],
            },
            edit::BBox {
                min: [-3., -4.],
                max: [15., 16.],
            },
            edit::BBox {
                min: [-100., -100.],
                max: [100., 100.],
            },
        ],
    );
    assert_eq!(
        marks.iter().map(|m| m.distance).collect::<Vec<_>>(),
        vec![2., 3., 5., 4., 6.]
    );
    assert_eq!(marks[0].from, [-2., 5.]);
    assert_eq!(marks[0].to, [0., 5.]);
}

#[test]
fn crossing_directions_keep_crossing_points_but_merge_opposite_angles() {
    let a = BezierPath::from_polyline(&[[0., -2.], [0., 2.]], false).unwrap();
    let b = BezierPath::from_polyline(&[[2., 2.], [2., -2.]], false).unwrap();
    let paths = [a, b];
    let crossings = edit::intersecting_paths(&paths, [-1., 0.], [3., 0.]).unwrap();
    assert_eq!(crossings.len(), 2);
    assert_eq!(crossings[0].1, [0., 0.]);
    assert_eq!(crossings[1].1, [2., 0.]);
    assert_eq!(
        edit::intersecting_path_directions(&paths, [-1., 0.], [3., 0.]).unwrap(),
        vec![std::f64::consts::FRAC_PI_2]
    );
}

#[test]
fn degenerate_ellipse_axis_still_builds_finite_polar_grid() {
    let paths =
        planar_geometry::effects::polar_grid([2., 0.], [2., 10.], 2, 4, 0.2, false).unwrap();
    assert_eq!(paths.len(), 6);
    for path in paths {
        assert!(
            path.flatten()
                .unwrap()
                .iter()
                .all(|p| p[0] == 2. && p[1].is_finite())
        );
    }
}

#[test]
fn closed_simplify_matches_farthest_pair_reference_on_irregular_rings() {
    // Independent quadratic reference matches Curvex; the production diameter
    // search uses convex hull/calipers so dense curves avoid quadratic work.
    fn reduce(points: &[[f64; 2]], eps: f64) -> Vec<[f64; 2]> {
        if points.len() < 3 {
            return points.to_vec();
        }
        let a = points[0];
        let b = *points.last().unwrap();
        let d = [b[0] - a[0], b[1] - a[1]];
        let len = d[0].hypot(d[1]);
        let (index, maximum) = (1..points.len() - 1)
            .map(|i| {
                let p = [points[i][0] - a[0], points[i][1] - a[1]];
                let distance = if len < 1e-15 {
                    p[0].hypot(p[1])
                } else {
                    (d[0] * p[1] - d[1] * p[0]).abs() / len
                };
                (i, distance)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .unwrap();
        if maximum <= eps {
            return vec![a, b];
        }
        let mut output = reduce(&points[..=index], eps);
        output.pop();
        output.extend(reduce(&points[index..], eps));
        output
    }
    let mut seed = 21u64;
    for n in 5..90 {
        let points: Vec<_> = (0..n)
            .map(|i| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let radius = 3.0 + (seed >> 32) as f64 / u32::MAX as f64 * 7.0;
                let angle = i as f64 / n as f64 * std::f64::consts::TAU;
                [radius * angle.cos(), radius * angle.sin()]
            })
            .collect();
        let mut pair = (0, 1);
        let mut maximum = -1.;
        for i in 0..n {
            for j in i + 1..n {
                let d = [points[j][0] - points[i][0], points[j][1] - points[i][1]];
                let distance = d[0] * d[0] + d[1] * d[1];
                if distance > maximum {
                    maximum = distance;
                    pair = (i, j);
                }
            }
        }
        let mut reference = reduce(&points[pair.0..=pair.1], 0.4);
        let mut second = points[pair.1..].to_vec();
        second.extend_from_slice(&points[..=pair.0]);
        let second = reduce(&second, 0.4);
        reference.extend_from_slice(&second[1..second.len() - 1]);
        let path = BezierPath::from_polyline(&points, true)
            .unwrap()
            .simplify(0.4)
            .unwrap();
        assert_eq!(
            path.to_ring(0.25).unwrap(),
            reference,
            "ring with {n} points"
        );
    }
}
