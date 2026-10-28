use super::*;

fn near(a: [f64; 2], b: [f64; 2]) {
    assert!(norm2(sub2(a, b)) < 1e-9, "{a:?} != {b:?}");
}

#[test]
fn zig_zag_creates_ridges_on_a_straight_line_and_pins_endpoints() {
    let path = BezierPath::from_polyline(&[[0., 0.], [8., 0.]], false).unwrap();
    let options = ZigZagOptions {
        amplitude: 1.,
        ridges: 3,
        ..Default::default()
    };
    let sharp = zig_zag_with_options(&path, &options).unwrap();
    assert_eq!(
        sharp.anchors(),
        vec![[0., 0.], [2., 1.], [4., -1.], [6., 1.], [8., 0.]]
    );
    let smooth = zig_zag_with_options(
        &path,
        &ZigZagOptions {
            smooth: true,
            ..options
        },
    )
    .unwrap();
    assert_eq!(smooth.anchors(), sharp.anchors());
    assert!(
        smooth
            .segments
            .iter()
            .all(|s| matches!(s, PathSegment::Cubic { .. }))
    );
    for pair in smooth.segments.windows(2) {
        if let [
            PathSegment::Cubic { c2, to, .. },
            PathSegment::Cubic { c1, .. },
        ] = pair
        {
            near(sub2(*to, *c2), sub2(*c1, *to));
        }
    }
    assert!(zig_zag(&path, 1., 2.).unwrap().segments.len() > 2);
}

#[test]
fn zig_zag_closed_seam_has_the_same_treatment_as_other_corners() {
    let path = BezierPath::from_rect([0., 0.], [8., 8.]).unwrap();
    let result = zig_zag_with_options(
        &path,
        &ZigZagOptions {
            amplitude: 1.,
            ridges: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.closed);
    assert_eq!(result.anchor_count(), 8);
    near(
        result.start,
        [
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        ],
    );
    assert_eq!(result.segments.last().unwrap().end(), result.start);
    assert!(result.anchors().iter().flatten().all(|v| v.is_finite()));
}

#[test]
fn pucker_bloat_bows_rectangle_sides_and_uses_anchor_centroid() {
    let rect = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
    let bloat = pucker_bloat(&rect, 0.5).unwrap();
    assert_eq!(bloat.start, [-1., -0.5]);
    assert_eq!(
        bloat.segments[0],
        PathSegment::Cubic {
            c1: [-1.75, -0.875],
            c2: [5.75, -0.875],
            to: [5., -0.5]
        }
    );
    let (_, middle) = split_cubic(to_cubics(&bloat)[0], 0.5);
    near(middle.0, [2., -0.78125]);
    let asym = BezierPath::from_polyline(&[[0., 0.], [6., 0.], [0., 3.]], true).unwrap();
    let result = pucker_bloat(&asym, 0.5).unwrap();
    near(result.start, [-1., -0.5]); // centroid (2,1), not bounds center (3,1.5)
    let pucker = pucker_bloat(&rect, -0.5).unwrap();
    assert_eq!(pucker.start, [1., 0.5]);
    assert!(matches!(pucker.segments[0], PathSegment::Cubic { c1, .. } if c1[1] > 0.5));
    assert_eq!(pucker_bloat(&rect, 0.).unwrap(), rect);
}

#[test]
fn roughen_densifies_pins_open_ends_and_can_smooth() {
    let line = BezierPath::from_polyline(&[[0., 0.], [8., 0.]], false).unwrap();
    let options = RoughenOptions {
        amplitude: 0.5,
        detail: 4,
        smooth: false,
        seed: 0,
    };
    let result = roughen_with_options(&line, &options).unwrap();
    assert_eq!(result.anchor_count(), 5);
    assert_eq!(result.start, line.start);
    assert_eq!(result.segments.last().unwrap().end(), [8., 0.]);
    for (i, p) in result.anchors().iter().enumerate().skip(1).take(3) {
        assert!((p[0] - 2. * i as f64).abs() <= 0.5 && p[1].abs() <= 0.5);
        assert_ne!(p[1], 0.);
    }
    assert_eq!(result, roughen_with_options(&line, &options).unwrap());
    assert_ne!(
        result,
        roughen_with_options(
            &line,
            &RoughenOptions {
                seed: 42,
                ..options
            }
        )
        .unwrap()
    );
    let smooth = roughen_with_options(
        &line,
        &RoughenOptions {
            smooth: true,
            ..options
        },
    )
    .unwrap();
    assert_eq!(result.anchors(), smooth.anchors());
    assert!(
        smooth
            .segments
            .iter()
            .all(|s| matches!(s, PathSegment::Cubic { .. }))
    );
}

#[test]
fn roughen_closed_contour_has_one_consistent_seam() {
    let rect = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
    let result = roughen_with_options(
        &rect,
        &RoughenOptions {
            detail: 3,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.anchor_count(), 12);
    assert_ne!(result.start, rect.start);
    assert_eq!(result.segments.last().unwrap().end(), result.start);
}

#[test]
fn twist_bends_straight_edges_and_rotates_the_outer_radius_by_full_angle() {
    let rect = BezierPath::from_rect([-2., -1.], [2., 1.]).unwrap();
    let result = twist(&rect, PI * 0.5).unwrap();
    assert_eq!(result.anchor_count(), 4 * 24);
    near(result.start, [1., -2.]);
    assert_eq!(result.segments.last().unwrap().end(), result.start);
    // Midpoint radius is smaller than a corner radius and rotates less.
    let mid = result.anchors()[12];
    assert!(mid[0] > 0. && mid[0] < 1. && mid[1] < 0.);
    assert_eq!(twist(&rect, 0.).unwrap(), rect);
}

#[test]
fn scatter_preserves_group_geometry_with_scale_rotation_and_seed() {
    let a = BezierPath::from_rect([0., 0.], [2., 1.]).unwrap();
    let b = translate_path(&a, [5., 0.]);
    let source = [a, b];
    let options = ScatterOptions {
        count: 3,
        position: 3.,
        rotation_radians: PI,
        scale_min: 2.,
        scale_max: 2.,
        seed: 13,
    };
    let result = scatter_paths(&source, &options).unwrap();
    assert_eq!(result, scatter_paths(&source, &options).unwrap());
    assert_eq!(result.len(), 6);
    assert_ne!(
        result,
        scatter_paths(
            &source,
            &ScatterOptions {
                seed: 14,
                ..options
            }
        )
        .unwrap()
    );
    for pair in result.as_chunks::<2>().0 {
        let offset = sub2(pair[1].start, pair[0].start);
        assert!((norm2(offset) - 10.).abs() < 1e-9);
        assert!(offset[1].abs() > 0.1); // rotation really happened
        for (p, q) in pair[0].anchors().into_iter().zip(pair[1].anchors()) {
            near(sub2(q, p), offset);
        }
        let edge = sub2(pair[0].segments[0].end(), pair[0].start);
        assert!((norm2(edge) - 4.).abs() < 1e-9);
    }
}

#[test]
fn blend_aligns_both_seam_and_winding_before_interpolation() {
    let a = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
    let b =
        BezierPath::from_polyline(&[[14., 2.], [14., 0.], [10., 0.], [10., 2.]], true).unwrap();
    let result = blend(&a, &b, 1).unwrap().remove(0);
    assert_eq!(
        result.anchors(),
        vec![[5., 0.], [9., 0.], [9., 2.], [5., 2.]]
    );
    for c in to_cubics(&result) {
        near(c.1, lerp(c.0, c.3, 1. / 3.));
        near(c.2, lerp(c.0, c.3, 2. / 3.));
    }
    assert_eq!(result.segments.last().unwrap().end(), result.start);
}

#[test]
fn partial_and_negative_arc_sweeps_follow_the_ellipse() {
    for sweep in [-135., -45., 45., 135.] {
        let path = arc_path([-2., -1.], [2., 1.], 0., sweep, ArcMode::Arc).unwrap();
        let cubics = to_cubics(&path);
        for c in cubics {
            for k in 0..=10 {
                let (_, right) = split_cubic(c, k as f64 / 10.);
                let p = right.0;
                let ellipse = (p[0] * 0.5).powi(2) + p[1].powi(2);
                assert!((ellipse - 1.).abs() < 0.0006, "sweep {sweep}: {ellipse}");
                assert!(p[1] * sweep >= -1e-12);
            }
        }
    }
}

#[test]
fn grouped_repeats_omit_sources_and_support_upright_orbits() {
    let path = BezierPath::from_rect([2., -0.5], [4., 0.5]).unwrap();
    let source = [path.clone()];
    let linear = step_and_repeat_paths(&source, 2, [10., 0.]).unwrap();
    assert_eq!(linear[0].start, [12., -0.5]);
    assert_eq!(linear[1].start, [22., -0.5]);
    let grid = grid_array_paths(&source, 2, 2, [10., 20.]).unwrap();
    assert_eq!(grid.len(), 3);
    assert_eq!(
        grid.iter().map(|p| p.start).collect::<Vec<_>>(),
        vec![[12., -0.5], [2., 19.5], [12., 19.5]]
    );
    assert!(
        grid_array_paths(&source, 1, 1, [0., 0.])
            .unwrap()
            .is_empty()
    );
    let upright = radial_repeat_paths(&source, 1, [0., 0.], PI * 0.5, false).unwrap();
    near(upright[0].start, [-1., 2.5]);
    near(
        sub2(upright[0].segments[0].end(), upright[0].start),
        [2., 0.],
    );
    let rotated = radial_repeat_paths(&source, 1, [0., 0.], PI * 0.5, true).unwrap();
    near(
        sub2(rotated[0].segments[0].end(), rotated[0].start),
        [0., 2.],
    );
}

#[test]
fn effects_reject_unbounded_and_non_finite_inputs() {
    let line = BezierPath::from_polyline(&[[0., 0.], [100., 0.]], false).unwrap();
    assert!(zig_zag(&line, 1., 0.000002).is_err());
    assert!(zig_zag(&line, 1., f64::INFINITY).is_err());
    assert!(
        roughen_with_options(
            &line,
            &RoughenOptions {
                detail: usize::MAX,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        scatter_paths(
            std::slice::from_ref(&line),
            &ScatterOptions {
                count: usize::MAX,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut invalid = line;
    invalid.start[0] = f64::NAN;
    assert!(pucker_bloat(&invalid, 0.5).is_err());
    assert!(blend(&invalid, &invalid, 1).is_err());
}

#[test]
fn stipple_bounds_candidate_work_even_when_no_mark_would_survive() {
    let empty_region = vec![
        vec![
            [0., 0.],
            [1000000., 0.],
            [1000000., 1000000.],
            [0., 1000000.]
        ];
        2
    ];
    assert!(
        stipple_rings(
            &empty_region,
            &StippleOptions {
                spacing: 0.01,
                fill_rule: crate::tessellation::FillRule::EvenOdd,
                ..Default::default()
            }
        )
        .is_err()
    );
    let ring = stipple_mark([2., 3.], 4., StippleKind::Ring).unwrap();
    let dot = stipple_mark([2., 3.], 4., StippleKind::Dot).unwrap();
    assert_eq!(ring, dot); // appearance, not a different polygon, makes a ring
    assert_eq!(ring.segments.len(), 4);
}

#[test]
fn free_distort_uses_the_editable_control_bounds_and_handles_flat_axes() {
    let path = BezierPath::open(
        [0., 0.],
        vec![PathSegment::Cubic {
            c1: [2., 5.],
            c2: [7., -3.],
            to: [10., 0.],
        }],
    )
    .unwrap();
    let identity = free_distort(&path, [[0., -3.], [10., -3.], [10., 5.], [0., 5.]]).unwrap();
    for (a, b) in to_cubics(&path).into_iter().zip(to_cubics(&identity)) {
        near(a.0, b.0);
        near(a.1, b.1);
        near(a.2, b.2);
        near(a.3, b.3);
    }
    let line = BezierPath::from_polyline(&[[0., 2.], [10., 2.]], false).unwrap();
    let warped = free_distort(&line, [[1., 4.], [20., 8.], [20., 20.], [1., 20.]]).unwrap();
    near(warped.start, [1., 4.]);
    near(warped.segments[0].end(), [20., 8.]);
}

#[test]
fn spiral_and_polar_grid() {
    let s = spiral([-10., -10.], [10., 10.], 2., 0.1, true).unwrap();
    assert!(!s.closed);
    assert!(s.segments.len() > 20);
    let g = polar_grid([-5., -5.], [5., 5.], 3, 8, 0.2, true).unwrap();
    assert!(g.len() >= 3 + 4); // rings + half spokes
}

#[test]
fn arrays_and_hatch() {
    let p = BezierPath::from_rect([0., 0.], [1., 1.]).unwrap();
    assert_eq!(step_and_repeat(&p, 3, [2., 0.]).unwrap().len(), 3);
    assert_eq!(radial_repeat(&p, 4, [0., 0.]).unwrap().len(), 4);
    assert_eq!(grid_array(&p, 2, 3, [2., 2.]).unwrap().len(), 6);
    let ring = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
    let h = hatch(&ring, 2., 0., false).unwrap();
    assert!(!h.is_empty());
    let st = stipple(&ring, 3., 0.5, StippleKind::Dot).unwrap();
    assert!(!st.is_empty());
}

#[test]
fn hatch_skips_donut_hole() {
    use crate::tessellation::FillRule;
    let outer = vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]];
    let hole = vec![[6., 6.], [14., 6.], [14., 14.], [6., 14.]];
    // Opposite winding so nonzero treats hole as empty.
    let hole_cw: Vec<[f64; 2]> = hole.into_iter().rev().collect();
    let rings = vec![outer, hole_cw];
    let lines = hatch_rings(&rings, FillRule::NonZero, 2., 0., false).unwrap();
    assert!(!lines.is_empty());
    for line in &lines {
        let pts = line.flatten().unwrap();
        for p in pts {
            // No hatch geometry should sit deep inside the hole.
            assert!(
                !(p[0] > 7. && p[0] < 13. && p[1] > 7. && p[1] < 13.),
                "hatch point in hole {p:?}"
            );
        }
    }
}

#[test]
fn stipple_jitter_is_deterministic() {
    use crate::tessellation::FillRule;
    let ring = vec![vec![[0., 0.], [12., 0.], [12., 12.], [0., 12.]]];
    let opts = StippleOptions {
        spacing: 4.,
        size: 0.4,
        kind: StippleKind::Ring,
        jitter: 0.8,
        seed: 42,
        fill_rule: FillRule::NonZero,
    };
    let a = stipple_rings(&ring, &opts).unwrap();
    let b = stipple_rings(&ring, &opts).unwrap();
    assert_eq!(a.len(), b.len());
    assert!(!a.is_empty());
    assert!(a.iter().all(|m| m.closed));
    let starts: Vec<_> = a.iter().map(|m| m.start).collect();
    let starts_b: Vec<_> = b.iter().map(|m| m.start).collect();
    assert_eq!(starts, starts_b);
    let no_jitter = stipple_rings(
        &ring,
        &StippleOptions {
            jitter: 0.,
            kind: StippleKind::Dot,
            ..opts
        },
    )
    .unwrap();
    assert_ne!(
        a.iter().map(|m| m.start).collect::<Vec<_>>(),
        no_jitter.iter().map(|m| m.start).collect::<Vec<_>>()
    );
}

#[test]
fn hatch_path_and_cross() {
    use crate::tessellation::FillRule;
    let outer = BezierPath::from_rect([0., 0.], [10., 8.]).unwrap();
    let hole = BezierPath::from_rect([3., 2.], [7., 6.]).unwrap().reverse();
    let lines = hatch_path(
        &outer,
        &[hole],
        FillRule::NonZero,
        1.5,
        PI * 0.25,
        true,
        0.25,
    )
    .unwrap();
    assert!(lines.len() >= 4);
}

#[test]
fn distortions_and_blend() {
    let p = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
    assert!(zig_zag(&p, 0.2, 1.).unwrap().segments.len() >= 3);
    assert!(pucker_bloat(&p, 0.2).unwrap().closed);
    assert!(roughen(&p, 0.1, 1).unwrap().closed);
    assert!(twist(&p, 0.5).unwrap().closed);
    assert_eq!(scatter(&p, 5, 2., 7).unwrap().len(), 5);
    let q = BezierPath::from_circle([2., 1.], 1.).unwrap();
    let b = blend(&p, &q, 3).unwrap();
    assert_eq!(b.len(), 3);
    let d = free_distort(&p, [[0., 0.], [5., -1.], [6., 3.], [-1., 4.]]).unwrap();
    assert!(d.closed);
}

#[test]
fn arc_and_star() {
    let a = arc_path([-1., -1.], [1., 1.], 0., 90., ArcMode::Pie).unwrap();
    assert!(a.closed);
    let s = star([-2., -2.], [2., 2.], 5, 0.4).unwrap();
    assert_eq!(s.anchor_count(), 10);
}

#[test]
fn concentric_offset_rings() {
    let ring = vec![vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]]];
    let rings = concentric_offset(&ring, 3, 1., "Miter", 8).unwrap();
    assert_eq!(rings.len(), 3);
}

#[test]
fn concentric_offsets_preserve_evenodd_holes_and_reject_invalid_options() {
    use crate::path_offset::OffsetOptions;
    use crate::tessellation::FillRule;
    let rings = vec![
        vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]],
        vec![[6., 6.], [14., 6.], [14., 14.], [6., 14.]],
    ];
    let options = OffsetOptions {
        distance: 1.,
        fill_rule: FillRule::EvenOdd,
        ..Default::default()
    };
    let result = concentric_offset_with_options(&rings, 2, &options).unwrap();
    assert_eq!(
        result.iter().map(|r| r.len()).collect::<Vec<_>>(),
        vec![2, 2]
    );
    for (i, rings) in result.iter().enumerate() {
        let area: f64 = rings.iter().map(|r| crate::rings::area(r)).sum();
        let d = (i + 1) as f64;
        assert!((area - ((20. + 2. * d).powi(2) - (8. - 2. * d).powi(2))).abs() < 1e-7);
    }
    assert!(
        concentric_offset_with_options(
            &rings,
            2,
            &OffsetOptions {
                tolerance: 0.,
                ..options
            }
        )
        .is_err()
    );
    let small = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
    assert_eq!(
        concentric_offset_with_options(
            &small,
            4,
            &OffsetOptions {
                distance: -1.,
                ..options
            }
        )
        .unwrap()
        .len(),
        1
    );
}
