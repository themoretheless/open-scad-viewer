#![cfg(feature = "transport")]
#[path = "support/exact_primitive_transport.rs"]
mod support;
use nurbs_core::{
    curve::Curve,
    primitives::{
        exact_conics as C, exact_frusta as F, exact_planar_regions as P, exact_quadrics as Q,
        exact_tori as T,
    },
    surface::Surface,
};
use support::*;
use value_codec::{Value, json};
const UNIT: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
#[test]
fn ellipse_wire_matches_the_fresh_typed_native_report() {
    let r = C::ellipse([0.; 3], UNIT[0], UNIT[1], 1_000_000).unwrap();
    let v = constructed(&requests()[0]);
    let c: Curve = value_codec::from_value(v["geometry"].clone()).unwrap();
    assert_eq!(Some(c), r.curve);
    assert_eq!(v["workUsed"].as_u64(), Some(r.work_used));
    assert_eq!(v["geometryKind"], "curve");
    assert_eq!(v["scope"], "literal-curve-geometry");
}
#[test]
fn ellipsoid_wire_matches_the_fresh_typed_native_report() {
    let r = Q::ellipsoid([0.; 3], UNIT, 1_000_000).unwrap();
    let v = constructed(&requests()[1]);
    let s: Surface = value_codec::from_value(v["geometry"].clone()).unwrap();
    assert_eq!(Some(s), r.surface);
    assert_eq!(v["workUsed"].as_u64(), Some(r.work_used));
    assert_eq!(v["geometryKind"], "surface");
}
#[test]
fn ring_torus_wire_matches_the_fresh_typed_native_report() {
    let r = T::ring([0.; 3], UNIT, 2., 1., 1_000_000).unwrap();
    let v = constructed(&requests()[2]);
    let s: Surface = value_codec::from_value(v["geometry"].clone()).unwrap();
    assert_eq!(Some(s), r.surface);
    assert_eq!(v["workUsed"].as_u64(), Some(r.work_used));
}
#[test]
fn frustum_metadata_distinguishes_sharp_apices_from_regular_sides() {
    for radii in [[1., 1.], [2., 1.], [2., 0.], [0., 2.]] {
        let mut request = requests()[3].clone();
        request["radii"] = json!(radii);
        let r = F::side([0.; 3], UNIT, radii, 1_000_000).unwrap();
        let v = constructed(&request);
        let s: Surface = value_codec::from_value(v["geometry"].clone()).unwrap();
        assert_eq!(Some(s), r.surface);
        assert_eq!(v["workUsed"].as_u64(), Some(r.work_used));
        let expected = r.collapsed_end.map(|e| match e {
            F::End::Start => "start",
            F::End::End => "end",
        });
        assert_eq!(v["metadata"]["collapsedEnd"].as_str(), expected);
        assert_eq!(v["metadata"]["intrinsicApex"], expected.is_some());
    }
}
#[test]
fn planar_metadata_distinguishes_regular_center_from_a_real_hole() {
    for radii in [[0., 1.], [1., 2.]] {
        let mut request = requests()[4].clone();
        request["radii"] = json!(radii);
        let r = P::radial([0.; 3], [UNIT[0], UNIT[1]], radii, 1_000_000).unwrap();
        let v = constructed(&request);
        let s: Surface = value_codec::from_value(v["geometry"].clone()).unwrap();
        assert_eq!(Some(s), r.surface);
        assert_eq!(v["workUsed"].as_u64(), Some(r.work_used));
        assert_eq!(
            v["metadata"]["regionKind"],
            if radii[0] == 0. {
                "filledDisk"
            } else {
                "annulus"
            }
        );
        assert_eq!(v["metadata"]["collapsedCenter"], radii[0] == 0.);
        assert_eq!(v["metadata"]["regularPlaneSupport"], true);
    }
}
#[test]
fn subnormal_oblique_disk_survives_actual_text_json_roundtrip() {
    let t = f64::from_bits(1);
    let axes = [[t, t, 0.], [t, 0., t]];
    let mut request = requests()[4].clone();
    request["axes"] = json!(axes);
    request["radii"] = json!([0., 2.]);
    let r = P::radial([0.; 3], axes, [0., 2.], 1_000_000).unwrap();
    let v = constructed(&request);
    let s: Surface = value_codec::from_value(v["geometry"].clone()).unwrap();
    assert_eq!(Some(s), r.surface);
}
#[test]
fn large_exact_integer_radius_with_small_axes_keeps_bounded_geometry() {
    let radius = 1u64 << 60;
    let scale = 2f64.powi(-60);
    let axes = [[scale, 0., 0.], [0., scale, 0.]];
    let mut request = requests()[4].clone();
    request["axes"] = json!(axes);
    request["radii"] = json!([0u64, radius]);
    let r = P::radial([0.; 3], axes, [0., radius as f64], 1_000_000).unwrap();
    let v = constructed(&request);
    let s: Surface = value_codec::from_value(v["geometry"].clone()).unwrap();
    assert_eq!(s.control_points[0][1], vec![1., 0., 0.]);
    assert_eq!(Some(s), r.surface);
}
#[test]
fn legacy_primitive_operation_keeps_its_existing_raw_surface_contract() {
    let request = json!({"op":"surface_ellipsoid","center":[0.,0.,0.],"radii":[2.,3.,4.]});
    let r = envelope(&request);
    assert_eq!(r["ok"], true);
    assert!(r["value"]["geometry"].is_null());
    let actual: Surface = value_codec::from_value(r["value"].clone()).unwrap();
    assert_eq!(
        actual,
        nurbs_core::primitives::ellipsoid([0.; 3], [2., 3., 4.]).unwrap()
    );
}
