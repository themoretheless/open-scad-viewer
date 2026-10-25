//! Profile-dependent extrusion subdivision shared by language planning and mesh execution.
//! Inputs are resolved language parameters; the caller owns engine limits and warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Single,
    TwistMinimum,
    TwistFn,
    TwistFragments,
    NonuniformFn,
    NonuniformFs,
}
impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::TwistMinimum => "twist-minimum",
            Self::TwistFn => "twist-$fn",
            Self::TwistFragments => "twist-$fa/$fs",
            Self::NonuniformFn => "nonuniform-$fn",
            Self::NonuniformFs => "nonuniform-$fs",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub slices: f64,
    pub source: Source,
}
fn maximum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}
fn minimum_value(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}
const EPSILON: f64 = 0.00000095367431640625;
fn spiral_length(radius: f64, radians: f64, scale: f64) -> f64 {
    let slope = radius * (scale - 1.) / radians;
    let primitive = |r: f64| 0.5 * (r * r.hypot(slope) + slope * slope * (r / slope.abs()).asinh());
    ((primitive(radius * scale) - primitive(radius)) / slope).abs()
}
/// Select subdivision before applying the host's maximum slice budget.
pub fn automatic(
    points: &[[f64; 2]],
    height: f64,
    twist: f64,
    scale: [f64; 2],
    fragments: [f64; 3],
) -> Selection {
    let [count, angle, size] = fragments;
    let (slices, source) = if twist != 0. {
        let absolute = twist.abs();
        let minimum = (absolute / 120.).ceil().max(1.);
        let radius = points
            .iter()
            .map(|[x, y]| x * x + y * y)
            .fold(0_f64, maximum)
            .sqrt();
        if radius < EPSILON || !count.is_finite() {
            (minimum, Source::TwistMinimum)
        } else if count > 0. {
            (
                (absolute * count / 360.).ceil().max(minimum),
                Source::TwistFn,
            )
        } else {
            let radians = absolute * std::f64::consts::PI / 180.;
            let planar = if scale[0] == scale[1] && scale[0] != 1. {
                spiral_length(radius, radians, scale[0])
            } else {
                radius * radians
            };
            (
                maximum(
                    minimum,
                    minimum_value(
                        (absolute / angle).ceil(),
                        (planar.hypot(height) / size).ceil(),
                    ),
                ),
                Source::TwistFragments,
            )
        }
    } else if scale[0] != scale[1] {
        let displacement = points
            .iter()
            .map(|[x, y]| {
                let dx = x * (1. - scale[0]);
                let dy = y * (1. - scale[1]);
                dx * dx + dy * dy
            })
            .fold(0_f64, maximum)
            .sqrt();
        if displacement < EPSILON || !count.is_finite() {
            (1., Source::Single)
        } else if count > 0. {
            (count.trunc().max(1.), Source::NonuniformFn)
        } else {
            (
                (displacement.hypot(height) / size).ceil().max(1.),
                Source::NonuniformFs,
            )
        }
    } else {
        (1., Source::Single)
    };
    Selection {
        slices: if slices.is_finite() && slices >= 1. {
            slices
        } else {
            1.
        },
        source,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_families_and_zero_radius() {
        let p = [[4., 0.]];
        assert_eq!(
            automatic(&p, 10., 360., [1., 1.], [12., 12., 2.]).slices,
            12.
        );
        assert_eq!(automatic(&p, 10., 0., [2., 1.], [5.9, 12., 2.]).slices, 5.);
        assert_eq!(
            automatic(&[], 10., 360., [1., 1.], [12., 12., 2.]).source,
            Source::TwistMinimum
        );
        assert_eq!(
            automatic(&p, 10., 0., [2., 1.], [f64::INFINITY, 12., 2.]).slices,
            1.
        );
    }
}

#[derive(Debug, PartialEq)]
pub struct Resolved {
    pub slices: usize,
    pub unbounded_slices: f64,
    /// None means the authored slice count was used.
    pub automatic_source: Option<Source>,
    pub reduced: bool,
}
/// Select interval count after profile evaluation, then apply the engine limit.
/// Mesh adapters use `slices - 1` for their interior subdivision count.
pub fn resolve(
    points: &[[f64; 2]],
    height: f64,
    twist: f64,
    scale: [f64; 2],
    fragments: [f64; 3],
    explicit: Option<f64>,
    maximum: usize,
) -> crate::Result<Resolved> {
    if maximum == 0 || maximum as u128 > 9_007_199_254_740_991 {
        return Err(crate::fail(
            "Extrusion slice maximum must be a positive safe integer",
        ));
    }
    let explicit = explicit
        .filter(|v| v.is_finite())
        .map(f64::trunc)
        .filter(|v| *v > 0.);
    let (unbounded_slices, automatic_source) = if let Some(value) = explicit {
        (value, None)
    } else {
        let selection = automatic(points, height, twist, scale, fragments);
        (selection.slices, Some(selection.source))
    };
    let selected = unbounded_slices.min(maximum as f64);
    Ok(Resolved {
        slices: selected as usize,
        unbounded_slices,
        automatic_source,
        reduced: selected != unbounded_slices,
    })
}
#[cfg(test)]
mod resolved_tests {
    use super::*;
    #[test]
    fn interval_selection_clamps_after_automatic_and_explicit_rules() {
        let points = [[4., 0.]];
        let auto = resolve(&points, 10., 360., [1.; 2], [12., 12., 2.], None, 8).unwrap();
        assert_eq!(auto.slices, 8);
        assert_eq!(auto.unbounded_slices, 12.);
        assert!(auto.reduced);
        assert_eq!(auto.automatic_source, Some(Source::TwistFn));
        let explicit = resolve(&points, 10., 0., [1.; 2], [0., 12., 2.], Some(3.9), 512).unwrap();
        assert_eq!(explicit.slices, 3);
        assert_eq!(explicit.automatic_source, None);
        assert!(!explicit.reduced);
        assert!(resolve(&points, 10., 0., [1.; 2], [0., 12., 2.], None, 0).is_err());
    }
}

/// Deferred interval selection after a profile's geometry has been evaluated.
#[derive(Debug, Clone, PartialEq)]
pub struct Policy {
    pub explicit: Option<f64>,
    pub fragments: [f64; 3],
    pub maximum: usize,
}
impl Policy {
    pub fn resolve(
        &self,
        points: &[[f64; 2]],
        height: f64,
        twist: f64,
        scale: [f64; 2],
    ) -> crate::Result<Resolved> {
        resolve(
            points,
            height,
            twist,
            scale,
            self.fragments,
            self.explicit,
            self.maximum,
        )
    }
}
#[cfg(feature = "codec")]
mod policy_codec {
    use super::*;
    use crate::codec::{decode_number as decode, encode_number as encode};
    use value_codec::{Deserialize, Result, Serialize, Value, json};
    impl Serialize for Policy {
        fn to_value(&self) -> Value {
            json!({"explicit":self.explicit,"fragments":self.fragments.map(encode),"maximum":self.maximum})
        }
    }
    impl<'de> Deserialize<'de> for Policy {
        fn from_value(value: Value) -> Result<Self> {
            let mut object = crate::codec::object(value)?;
            let fragments: Vec<Value> = crate::codec::required(&mut object, "fragments")?;
            if fragments.len() != 3 {
                return Err(value_codec::error(
                    "Expected three extrusion fragment values",
                ));
            }
            Ok(Self {
                explicit: crate::codec::required(&mut object, "explicit")?,
                fragments: [
                    decode(&fragments[0])?,
                    decode(&fragments[1])?,
                    decode(&fragments[2])?,
                ],
                maximum: crate::codec::required(&mut object, "maximum")?,
            })
        }
    }
}
