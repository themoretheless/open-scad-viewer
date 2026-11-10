//! Tests of `nurbs-numerics` that need higher nurbs-core layers (curves, interval Newton,
//! transport). They moved here unchanged when `numerics` became its own crate.
use crate::numerics::{error_upper::add, sturm::{count_roots, isolate_sturm}};
use crate::distance_bounds::Interval;
use crate::interval_newton::{PolynomialFunction, isolate_roots};

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

fn from_roots(roots: &[f64]) -> Vec<f64> {
    let mut c = vec![1.];
    for &r in roots {
        let mut n = vec![0.; c.len() + 1];
        for (i, &a) in c.iter().enumerate() {
            n[i] -= a * r;
            n[i + 1] += a;
        }
        c = n;
    }
    c
}

#[test]
fn curve_constructor_rejects_nan_control_point() {
    let points = vec![
        vec![0.0, 0.0],
        vec![f64::NAN, 1.0],
        vec![2.0, 0.0],
    ];
    let err = crate::curve::Curve::from_polyline(points).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("control_points[1]"), "{err}");
}

#[test]
fn curve_constructor_rejects_nan_weight_as_non_finite() {
    let curve = crate::curve::Curve {
        degree: 1,
        knots: vec![0.0, 0.0, 1.0, 1.0],
        control_points: vec![vec![0.0, 0.0], vec![1.0, 1.0]],
        weights: vec![1.0, f64::NAN],
        periodic: false,
    };
    let err = curve.validate().unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    // NaN must surface as a non-finite rejection, not as "weight ≤ 0".
    assert!(err.contains("weights[1]"), "{err}");
    assert!(err.contains("positive, finite"), "{err}");
    assert!(!err.contains("[1e-12, 1e12]"), "{err}");
}

#[test]
fn curve_constructor_rejects_inf_knot() {
    let curve = crate::curve::Curve {
        degree: 1,
        knots: vec![0.0, 0.0, f64::INFINITY, 1.0],
        control_points: vec![vec![0.0, 0.0], vec![1.0, 1.0]],
        weights: vec![1.0, 1.0],
        periodic: false,
    };
    let err = curve.validate().unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("knots[2] must be finite"), "{err}");
}

#[test]
fn cross_check_against_interval_newton() {
    // Deterministic random polynomials: Sturm count must match the
    // number of certified boxes from interval Newton isolation.
    let mut rng = Rng(2027);
    for _ in 0..20 {
        let count = 1 + (rng.next() % 4) as usize;
        let roots: Vec<f64> = (0..count).map(|_| rng.range(-1.5, 1.5)).collect();
        let p = from_roots(&roots);
        let sturm = count_roots(&p, -2., 2.).unwrap();
        let newton = isolate_roots(
            Interval::new(-2., 2.).unwrap(),
            &PolynomialFunction::new(p.clone()).unwrap(),
            1e-7,
            40,
        )
        .unwrap();
        assert_eq!(sturm, newton.len(), "roots {roots:?}");
        let boxes = isolate_sturm(&p, -2., 2., 1e-7).unwrap();
        for (nb, _) in &newton {
            assert!(boxes.iter().any(|sb| sb.hi >= nb.lo && sb.lo <= nb.hi));
        }
    }
}

    #[cfg(feature="transport")]
#[test]
fn transport_owns_rounding_and_complete_boundary_admission() {
    use value_codec::{json,Value};
    let r=crate::transport::dispatch(json!({"op":"sweep_error_upper_compose","kind":"add","a":1.,"b":2_f64.powi(-53)})).unwrap();
    assert_eq!(r["errorUpper"].as_f64(),add(1.,2_f64.powi(-53)));
    let overflow=crate::transport::dispatch(json!({"op":"sweep_error_upper_compose","kind":"multiply","a":f64::MAX,"b":2.})).unwrap();
    assert!(overflow["errorUpper"].is_null());
    let request=json!({"op":"sweep_boundary_certificate","wall":0.1,"caps":[0.2,0.3],"closed":false,"budget":0.25});
    let r=crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(r["continuousBound"],true); assert_eq!(r["withinBudget"],false);
    assert_eq!(r["reason"],"boundary-budget-exceeded");
    let mut missing=request.clone();missing["caps"]=Value::Null;
    assert_eq!(crate::transport::dispatch(missing.clone()).unwrap()["reason"],"filled-cap-bound-unproved");
    missing["closed"]=json!(true);
    assert_eq!(crate::transport::dispatch(missing).unwrap()["errorUpper"],0.1);
    let mut invalid=request; invalid["budget"]=Value::Null;
    assert_eq!(crate::transport::dispatch(invalid).unwrap()["reason"],"invalid-budget");
}
