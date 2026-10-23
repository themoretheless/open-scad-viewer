//! Frozen v1 rational geometry identity, computed directly from the rational definition.
//! Existing persisted topology IDs used the ordered surface wire representation
//! as their signature. Preserve those bytes without constructing dynamic values.
use nurbs_core::{curve::Curve, surface::Surface};
use std::fmt::Write;

fn sequence<T>(out: &mut String, values: &[T], mut append: impl FnMut(&mut String, &T)) {
    out.push('[');
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        append(out, value);
    }
    out.push(']');
}
fn number(out: &mut String, value: &f64) {
    // v1 distinguishes signed zero and uses explicit signs on positive exponents.
    // Invalid definitions historically encoded nonfinite coordinates as null.
    if !value.is_finite() {
        out.push_str("null");
        return;
    }
    let text = format!("{value:?}");
    if let Some((mantissa, exponent)) = text.split_once('e')
        && !exponent.starts_with('-')
    {
        write!(out, "{mantissa}e+{exponent}").unwrap();
    } else {
        out.push_str(&text);
    }
}
fn vector(out: &mut String, values: &Vec<f64>) {
    sequence(out, values, number);
}
fn grid(out: &mut String, values: &Vec<Vec<f64>>) {
    sequence(out, values, vector);
}

pub(super) fn surface_signature(surface: &Surface) -> String {
    let mut out = String::new();
    out.push_str("{\"controlPoints\":");
    sequence(&mut out, &surface.control_points, grid);
    write!(
        &mut out,
        ",\"degreeU\":{},\"degreeV\":{},\"knotsU\":",
        surface.degree_u, surface.degree_v
    )
    .unwrap();
    vector(&mut out, &surface.knots_u);
    out.push_str(",\"knotsV\":");
    vector(&mut out, &surface.knots_v);
    write!(
        &mut out,
        ",\"periodicU\":{},\"periodicV\":{},\"weights\":",
        surface.periodic_u, surface.periodic_v
    )
    .unwrap();
    grid(&mut out, &surface.weights);
    out.push('}');
    out
}

pub(super) fn curve_signature(curve: &Curve) -> String {
    let mut out = String::new();
    out.push_str("{\"controlPoints\":");
    grid(&mut out, &curve.control_points);
    write!(&mut out, ",\"degree\":{},\"knots\":", curve.degree).unwrap();
    vector(&mut out, &curve.knots);
    write!(&mut out, ",\"periodic\":{},\"weights\":", curve.periodic).unwrap();
    vector(&mut out, &curve.weights);
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(feature = "codec")]
    fn retains_existing_signature_bytes_for_all_surface_fields_and_numeric_scales() {
        let mut surface = crate::cuboid([0.; 3], [2., 3., 4.]).unwrap().faces[0]
            .surface
            .clone();
        for value in [0., -0., 1e-300, 1e200, f64::from_bits(1), f64::MAX, 1. / 3.] {
            surface.control_points[0][0][0] = value;
            surface.weights[0][0] = value.abs();
            for periodic in [false, true] {
                surface.periodic_u = periodic;
                surface.periodic_v = !periodic;
                assert_eq!(
                    surface_signature(&surface),
                    value_codec::to_string(&surface).unwrap()
                );
            }
        }
    }

    #[test]
    #[cfg(feature = "codec")]
    fn curve_identity_retains_the_previous_numeric_and_periodic_signature() {
        let mut curve = crate::cuboid([0.; 3], [2., 3., 4.]).unwrap().edges[0]
            .curve
            .clone();
        for value in [0., -0., 1e-300, 1e200, f64::from_bits(1), f64::MAX, 1. / 3.] {
            curve.control_points[0][0] = value;
            curve.weights[0] = value.abs();
            for periodic in [false, true] {
                curve.periodic = periodic;
                assert_eq!(
                    curve_signature(&curve),
                    value_codec::to_string(&curve).unwrap()
                );
            }
        }
    }
}
