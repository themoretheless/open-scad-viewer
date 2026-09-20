#[path = "support/profile.rs"]
mod profile;
use planar_geometry::triangulation::triangulate_profile;

fn square(x: f64, y: f64, size: f64) -> Vec<[f64; 2]> {
    vec![[x, y], [x + size, y], [x + size, y + size], [x, y + size]]
}

#[test]
fn axis_aligned_square_grids_preserve_boundaries() {
    for side in 2..=10 {
        let outer = square(0., 0., side as f64 * 3.);
        let holes: Vec<_> = (0..side * side)
            .map(|i| square((i % side) as f64 * 3. + 1., (i / side) as f64 * 3. + 1., 1.))
            .collect();
        let mesh = triangulate_profile(&outer, &holes)
            .unwrap_or_else(|error| panic!("{side}x{side}: {error}"));
        profile::validate(&mesh, &outer, &holes);
    }
}

#[test]
fn rejects_bow_tie_boundary() {
    let outer = [[0., 0.], [4., 4.], [0., 4.], [4., 0.]];
    assert!(triangulate_profile(&outer, &[]).is_err());
}

#[test]
fn rejects_self_intersection_with_nonzero_signed_area() {
    let outer = [[0., 3.], [2., -3.], [-3., 1.], [3., 1.], [-2., -3.]];
    assert!(triangulate_profile(&outer, &[]).is_err());
}

#[test]
fn rejects_crossing_permutations_of_boundary_vertices() {
    let points = [
        [0., 0.],
        [2., 0.],
        [4., 0.],
        [4., 2.],
        [4., 4.],
        [2., 4.],
        [0., 4.],
        [0., 2.],
    ];
    let orient = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let mut state = 0x12345678_u32;
    let mut checked = 0;
    for _ in 0..500 {
        let mut outer = points;
        for i in (1..outer.len()).rev() {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            outer.swap(i, state as usize % (i + 1));
        }
        let crossing = (0..outer.len()).any(|i| {
            (i + 1..outer.len()).any(|j| {
                let (a, b, c, d) = (
                    outer[i],
                    outer[(i + 1) % outer.len()],
                    outer[j],
                    outer[(j + 1) % outer.len()],
                );
                orient(a, b, c) * orient(a, b, d) < 0. && orient(c, d, a) * orient(c, d, b) < 0.
            })
        });
        if crossing {
            checked += 1;
            assert!(
                triangulate_profile(&outer, &[]).is_err(),
                "Accepted crossing profile: {outer:?}"
            );
        }
    }
    assert!(checked > 300, "Insufficient crossing fixtures: {checked}");
}

#[test]
fn oversized_square_grid_refuses_at_profile_budget() {
    let holes: Vec<_> = (0..1024)
        .map(|i| square((i % 32) as f64 * 2. + 1., (i / 32) as f64 * 2. + 1., 0.5))
        .collect();
    let error = triangulate_profile(&square(0., 0., 64.), &holes).unwrap_err();
    assert_eq!(error.message, "Profile triangulation budget exceeded");
}

#[test]
fn aligned_holes_have_conforming_boundaries() {
    for side in [1, 2, 4, 7, 10] {
        for segments in [4, 12, 32] {
            let (outer, holes) = profile::grid(side, segments);
            let mesh = triangulate_profile(&outer, &holes)
                .unwrap_or_else(|error| panic!("{side}x{side}/{segments}: {error}"));
            profile::validate(&mesh, &outer, &holes);
        }
    }
}

#[test]
fn preserves_collinear_boundary_segments_and_ring_orientations() {
    let outer = vec![
        [0., 0.],
        [5., 0.],
        [10., 0.],
        [10., 5.],
        [10., 10.],
        [5., 10.],
        [0., 10.],
        [0., 5.],
    ];
    let hole = vec![
        [2., 2.],
        [3., 2.],
        [4., 2.],
        [4., 3.],
        [4., 4.],
        [3., 4.],
        [2., 4.],
        [2., 3.],
    ];
    for reverse_outer in [false, true] {
        for reverse_hole in [false, true] {
            let mut outer = outer.clone();
            let mut hole = hole.clone();
            if reverse_outer {
                outer.reverse();
            }
            if reverse_hole {
                hole.reverse();
            }
            let holes = vec![hole];
            profile::validate(
                &triangulate_profile(&outer, &holes).unwrap(),
                &outer,
                &holes,
            );
        }
    }
}

#[test]
fn holes_remain_conforming_after_reflection_scaling_and_reordering() {
    let (outer, holes) = profile::grid(4, 32);
    for scale in [0.001, 1., 1000.] {
        for sign in [-1., 1.] {
            for reverse in [false, true] {
                let transform = |p: &[f64; 2]| [scale * sign * p[1] + 17., scale * p[0] - 23.];
                let mut outer: Vec<_> = outer.iter().map(transform).collect();
                let mut holes: Vec<Vec<_>> = holes
                    .iter()
                    .map(|h| h.iter().map(transform).collect())
                    .collect();
                if reverse {
                    outer.reverse();
                    holes.reverse();
                    for hole in &mut holes {
                        hole.reverse();
                        hole.rotate_left(7);
                    }
                }
                profile::validate(
                    &triangulate_profile(&outer, &holes).unwrap(),
                    &outer,
                    &holes,
                );
            }
        }
    }
}

#[test]
fn rejects_nonfinite_and_excessive_inputs() {
    let (outer, holes) = profile::grid(12, 32);
    assert!(
        triangulate_profile(&outer, &holes)
            .unwrap_err()
            .contains("budget")
    );
    assert!(triangulate_profile(&[[0., 0.], [1., 0.], [0., f64::NAN]], &[]).is_err());
    assert!(triangulate_profile(&outer, &[vec![[0., 0.], [1., 0.]]]).is_err());
    assert!(triangulate_profile(&[[0., 0.], [1., 0.], [2., 0.]], &[]).is_err());
}
