//! Typed stable-profile parameter normalization; geometry and transport stay outside.
#[derive(Debug, Clone)]
pub enum Size {
    Missing,
    Scalar(f64),
    Vector(Vec<Option<f64>>),
    Invalid,
}
#[derive(Debug, Clone, PartialEq)]
pub struct BoxPlan {
    pub dimensions: Vec<f64>,
    pub center: bool,
    pub source: &'static str,
    pub empty: bool,
    pub defaulted: bool,
}
pub fn box_plan(size: &Size, cube: bool, center: bool) -> BoxPlan {
    let axes = if cube { 3 } else { 2 };
    let mut dimensions = vec![1.; axes];
    let mut source = "default";
    let mut defaulted = false;
    match size {
        Size::Missing => {}
        Size::Scalar(value) => {
            dimensions.fill(*value);
            source = "scalar";
        }
        Size::Vector(values) if values.len() == axes && values.iter().all(Option::is_some) => {
            dimensions = values.iter().map(|v| v.unwrap()).collect();
            source = "vector";
        }
        other => {
            defaulted = true;
            if cube
                && let Size::Vector(values) = other
                && values.len() == 3
            {
                for (index, value) in values.iter().enumerate() {
                    let Some(value) = value else {
                        break;
                    };
                    dimensions[index] = *value;
                    source = "partial-vector";
                }
            }
        }
    }
    let empty = dimensions.iter().any(|v| !v.is_finite() || *v <= 0.);
    BoxPlan {
        dimensions,
        center,
        source,
        empty,
        defaulted,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefix_conversion_and_empty_values() {
        let size = Size::Vector(vec![Some(2.), None, Some(4.)]);
        let cube = box_plan(&size, true, false);
        assert_eq!(cube.dimensions, vec![2., 1., 1.]);
        assert_eq!(cube.source, "partial-vector");
        assert!(cube.defaulted);
        assert_eq!(box_plan(&size, false, true).dimensions, vec![1., 1.]);
        for value in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(box_plan(&Size::Scalar(value), true, false).empty);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RadiusPair {
    pub value: f64,
    pub source: &'static str,
    pub supplied: bool,
    pub shadowed: bool,
}
pub fn radius_pair(radius: Option<f64>, diameter: Option<f64>) -> RadiusPair {
    match (radius, diameter) {
        (r, Some(d)) => RadiusPair {
            value: d / 2.,
            source: "diameter",
            supplied: true,
            shadowed: r.is_some(),
        },
        (Some(r), None) => RadiusPair {
            value: r,
            source: "radius",
            supplied: true,
            shadowed: false,
        },
        (None, None) => RadiusPair {
            value: 1.,
            source: "default",
            supplied: false,
            shadowed: false,
        },
    }
}
pub fn radial_empty(radius: f64) -> bool {
    !radius.is_finite() || radius <= 0.
}
#[derive(Debug)]
pub struct CylinderPlan {
    pub height: f64,
    pub radius1: f64,
    pub radius2: f64,
    pub source1: &'static str,
    pub source2: &'static str,
    pub fragment_radius: f64,
    pub empty: bool,
    pub ambiguous: bool,
}
pub fn cylinder_plan(
    height: Option<f64>,
    common: RadiusPair,
    low: RadiusPair,
    high: RadiusPair,
) -> CylinderPlan {
    fn end(pair: RadiusPair, common: RadiusPair, low: bool) -> (f64, &'static str) {
        if pair.supplied {
            (
                pair.value,
                match (pair.source, low) {
                    ("diameter", true) => "diameter1",
                    ("diameter", false) => "diameter2",
                    (_, true) => "radius1",
                    (_, false) => "radius2",
                },
            )
        } else {
            (common.value, common.source)
        }
    }
    let height = height.unwrap_or(1.);
    let (radius1, source1) = end(low, common, true);
    let (radius2, source2) = end(high, common, false);
    let empty = !height.is_finite()
        || height <= 0.
        || !radius1.is_finite()
        || !radius2.is_finite()
        || radius1 < 0.
        || radius2 < 0.
        || (radius1 == 0. && radius2 == 0.);
    let fragment_radius = if radius1.is_nan() || radius2.is_nan() {
        f64::NAN
    } else if radius1 == 0. && radius2 == 0. {
        if radius1.is_sign_negative() && radius2.is_sign_negative() {
            -0.
        } else {
            0.
        }
    } else {
        radius1.max(radius2)
    };
    CylinderPlan {
        height,
        radius1,
        radius2,
        source1,
        source2,
        fragment_radius,
        empty,
        ambiguous: common.supplied && (low.supplied || high.supplied),
    }
}

#[cfg(test)]
mod radius_tests {
    use super::*;
    #[test]
    fn diameter_precedence_and_cone_empty_rules() {
        let shared = radius_pair(Some(3.), Some(8.));
        assert!(shared.shadowed);
        assert_eq!(shared.value, 4.);
        let zero = radius_pair(Some(0.), None);
        let missing = radius_pair(None, None);
        let cone = cylinder_plan(Some(2.), shared, zero, missing);
        assert!(!cone.empty);
        assert!(cone.ambiguous);
        assert_eq!(cone.radius1, 0.);
        assert_eq!(cone.radius2, 4.);
        assert_eq!(cone.source1, "radius1");
        assert_eq!(cone.source2, "diameter");
        assert!(cylinder_plan(None, missing, zero, zero).empty);
        let negative_zero = radius_pair(Some(-0.), None);
        assert!(
            !cylinder_plan(None, missing, zero, negative_zero)
                .fragment_radius
                .is_sign_negative()
        );
        assert!(
            cylinder_plan(None, missing, negative_zero, negative_zero)
                .fragment_radius
                .is_sign_negative()
        );
        assert!(
            cylinder_plan(None, radius_pair(Some(f64::NAN), None), missing, missing)
                .fragment_radius
                .is_nan()
        );
    }
}
