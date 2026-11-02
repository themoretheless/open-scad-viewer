//! Stable offset parameter precedence. Geometry validation belongs to the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Radius,
    Delta,
}
pub use geometry_ops::profile_program::OffsetJoin as Join;
#[derive(Debug, PartialEq)]
pub struct Plan {
    pub mode: Mode,
    pub distance: f64,
    pub join: Join,
    pub chamfer: bool,
}
pub fn resolve(radius: Option<f64>, delta: Option<f64>, chamfer: bool) -> Plan {
    if let Some(distance) = radius {
        Plan {
            mode: Mode::Radius,
            distance,
            join: Join::Round,
            chamfer: false,
        }
    } else if let Some(distance) = delta {
        Plan {
            mode: Mode::Delta,
            distance,
            join: if chamfer { Join::Square } else { Join::Miter },
            chamfer,
        }
    } else {
        Plan {
            mode: Mode::Radius,
            distance: 1.,
            join: Join::Round,
            chamfer: false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn precedence_defaults_and_nonfinite_parameters() {
        assert_eq!(
            resolve(None, None, true),
            Plan {
                mode: Mode::Radius,
                distance: 1.,
                join: Join::Round,
                chamfer: false
            }
        );
        assert_eq!(resolve(Some(0.), Some(2.), true).join, Join::Round);
        assert_eq!(resolve(None, Some(-2.), false).join, Join::Miter);
        assert_eq!(resolve(None, Some(-2.), true).join, Join::Square);
        assert!(resolve(Some(f64::NAN), Some(2.), true).distance.is_nan());
        assert_eq!(
            resolve(None, Some(f64::INFINITY), false).distance,
            f64::INFINITY
        );
    }
}
