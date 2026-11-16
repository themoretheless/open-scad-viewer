//! Property tests for the monotone sweep triangulation: area conservation,
//! strictly positive triangles and no overlaps (checked by `validate` through
//! directed-edge pairing) on random and adversarial polygons with holes.
#[path = "support/profile.rs"]
mod profile;
use planar_geometry::triangulation::{triangulate_profile, triangulate_profile_ear_clipping};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn star(rng: &mut Rng, center: [f64; 2], r_min: f64, r_max: f64, n: usize) -> Vec<[f64; 2]> {
    (0..n)
        .map(|i| {
            let angle = std::f64::consts::TAU * (i as f64 + 0.8 * rng.next()) / n as f64;
            let r = r_min + (r_max - r_min) * rng.next();
            [center[0] + r * angle.cos(), center[1] + r * angle.sin()]
        })
        .collect()
}

fn check(outer: &[[f64; 2]], holes: &[Vec<[f64; 2]>]) {
    let mesh = triangulate_profile(outer, holes)
        .unwrap_or_else(|e| panic!("{e}: outer {outer:?} holes {holes:?}"));
    profile::validate(&mesh, outer, holes);
    let again = triangulate_profile(outer, holes).unwrap();
    assert_eq!(
        mesh.indices, again.indices,
        "nondeterministic triangulation"
    );
}

#[test]
fn random_star_polygons_with_holes() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for case in 0..400 {
        let n = 12 + case % 60;
        let outer = star(&mut rng, [0., 0.], 40., 100., n);
        let count = case % 9;
        let holes: Vec<_> = (0..count)
            .map(|k| {
                let a = std::f64::consts::TAU * k as f64 / count as f64;
                let c = [20. * a.cos(), 20. * a.sin()];
                star(&mut rng, c, 1., 6., 3 + (case + k) % 20)
            })
            .collect();
        check(&outer, &holes);
    }
}

#[test]
fn random_quantized_polygons_with_many_collinear_vertices() {
    // Coordinates snapped to a coarse grid create horizontal and vertical
    // runs, equal-y split/merge vertices and exactly collinear triples.
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let snap = |p: [f64; 2]| p.map(|v| (v / 4.).round() * 4.);
    let mut checked = 0;
    for case in 0..400 {
        let mut outer: Vec<_> = star(&mut rng, [0., 0.], 30., 100., 4 + case % 40)
            .into_iter()
            .map(snap)
            .collect();
        outer.dedup();
        let hole: Vec<_> = (0..4)
            .map(|k| [[-4., -4.], [4., -4.], [4., 4.], [-4., 4.]][k])
            .collect();
        let simple = triangulate_profile_ear_clipping(&outer, &[]).is_ok();
        if simple {
            check(&outer, &[hole]);
            checked += 1;
        }
    }
    assert!(checked > 200, "too few simple fixtures: {checked}");
}

#[test]
fn combs_spirals_and_staircases() {
    // Comb: many split vertices on one horizontal line.
    let mut comb = vec![[0., 0.], [41., 0.], [41., 10.]];
    for k in (0..20).rev() {
        let x = 2. * k as f64 + 1.;
        comb.extend([[x + 1., 10.], [x + 1., 1.], [x, 1.], [x, 10.]]);
    }
    comb.push([0., 10.]);
    comb.dedup();
    check(&comb, &[]);
    let flipped: Vec<_> = comb.iter().map(|p| [p[0], -p[1]]).collect();
    check(&flipped, &[]);
    let rotated: Vec<_> = comb.iter().map(|p| [p[1], p[0]]).collect();
    check(&rotated, &[]);
    // Staircase on an integer grid.
    let mut stairs = vec![[0., 0.]];
    for k in 0..30 {
        stairs.push([k as f64 + 1., k as f64]);
        stairs.push([k as f64 + 1., k as f64 + 1.]);
    }
    stairs.push([0., 30.]);
    check(&stairs, &[vec![[1., 5.], [1., 6.], [2., 6.], [2., 5.]]]);
    // Spiral corridor.
    let mut inner = vec![];
    let mut outer_arm = vec![];
    for k in 0..200 {
        let a = k as f64 * 0.1;
        let r = 1. + a;
        inner.push([r * a.cos(), r * a.sin()]);
        outer_arm.push([(r + 0.5) * a.cos(), (r + 0.5) * a.sin()]);
    }
    outer_arm.reverse();
    let spiral: Vec<_> = inner.into_iter().chain(outer_arm).collect();
    check(&spiral, &[]);
}

#[test]
fn duplicate_vertices_and_spikes_are_dropped() {
    let outer = vec![
        [0., 0.],
        [0., 0.],
        [10., 0.],
        [10., 10.],
        [12., 10.],
        [10., 10.],
        [0., 10.],
        [0., 0.],
    ];
    let mesh = triangulate_profile(&outer, &[]).unwrap();
    assert_eq!(mesh.positions.len(), 4);
    profile::validate(&mesh, &[[0., 0.], [10., 0.], [10., 10.], [0., 10.]], &[]);
}

#[test]
fn touching_holes_share_vertices_without_overlap() {
    let outer = vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
    let a = vec![[2., 5.], [3.5, 3.5], [5., 5.], [3.5, 6.5]];
    let b = vec![[5., 5.], [6.5, 3.5], [8., 5.], [6.5, 6.5]];
    check(&outer, &[a.clone(), b.clone()]);
    // A hole touching the outer ring at a vertex of both.
    let corner = vec![[0., 0.], [3., 1.], [1., 3.]];
    check(&outer, std::slice::from_ref(&corner));
    check(&outer, &[a, b, corner]);
    // Axis-aligned squares touching at one corner (equal-y ties).
    let s1 = vec![[2., 2.], [5., 2.], [5., 5.], [2., 5.]];
    let s2 = vec![[5., 5.], [8., 5.], [8., 8.], [5., 8.]];
    check(&outer, &[s1, s2]);
    // Checkerboard: four holes meeting pairwise at shared corners.
    let sq = |x: f64, y: f64| vec![[x, y], [x + 2., y], [x + 2., y + 2.], [x, y + 2.]];
    check(
        &outer,
        &[sq(2., 2.), sq(4., 4.), sq(6., 2.), sq(2., 6.), sq(6., 6.)],
    );
}

#[test]
fn near_degenerate_inputs_are_decided_exactly() {
    for scale in [1e-6, 1., 1e6] {
        let mut outer = vec![];
        for k in 0..64 {
            let t = k as f64 / 64.;
            outer.push([scale * t, scale * 1e-15 * ((k * 7919) % 13) as f64]);
        }
        outer.push([scale, scale]);
        outer.push([0., scale]);
        check(&outer, &[]);
    }
    let outer = vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let e = f64::EPSILON;
    check(&outer, &[vec![[0.25, e], [0.75, e], [0.5, 0.5]]]);
}

#[test]
fn invalid_inputs_are_refused() {
    let outer = vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
    let outside = vec![[20., 20.], [21., 20.], [21., 21.]];
    assert!(triangulate_profile(&outer, &[outside]).is_err());
    let big = vec![[2., 2.], [8., 2.], [8., 8.], [2., 8.]];
    let nested = vec![[4., 4.], [6., 4.], [6., 6.], [4., 6.]];
    assert!(triangulate_profile(&outer, &[big.clone(), nested]).is_err());
    let t_junction = vec![[5., 0.], [6., 1.], [4., 1.]];
    assert!(triangulate_profile(&outer, &[t_junction]).is_err());
    let shifted = vec![[5., 5.], [9., 5.], [9., 9.], [5., 9.]];
    assert!(triangulate_profile(&outer, &[big.clone(), shifted]).is_err());
    // Two holes sharing a vertex with interleaved sectors cross there.
    let x1 = vec![[5., 5.], [7., 3.], [7., 7.]];
    let x2 = vec![[5., 5.], [6., 2.], [8., 4.]];
    assert!(triangulate_profile(&outer, &[x1, x2]).is_err());
}

#[test]
fn agrees_with_ear_clipping_triangle_counts() {
    for side in [1, 3, 5] {
        let (outer, holes) = profile::grid(side, 24);
        let old = triangulate_profile_ear_clipping(&outer, &holes).unwrap();
        let new = triangulate_profile(&outer, &holes).unwrap();
        assert_eq!(old.indices.len(), new.indices.len());
    }
}
