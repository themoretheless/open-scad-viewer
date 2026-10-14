#[cfg(feature = "codec")]
use value_codec::json;

use super::*;
use crate::curve::Curve;
fn wire(points: Vec<Vec<f64>>) -> Curve {
    let n = points.len();
    let mut knots = vec![0.];
    knots.extend((0..n).map(|i| i as f64));
    knots.push((n - 1) as f64);
    Curve {
        degree: 1,
        knots,
        weights: vec![1.; n],
        control_points: points,
        periodic: false,
    }
}
#[test]
fn edits_change_current_diagnostics_without_offset_metadata() {
    let square = wire(vec![
        vec![0., 0., 7.],
        vec![2., 0., 7.],
        vec![2., 2., 7.],
        vec![0., 2., 7.],
        vec![0., 0., 7.],
    ]);
    assert!(inspect_curves(&[square], 100).unwrap().crossings.is_empty());
    let crossed = wire(vec![
        vec![0., 0., 7.],
        vec![2., 2., 7.],
        vec![0., 2., 7.],
        vec![2., 0., 7.],
        vec![0., 0., 7.],
    ]);
    assert_eq!(
        inspect_curves(&[crossed], 100).unwrap().crossings,
        vec![[0, 2]]
    );
}
#[cfg(feature = "transport")]
#[test]
fn transport_dispatch_uses_current_curve_definitions() {
    let curve = wire(vec![
        vec![0., 0.],
        vec![2., 2.],
        vec![0., 2.],
        vec![2., 0.],
        vec![0., 0.],
    ]);
    let value=crate::transport::dispatch(json!({"op":"curve_chain_diagnostics","curves":[value_codec::to_value(curve).unwrap()],"maxPairs":100})).unwrap();
    assert_eq!(value["crossings"], json!([[0, 2]]));
    assert_eq!(value["originalOffsetTopologyCertified"], false);
}
#[test]
fn disconnected_nonplanar_and_discontinuous_inputs_are_refused() {
    let a = wire(vec![vec![0., 0.], vec![1., 0.]]);
    let b = wire(vec![vec![2., 0.], vec![3., 0.]]);
    assert!(inspect_curves(&[a, b], 100).is_err());
    let a = wire(vec![vec![0., 0., 0.], vec![1., 0., 1.]]);
    assert!(inspect_curves(&[a], 100).is_err());
    let mut a = wire(vec![vec![0., 0.], vec![1., 0.], vec![2., 0.], vec![3., 0.]]);
    a.knots[3] = a.knots[2];
    assert!(inspect_curves(&[a], 100).is_err());
}
