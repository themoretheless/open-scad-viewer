use super::*;
use num_traits::ToPrimitive;

#[test]
fn exact_area_certificate_checks_holes_and_rejects_forged_or_stale_bounds() {
    let polygon = |a: [f64; 2], b: [f64; 2]| {
        crate::sketch::polygon_wire(vec![a, [b[0], a[1]], b, [a[0], b[1]]]).unwrap()
    };
    let hole = polygon([2., 2.], [8., 8.])
        .into_iter()
        .rev()
        .map(|c| c.reverse().unwrap())
        .collect();
    let loops = vec![
        polygon([0., 0.], [10., 10.]),
        hole,
        polygon([4., 4.], [6., 6.]),
    ];
    let c = certify_measure(&loops, 1e-7, 1e-9).unwrap();
    assert_eq!(c.area_mm2.lower, ["68".to_string(), "1".to_string()]);
    assert_eq!(c.area_mm2.lower, c.area_mm2.upper);
    verify_measure(&loops, 1e-7, &c).unwrap();
    for ratio in [
        ["69", "1"],
        ["-1", "0"],
        ["068", "1"],
        ["136", "2"],
        ["1", "-1"],
    ] {
        let mut bad = c.clone();
        bad.area_mm2.lower = ratio.map(str::to_string);
        assert!(verify_measure(&loops, 1e-7, &bad).is_err());
    }
    let mut bad = c.clone();
    bad.area_depth = 255;
    assert!(verify_measure(&loops, 1e-7, &bad).is_err());
    let mut bad = c.clone();
    bad.topology.source.push('0');
    assert!(verify_measure(&loops, 1e-7, &bad).is_err());
    assert!(verify_measure(&loops, 1e-6, &c).is_err());
    assert!(certify_measure(&loops, 1e-7, f64::NAN).is_err());
    #[cfg(feature = "codec")]
    {
        let json = value_codec::to_string(&c).unwrap();
        let decoded: ProfileMeasureCertificate = value_codec::from_str(&json).unwrap();
        assert_eq!(decoded, c);
        let mut v = value_codec::to_value(&c).unwrap();
        v["solidValid"] = value_codec::Value::Bool(true);
        assert!(value_codec::from_value::<ProfileMeasureCertificate>(v).is_err());
        let mut v = value_codec::to_value(&c).unwrap();
        v["scope"] = value_codec::Value::String("solid-validity".into());
        assert!(value_codec::from_value::<ProfileMeasureCertificate>(v).is_err());
    }
}

#[test]
fn rational_ellipse_gets_a_tight_exact_green_enclosure() {
    let mut loops = vec![crate::sketch::circle_wire(2.).unwrap()];
    for c in &mut loops[0] {
        for p in &mut c.control_points {
            p[0] = 3. * p[0] + p[1];
            p[1] *= 2.;
        }
    }
    let c = certify_measure(&loops, 1e-7, 1e-6).unwrap();
    verify_measure(&loops, 1e-7, &c).unwrap();
    let ratio = |s: &[String; 2]| Q::new(s[0].parse().unwrap(), s[1].parse().unwrap());
    let (lo, hi) = (ratio(&c.area_mm2.lower), ratio(&c.area_mm2.upper));
    assert!((&hi - &lo) <= Q::from_float(1e-6).unwrap());
    let expected = 24. * std::f64::consts::PI;
    assert!(lo.to_f64().unwrap() < expected && hi.to_f64().unwrap() > expected);
    let mut bad = c.clone();
    bad.area_mm2.upper = ["75".into(), "1".into()];
    assert!(verify_measure(&loops, 1e-7, &bad).is_err());
}

#[test]
fn empty_material_has_exact_zero_area() {
    let c = certify_measure(&[], 1e-7, 1e-9).unwrap();
    assert_eq!(c.area_mm2.lower, ["0".to_string(), "1".to_string()]);
    verify_measure(&[], 1e-7, &c).unwrap();
}
