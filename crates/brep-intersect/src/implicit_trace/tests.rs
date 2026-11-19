use super::*;

#[test]
fn bernstein_products_bound_polynomials() {
    // (t)(1 - t) has Bernstein coefficients [0, 1/2, 0] in degree 2 and a
    // maximum of 1/4.
    let t = Bern(vec![Interval::point(0.), Interval::point(1.)]);
    let u = Bern(vec![Interval::point(1.), Interval::point(0.)]);
    let p = t.mul(&u).unwrap();
    assert_eq!(p.degree(), 2);
    assert!((p.magnitude() - 0.5).abs() < 1e-15);
    let elevated = p.elevate_to(4).unwrap();
    assert!(elevated.magnitude() <= p.magnitude() + 1e-15);
    assert!(elevated.magnitude() >= 0.25);
}

#[test]
fn certified_off_axis_sphere_torus_curves() {
    let torus = TorusImplicit {
        center: [0.; 3],
        frame: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        major: 3.,
        minor: 1.,
    };
    let sphere = SphereImplicit {
        center: [3.5, 0.4, 0.3],
        radius: 1.,
    };
    let seed = |theta: f64, phi: f64| -> P {
        let (st, ct) = theta.sin_cos();
        let (sp, cp) = phi.sin_cos();
        [3.5 + st * cp, 0.4 + st * sp, 0.3 + ct]
    };
    let loops = trace_intersection(&sphere, &torus, seed, 5., 1., 1e-7).unwrap();
    assert!(!loops.is_empty());
    for l in &loops {
        assert!(l.distance_bound <= 1e-7, "{}", l.distance_bound);
        // Independent check: dense samples lie on both surfaces.
        for piece in &l.pieces {
            for k in 0..=50 {
                let p = piece.evaluate(k as f64 / 50.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                let ds = (norm(sub(p, sphere.center)) - 1.).abs();
                let rho = p[0].hypot(p[1]);
                let dt = ((rho - 3.).hypot(p[2]) - 1.).abs();
                assert!(ds <= 1e-7 && dt <= 1e-7, "{ds} {dt}");
            }
        }
        // Pieces chain into a closed loop with exactly shared ends.
        for w in l.pieces.windows(2) {
            assert_eq!(w[0].control_points.last(), w[1].control_points.first());
        }
        assert_eq!(
            l.pieces.last().unwrap().control_points.last(),
            l.pieces[0].control_points.first()
        );
    }
}
