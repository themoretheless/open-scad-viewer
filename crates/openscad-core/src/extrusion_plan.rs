//! Stable extrusion parameters before profile-dependent subdivision selection.
use crate::primitive_plan::Size;
#[derive(Debug, PartialEq)]
pub struct RevolutionParameters {
    pub angle: f64,
    pub angle_defaulted: bool,
    pub side: &'static str,
    pub empty: bool,
    pub reflect: bool,
    pub radius: f64,
}
/// Stable angle normalization and classification of a profile about the Y axis.
pub fn revolution_parameters(
    angle: Option<f64>,
    angle_provided: bool,
    minimum: f64,
    maximum: f64,
) -> RevolutionParameters {
    let valid = angle.is_some_and(f64::is_finite);
    let mut angle = angle.filter(|v| v.is_finite()).unwrap_or(360.);
    if angle <= -360. || angle > 360. {
        angle = 360.;
    }
    let side = geometry_ops::revolution::profile_side(minimum, maximum);
    RevolutionParameters {
        angle,
        angle_defaulted: angle_provided && !valid,
        side,
        empty: angle == 0. || side == "axis" || side == "crossing",
        reflect: side == "negative",
        radius: minimum.abs().max(maximum.abs()),
    }
}
#[derive(Debug, PartialEq)]
pub struct Parameters {
    pub height: f64,
    pub scale: [f64; 2],
    pub twist: f64,
    pub center: bool,
    pub explicit_slices: Option<f64>,
    pub empty: bool,
    pub height_defaulted: bool,
    pub scale_defaulted: bool,
}
pub fn parameters(
    height: Option<f64>,
    height_provided: bool,
    scale: &Size,
    twist: Option<f64>,
    center: bool,
    slices: Option<f64>,
) -> Parameters {
    let height_valid = height.is_some_and(f64::is_finite);
    let height = if height_valid {
        height.unwrap().max(0.)
    } else {
        100.
    };
    let scale_value = match scale {
        Size::Missing => Some([1.; 2]),
        Size::Scalar(value) if value.is_finite() => Some([*value; 2]),
        Size::Vector(values) if values.len() == 2 => match (values[0], values[1]) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Some([x, y]),
            _ => None,
        },
        _ => None,
    };
    Parameters {
        height,
        scale: scale_value.unwrap_or([1.; 2]).map(|value| value.max(0.)),
        twist: twist.filter(|value| value.is_finite()).unwrap_or(0.),
        center,
        explicit_slices: slices
            .filter(|value| value.is_finite())
            .map(f64::trunc)
            .filter(|value| *value > 0.),
        empty: height == 0.,
        height_defaulted: height_provided && !height_valid,
        scale_defaulted: scale_value.is_none(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revolution_angles_and_profile_sides() {
        for (angle, expected) in [
            (-361., 360.),
            (-360., 360.),
            (-359., -359.),
            (0., 0.),
            (360., 360.),
            (361., 360.),
        ] {
            let plan = revolution_parameters(Some(angle), true, 1., 2.);
            assert_eq!(plan.angle, expected);
            assert!(!plan.angle_defaulted);
            assert_eq!(plan.empty, angle == 0.);
        }
        let plan = revolution_parameters(Some(f64::NAN), true, -3., -1.);
        assert!(plan.angle_defaulted && plan.reflect);
        assert_eq!(plan.radius, 3.);
        assert_eq!(plan.side, "negative");
        assert!(revolution_parameters(None, false, -1., 1.).empty);
        assert_eq!(revolution_parameters(None, false, 0., 0.).side, "axis");
        assert!(!revolution_parameters(None, false, 0., 1.).angle_defaulted);
    }
    #[test]
    fn defaults_partial_vectors_clamping_and_explicit_slices() {
        let plan = parameters(None, false, &Size::Missing, None, false, None);
        assert_eq!(plan.height, 100.);
        assert_eq!(plan.scale, [1.; 2]);
        assert!(!plan.height_defaulted);
        let plan = parameters(
            Some(-2.),
            true,
            &Size::Vector(vec![Some(-1.), Some(2.)]),
            Some(f64::INFINITY),
            true,
            Some(3.9),
        );
        assert!(plan.empty);
        assert_eq!(plan.scale, [0., 2.]);
        assert_eq!(plan.twist, 0.);
        assert_eq!(plan.explicit_slices, Some(3.));
        let plan = parameters(
            Some(f64::NAN),
            true,
            &Size::Vector(vec![Some(2.), None]),
            None,
            false,
            Some(-2.),
        );
        assert!(plan.height_defaulted && plan.scale_defaulted);
        assert_eq!(plan.scale, [1.; 2]);
        assert_eq!(plan.explicit_slices, None);
    }
}
