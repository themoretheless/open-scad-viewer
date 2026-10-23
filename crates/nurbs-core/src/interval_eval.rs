//! Public certified interval evaluation: outward-rounded enclosures of a
//! curve over a parameter interval, or of a surface over a uv box.
//!
//! These wrap the same outward-rounded homogeneous de Boor machinery that
//! backs `curve_distance::enclosure` / `surface_distance::enclosure`. Every
//! arithmetic operation rounds outward, so the returned per-coordinate
//! intervals are certified outer bounds of the exact image. Tightness is
//! limited by the underlying machinery: the construction is convex-hull and
//! blossom based, which avoids interval-parameter dependency inside a single
//! knot cell, but the union across cells is a plain box union.
use crate::curve::Curve;
use crate::surface::Surface;
use crate::{Result, check, numeric_err};

/// Outward-rounded binary64 interval, promoted from the shared distance
/// machinery for public use.
pub use crate::distance_bounds::Interval;

/// Certified per-coordinate enclosure of `curve` over the parameter interval
/// `u`. The result contains `curve.evaluate(t).point` for every `t ∈ u`.
/// `u` must overlap the active knot domain; a box that misses it entirely is
/// an input error.
pub fn evaluate_interval(curve: &Curve, u: Interval) -> Result<Vec<Interval>> {
    curve.validate()?;
    let [a, b] = curve.domain();
    check(
        u.lo <= b && u.hi >= a,
        "Parameter interval is outside the active knot domain",
    )?;
    let mut enclosure: Option<Vec<Interval>> = None;
    for span in curve.degree..curve.control_points.len() {
        let (ka, kb) = (curve.knots[span], curve.knots[span + 1]);
        if ka >= kb || kb < u.lo || ka > u.hi {
            continue;
        }
        let t = Interval::new(u.lo.max(ka), u.hi.min(kb))?;
        let part = crate::curve_distance::enclosure(curve, span, t)?;
        enclosure = Some(match enclosure {
            None => part,
            Some(mut acc) => {
                for (a, p) in acc.iter_mut().zip(part) {
                    *a = Interval::new(a.lo.min(p.lo), a.hi.max(p.hi))?;
                }
                acc
            }
        });
    }
    enclosure.ok_or_else(|| numeric_err("Parameter interval misses every nonempty knot span"))
}

/// Certified per-coordinate enclosure of `surface` over the uv box
/// `u × v`. The result contains `surface.evaluate(s, t).point` for every
/// `(s, t)` in the box. The box must overlap the active domain rectangle.
pub fn evaluate_surface_interval(
    surface: &Surface,
    u: Interval,
    v: Interval,
) -> Result<Vec<Interval>> {
    surface.validate()?;
    let du = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let dv = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    check(
        u.lo <= du[1] && u.hi >= du[0] && v.lo <= dv[1] && v.hi >= dv[0],
        "Parameter box is outside the active knot domain",
    )?;
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let mut enclosure: Option<Vec<Interval>> = None;
    for iu in surface.degree_u..nu {
        let (ka, kb) = (surface.knots_u[iu], surface.knots_u[iu + 1]);
        if ka >= kb || kb < u.lo || ka > u.hi {
            continue;
        }
        for iv in surface.degree_v..nv {
            let (la, lb) = (surface.knots_v[iv], surface.knots_v[iv + 1]);
            if la >= lb || lb < v.lo || la > v.hi {
                continue;
            }
            let domain = [
                [u.lo.max(ka), u.hi.min(kb)],
                [v.lo.max(la), v.hi.min(lb)],
            ];
            let part = crate::surface_distance::enclosure(surface, [iu, iv], domain)?;
            enclosure = Some(match enclosure {
                None => part,
                Some(mut acc) => {
                    for (a, p) in acc.iter_mut().zip(part) {
                        *a = Interval::new(a.lo.min(p.lo), a.hi.max(p.hi))?;
                    }
                    acc
                }
            });
        }
    }
    enclosure.ok_or_else(|| numeric_err("Parameter box misses every nonempty knot cell"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        }
        fn f64(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
        fn range(&mut self, a: f64, b: f64) -> f64 {
            a + (b - a) * self.f64()
        }
    }

    fn random_curve(rng: &mut Rng) -> Curve {
        let p = 1 + (rng.next() % 4) as usize;
        let interior = 1 + (rng.next() % 5) as usize;
        let mut knots = vec![0.; p + 1];
        for i in 0..interior {
            knots.push((i + 1) as f64 / (interior + 1) as f64);
        }
        knots.extend(vec![1.; p + 1]);
        let n = knots.len() - p - 1;
        let control_points = (0..n)
            .map(|_| (0..3).map(|_| rng.range(-5., 5.)).collect::<Vec<f64>>())
            .collect();
        let weights = (0..n).map(|_| rng.range(0.5, 2.)).collect();
        let curve = Curve {
            degree: p,
            knots,
            control_points,
            weights,
            periodic: false,
        };
        curve.validate().unwrap();
        curve
    }

    /// A curved bicubic-ish test surface on [0,1]² with rational weights.
    fn test_surface() -> Surface {
        let control_points = vec![
            vec![
                vec![0., 0., 0.],
                vec![0., 0.5, 1.],
                vec![0., 1., 0.],
            ],
            vec![
                vec![0.5, 0., 1.],
                vec![0.5, 0.5, 2.],
                vec![0.5, 1., 1.],
            ],
            vec![
                vec![1., 0., 0.],
                vec![1., 0.5, 1.],
                vec![1., 1., 0.],
            ],
        ];
        let s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![
                vec![1., 1.5, 1.],
                vec![1.5, 2., 1.5],
                vec![1., 1.5, 1.],
            ],
            control_points,
            periodic_u: false,
            periodic_v: false,
        };
        s.validate().unwrap();
        s
    }

    #[test]
    fn curve_enclosure_contains_all_samples() {
        let mut rng = Rng(7);
        for _ in 0..10 {
            let c = random_curve(&mut rng);
            let lo = rng.range(0., 0.5);
            let hi = rng.range(0.5, 1.);
            let box_ = evaluate_interval(&c, Interval::new(lo, hi).unwrap()).unwrap();
            for _ in 0..256 {
                let u = rng.range(lo, hi);
                let p = c.evaluate(u).unwrap().point;
                for (axis, &x) in p.iter().enumerate() {
                    assert!(
                        box_[axis].contains(x),
                        "axis {axis}: {x} outside [{}, {}] at u={u}",
                        box_[axis].lo,
                        box_[axis].hi
                    );
                }
            }
            // Degenerate interval still encloses the point evaluation.
            let point_box = evaluate_interval(&c, Interval::point(0.3)).unwrap();
            let p = c.evaluate(0.3).unwrap().point;
            for (axis, &x) in p.iter().enumerate() {
                assert!(point_box[axis].contains(x));
            }
        }
    }

    #[test]
    fn curve_enclosure_width_shrinks_with_interval() {
        let mut rng = Rng(13);
        for _ in 0..10 {
            let c = random_curve(&mut rng);
            let outer = evaluate_interval(&c, Interval::new(0.2, 0.8).unwrap()).unwrap();
            let inner = evaluate_interval(&c, Interval::new(0.4, 0.6).unwrap()).unwrap();
            let tiny = evaluate_interval(&c, Interval::new(0.49, 0.51).unwrap()).unwrap();
            for axis in 0..3 {
                assert!(inner[axis].width() <= outer[axis].width() + 1e-12);
                assert!(tiny[axis].width() <= inner[axis].width() + 1e-12);
            }
        }
    }

    #[test]
    fn surface_enclosure_contains_all_samples() {
        let s = test_surface();
        let mut rng = Rng(29);
        let (u, v) = (Interval::new(0.1, 0.9).unwrap(), Interval::new(0.2, 0.8).unwrap());
        let box_ = evaluate_surface_interval(&s, u, v).unwrap();
        assert_eq!(box_.len(), 3);
        for _ in 0..256 {
            let p = s.evaluate(rng.range(0.1, 0.9), rng.range(0.2, 0.8)).unwrap().point;
            for axis in 0..3 {
                assert!(
                    box_[axis].contains(p[axis]),
                    "axis {axis}: {} outside [{}, {}]",
                    p[axis],
                    box_[axis].lo,
                    box_[axis].hi
                );
            }
        }
        // Width shrinks with the box.
        let small_u = Interval::new(0.4, 0.6).unwrap();
        let small_v = Interval::new(0.4, 0.6).unwrap();
        let small = evaluate_surface_interval(&s, small_u, small_v).unwrap();
        for axis in 0..3 {
            assert!(small[axis].width() <= box_[axis].width() + 1e-12);
        }
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        let c = random_curve(&mut Rng(3));
        // Reversed / non-finite intervals fail at construction.
        assert!(Interval::new(0.5, 0.25).is_err());
        assert!(Interval::new(f64::NAN, 1.).is_err());
        assert!(Interval::new(0., f64::INFINITY).is_err());
        // Boxes fully outside the active domain are input errors.
        assert!(evaluate_interval(&c, Interval::new(2., 3.).unwrap()).is_err());
        assert!(evaluate_interval(&c, Interval::new(-3., -1.).unwrap()).is_err());
        let s = test_surface();
        let inside = Interval::new(0.2, 0.8).unwrap();
        let outside = Interval::new(5., 6.).unwrap();
        assert!(evaluate_surface_interval(&s, outside, inside).is_err());
        assert!(evaluate_surface_interval(&s, inside, outside).is_err());
    }
}
