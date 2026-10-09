use super::*;

fn clamped_knots(n: usize, p: usize) -> Vec<f64> {
    let spans = n - p;
    let mut knots = vec![0.; p + 1];
    for s in 1..spans {
        knots.push(s as f64 / spans as f64);
    }
    knots.extend(vec![1.; p + 1]);
    knots
}

/// Bilinear plane patch on [0,1]² at height z, 5x5 cubic net.
fn plane_surface(z: f64) -> Surface {
    let (n, p) = (5usize, 3usize);
    let knots = clamped_knots(n, p);
    let g: Vec<f64> = (0..n).map(|i| greville(&knots, p, i)).collect();
    let mut control_points = vec![vec![vec![0.; 3]; n]; n];
    for i in 0..n {
        for j in 0..n {
            control_points[i][j] = vec![g[i], g[j], z];
        }
    }
    Surface {
        degree_u: p,
        degree_v: p,
        knots_u: knots.clone(),
        knots_v: knots,
        control_points,
        weights: vec![vec![1.; n]; n],
        periodic_u: false,
        periodic_v: false,
    }
}

fn straight_wire(dx: f64) -> Wire {
    // Linear B-spline along x at y=z=0; the target is lifted by dx in z.
    let base = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let target = Curve {
        control_points: vec![vec![0., 0., dx], vec![1., 0., dx]],
        ..base.clone()
    };
    Wire {
        base,
        target,
        radius: 0.5,
    }
}

#[test]
fn identity_lattice_leaves_geometry_unchanged() {
    let lattice = FfdLattice::new([0., 0., 0.], [1., 1., 1.], [3, 3, 3], [2, 2, 2]).unwrap();
    let points = [
        [0.1, 0.2, 0.3],
        [0.5, 0.5, 0.5],
        [0.99, 0.01, 0.47],
        [0., 0., 0.],
        [1., 1., 1.],
    ];
    for p in points {
        let q = lattice.deform_point(p).unwrap();
        for axis in 0..3 {
            assert!(
                (q[axis] - p[axis]).abs() < 1e-12,
                "identity lattice moved {p:?} to {q:?}"
            );
        }
    }
    // Same for a curve and a surface through the lattice.
    let surface = plane_surface(0.4);
    let deformed = lattice.apply_to_surface(&surface).unwrap();
    for i in 0..5 {
        for j in 0..5 {
            for axis in 0..3 {
                assert!(
                    (deformed.control_points[i][j][axis]
                        - surface.control_points[i][j][axis])
                        .abs()
                        < 1e-12
                );
            }
        }
    }
}

#[test]
fn ffd_locality_single_control_moves_only_its_support() {
    let mut lattice = FfdLattice::new([0., 0., 0.], [1., 1., 1.], [2, 2, 2], [4, 4, 4]).unwrap();
    // Control (2,2,2) of degree-2 axes with 4 spans: support on each
    // axis is knots[2..5] = [0, 0.75]; move it up.
    let slot = lattice.control_index(2, 2, 2).unwrap();
    let mut position = lattice.control(slot).unwrap();
    position[2] += 0.3;
    lattice.set_control(slot, position).unwrap();
    // Point deep inside the support moves.
    let inside = lattice.deform_point([0.4, 0.4, 0.4]).unwrap();
    assert!((inside[2] - 0.4).abs() > 1e-6);
    // Point at (0.9, 0.9, 0.9): basis of control index 2 on axis with
    // knots {0,0,0,.25,.5,.75,1,1,1} has support [k2, k5) = [0, .75);
    // a parameter of 0.9 is outside on every axis.
    let outside = lattice.deform_point([0.9, 0.9, 0.9]).unwrap();
    for axis in 0..3 {
        assert!(
            (outside[axis] - 0.9).abs() < 1e-12,
            "point outside the support moved: {outside:?}"
        );
    }
}

#[test]
fn twist_rotates_by_the_requested_angle() {
    let mut surface = plane_surface(0.5);
    // Column of control points along z at x=0.5, y=0.
    for (i, row) in surface.control_points.iter_mut().enumerate() {
        for (j, point) in row.iter_mut().enumerate() {
            *point = vec![0.5, 0., (i * 5 + j) as f64 / 24.];
        }
    }
    let angle = std::f64::consts::FRAC_PI_2;
    let twisted = barr_twist(&surface, 2, [0.5, 0., 0.], [0., 1.], angle).unwrap();
    // A point at z = 1 rotated by π/2 about the z-axis through
    // (0.5, 0): offset (0, 0) stays; use a point with y-offset instead.
    let mut surface2 = plane_surface(0.5);
    for (i, row) in surface2.control_points.iter_mut().enumerate() {
        for (j, point) in row.iter_mut().enumerate() {
            *point = vec![0.7, 0., (i * 5 + j) as f64 / 24.];
        }
    }
    let twisted2 = barr_twist(&surface2, 2, [0.5, 0., 0.], [0., 1.], angle).unwrap();
    // z = 0: no rotation.
    assert!((twisted2.control_points[0][0][0] - 0.7).abs() < 1e-12);
    assert!(twisted2.control_points[0][0][1].abs() < 1e-12);
    // z = 1 (last row): offset (0.2, 0) rotates to (0, 0.2).
    let top = &twisted2.control_points[4][4];
    assert!((top[0] - 0.5).abs() < 1e-12, "x={}", top[0]);
    assert!((top[1] - 0.2).abs() < 1e-12, "y={}", top[1]);
    // Untouched original column sanity: x stays 0.5 everywhere.
    for row in &twisted.control_points {
        for point in row {
            assert!((point[0] - 0.5).abs() < 1e-12);
        }
    }
}

#[test]
fn taper_scales_and_corrects_normals() {
    let surface = plane_surface(0.5);
    let (tapered, normals) =
        barr_taper(&surface, 2, [0., 0., 0.], [0., 1.], 2.).unwrap();
    // z = 0.5 (mid band): r = 1.5; x = 1 scales to 1.5.
    assert!((tapered.control_points[4][0][0] - 1.5).abs() < 1e-12);
    // Plane normals (0,0,±1) are eigen-directions of the taper
    // Jacobian transpose: they stay axis-aligned after correction.
    for i in 0..5 {
        for j in 0..5 {
            let n = normals[i][j].unwrap();
            assert!((n[0]).abs() < 1e-9 && (n[1]).abs() < 1e-9);
            assert!((n[2].abs() - 1.).abs() < 1e-9);
        }
    }
    // Transposed-inverse correctness on a tilted normal: build a tilted
    // plane z = x and verify n'·(J t) ≈ 0 for tangent t = (1,0,1)/√2.
    let mut tilted = plane_surface(0.);
    for i in 0..5 {
        for j in 0..5 {
            tilted.control_points[i][j][2] = tilted.control_points[i][j][0];
        }
    }
    let (_, tilted_normals) = barr_taper(&tilted, 0, [0., 0., 0.], [0., 1.], 2.).unwrap();
    // Control (2, 0) sits at x = z = 0.5, strictly inside the band:
    // r = 1.5, dr/dx = 1, so J = [[1,0,0],[0,1.5,0],[0.5,0,1.5]] and
    // the tangent t = (1,0,1) maps to J t = (1, 0, 2).
    let n = tilted_normals[2][0].unwrap();
    let jt = [1., 0., 2.];
    let dot: f64 = (0..3).map(|a| n[a] * jt[a]).sum();
    assert!(dot.abs() < 1e-9, "corrected normal not orthogonal: {dot}");
}

#[test]
fn bend_is_rigid_beyond_the_band() {
    // Grid of control points spread in x (bend_dir) and z (axis).
    let mut surface = plane_surface(0.);
    for (i, row) in surface.control_points.iter_mut().enumerate() {
        for (j, point) in row.iter_mut().enumerate() {
            *point = vec![0.1 * i as f64, 0., 0.25 * j as f64];
        }
    }
    let rate = 1.0;
    let bent = barr_bend(&surface, 2, 0, [0., 0., 0.], [0., 0.5], rate).unwrap();
    // Band origin stays fixed: point (x=0, z=0) → centerline arc start.
    let origin = &bent.control_points[0][0];
    assert!((origin[0]).abs() < 1e-12 && (origin[2]).abs() < 1e-12);
    // Rigidity beyond the band: distances between two points both above
    // z = 0.5 with equal x are preserved (rigid rotation/translation).
    let before = {
        let a = &surface.control_points[2][3];
        let b = &surface.control_points[2][4];
        norm(sub([a[0], a[1], a[2]], [b[0], b[1], b[2]]))
    };
    let after = {
        let a = &bent.control_points[2][3];
        let b = &bent.control_points[2][4];
        norm(sub([a[0], a[1], a[2]], [b[0], b[1], b[2]]))
    };
    assert!((before - after).abs() < 1e-12, "{before} vs {after}");
    // Centerline in the band follows the circle of radius 1/rate:
    // point (0, y, 0.5) → (1−cos(0.5), y, sin(0.5)).
    let tip = &bent.control_points[0][2];
    let phi = 0.25 * 2. * rate; // z = 0.5 → φ = 0.5
    assert!((tip[0] - (1. - phi.cos())).abs() < 1e-9, "x={}", tip[0]);
    assert!((tip[2] - phi.sin()).abs() < 1e-9, "z={}", tip[2]);
}

#[test]
fn wire_with_zero_difference_is_identity() {
    let deformer = WireDeformer::new(vec![straight_wire(0.)]).unwrap();
    let points = [[0.3, 0.1, 0.2], [0.7, -0.4, 0.1], [0.5, 0., 0.]];
    for p in points {
        let q = deformer.deform_point(p).unwrap();
        for axis in 0..3 {
            assert!((q[axis] - p[axis]).abs() < 1e-12);
        }
    }
}

#[test]
fn wire_pulls_points_toward_the_target_curve() {
    let deformer = WireDeformer::new(vec![straight_wire(0.4)]).unwrap();
    // Point on the wire gets (nearly) the full displacement.
    let on = deformer.deform_point([0.5, 0., 0.]).unwrap();
    assert!((on[2] - 0.4).abs() < 1e-9, "z={}", on[2]);
    // Far point is barely affected (d = 2, r = 0.5 → w = e⁻¹⁶).
    let far = deformer.deform_point([0.5, 2., 0.]).unwrap();
    assert!(far[2] < 1e-5, "far point moved by {}", far[2]);
    assert!(far[2] > 0.);
}

#[test]
fn duplicate_wires_do_not_double_displace() {
    let single = WireDeformer::new(vec![straight_wire(0.4)]).unwrap();
    let doubled = WireDeformer::new(vec![straight_wire(0.4), straight_wire(0.4)]).unwrap();
    let p = [0.4, 0.2, 0.1];
    let a = single.deform_point(p).unwrap();
    let b = doubled.deform_point(p).unwrap();
    for axis in 0..3 {
        assert!(
            (a[axis] - b[axis]).abs() < 1e-15,
            "duplicate wires changed the result: {a:?} vs {b:?}"
        );
    }
}

#[test]
fn ffd_cache_canonicalizes_signed_zero() {
    let lattice = FfdLattice::new([0., 0., 0.], [1., 1., 1.], [3, 3, 3], [2, 2, 2]).unwrap();
    // -0.0 и +0.0 — одна каноническая запись кэша.
    let a = lattice.parameterize([0.0, 0.25, 0.75]).unwrap();
    let b = lattice.parameterize([-0.0, 0.25, 0.75]).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        lattice.cache.borrow().len(),
        1,
        "signed zeros must share one cache entry"
    );
    // Различные координаты — разные записи.
    lattice.parameterize([0.5, 0.25, 0.75]).unwrap();
    assert_eq!(lattice.cache.borrow().len(), 2);
}

#[test]
fn ffd_rejects_nan_query_point() {
    let lattice = FfdLattice::new([0., 0., 0.], [1., 1., 1.], [3, 3, 3], [2, 2, 2]).unwrap();
    let err = lattice.parameterize([f64::NAN, 0., 0.]).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(lattice.deform_point([0.5, f64::INFINITY, 0.5]).is_err());
}

#[test]
fn wire_newton_budget_is_a_named_stage() {
    // Истощение ньютоновского бюджета носит имя стадии.
    let mut guard = Budget::with_iterations(2)
        .unwrap()
        .guard("deform.wire-newton");
    guard.tick().unwrap();
    guard.tick().unwrap();
    let err = guard.tick().unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
    assert!(err.contains("deform.wire-newton"), "{err}");
}
