//! Physical planar end caps of an original-source finite worm-wheel network.
use crate::{Error, Result, planar_cap};
use nurbs_core::worm_wheel::AxialSideNetwork;
mod body;
pub use body::{WheelBodyCandidate, wheel_body_candidate};

#[derive(Clone, Copy, Debug)]
pub struct WheelCapLimits {
    pub trim_pairs: usize,
    pub trim_cells: usize,
    pub domain_cells: usize,
    pub agreement_cells: usize,
}
impl Default for WheelCapLimits {
    fn default() -> Self {
        Self {
            trim_pairs: 100000,
            trim_cells: 100000,
            domain_cells: 1000000,
            agreement_cells: 100000,
        }
    }
}
impl WheelCapLimits {
    fn validate(self) -> Result<()> {
        if !(1..=100000).contains(&self.trim_pairs)
            || !(1..=100000).contains(&self.trim_cells)
            || !(1..=1000000).contains(&self.domain_cells)
            || !(1..=100000).contains(&self.agreement_cells)
        {
            return Err(Error::new(
                "BREP_WHEEL_CAP_LIMITS",
                "Invalid wheel cap proof budgets",
            ));
        }
        Ok(())
    }
}
/// Explicit physical dimensions and resolution. Defaults match the module-1,
/// forty-tooth default source. Other specifications need compatible dimensions.
pub struct WheelBuildOptions {
    pub y_breaks: Vec<f64>,
    pub family_breaks: [Vec<f64>; 6],
    pub stock_radius: f64,
    pub tolerance: f64,
    pub max_patches_per_family: usize,
    pub caps: WheelCapLimits,
}
impl Default for WheelBuildOptions {
    fn default() -> Self {
        Self {
            y_breaks: (0..=16).map(|i| -2. + i as f64 / 4.).collect(),
            family_breaks: std::array::from_fn(|i| {
                if i == 1 || i == 3 {
                    vec![0., 0.5, 1.]
                } else {
                    vec![0., 1.]
                }
            }),
            stock_radius: 21.,
            tolerance: 1e-4,
            max_patches_per_family: 128,
            caps: WheelCapLimits::default(),
        }
    }
}
/// Complete finite ZA-generated wheel candidate with original side-source
/// error, two admitted physical planar ends and saved-edge closure. This is
/// separate from global geometric solid qualification.
pub fn finite_wheel_with_certificates(
    spec: nurbs_core::worm_wheel::Spec,
    options: WheelBuildOptions,
) -> Result<WheelBodyCandidate> {
    options.caps.validate()?;
    let sides = nurbs_core::worm_wheel::axial_side_network(
        spec,
        options.y_breaks,
        options.stock_radius,
        options.family_breaks,
        options.tolerance,
        options.max_patches_per_family,
    )?;
    wheel_body_candidate(wheel_caps(sides, options.tolerance, options.caps)?)
}

/// Admitted physical end faces, in the proper frame (x,-z,y). Both supports
/// point along +Z; a future closed shell must reverse the lower face use.
/// This owner does not claim a sewn shell or geometric solid.
pub struct WheelCaps {
    sides: AxialSideNetwork,
    caps: [planar_cap::Cap; 2],
    trim: [nurbs_core::trim_region_audit::Report; 2],
    agreement_cells: usize,
    tolerance: f64,
}
impl WheelCaps {
    pub fn sides(&self) -> &AxialSideNetwork {
        &self.sides
    }
    pub fn caps(&self) -> &[planar_cap::Cap; 2] {
        &self.caps
    }
    pub fn trims(&self) -> &[nurbs_core::trim_region_audit::Report; 2] {
        &self.trim
    }
    pub fn agreement_cells(&self) -> usize {
        self.agreement_cells
    }
}

/// Uses exact saved end rows; does not refit a contour, change a plane, or
/// accept sampled winding as a trim proof. Unresolved caps are refused.
pub fn wheel_caps(
    sides: AxialSideNetwork,
    tolerance: f64,
    limits: WheelCapLimits,
) -> Result<WheelCaps> {
    limits.validate()?;
    if !tolerance.is_finite() || tolerance <= 0. {
        return Err(Error::new(
            "BREP_WHEEL_CAP_LIMITS",
            "Wheel cap tolerance must be finite and positive",
        ));
    }
    let mut contours = sides.end_contours();
    let mut caps = Vec::new();
    let mut trims = Vec::new();
    let mut agreement_cells = 0;
    for contour in &mut contours {
        for curve in contour.iter_mut() {
            for p in &mut curve.control_points {
                let z = p[2];
                p[2] = p[1];
                p[1] = -z;
            }
        }
        let plane = contour[0].control_points[0][2];
        let report = planar_cap::build_general(
            std::slice::from_ref(contour),
            plane,
            tolerance,
            limits.trim_pairs,
            limits.trim_cells,
            limits.domain_cells,
            limits.agreement_cells,
        )?;
        let cap = report.cap.ok_or_else(|| {
            let unresolved = report
                .trim
                .loops
                .iter()
                .flat_map(|l| l.pairs.iter())
                .find(|p| !p.proven)
                .map(|p| p.curves);
            Error::new(
                "BREP_WHEEL_CAP_UNPROVEN",
                format!(
                    "{}; pairs={}, cells={}, domain={}, first unresolved pair={:?}",
                    report.trim.reason,
                    report.trim.pairs,
                    report.trim.cells,
                    report.trim.domain_cells,
                    unresolved
                ),
            )
        })?;
        agreement_cells += report.agreement_cells;
        caps.push(cap);
        trims.push(report.trim);
    }
    Ok(WheelCaps {
        sides,
        caps: caps.try_into().ok().unwrap(),
        trim: trims.try_into().ok().unwrap(),
        agreement_cells,
        tolerance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_wheel_refuses_incomplete_domains_invalid_geometry_and_proof_limits() {
        use nurbs_core::worm_wheel::Spec;
        for options in [
            WheelBuildOptions {
                tolerance: f64::NAN,
                ..Default::default()
            },
            WheelBuildOptions {
                stock_radius: f64::NAN,
                ..Default::default()
            },
            WheelBuildOptions {
                y_breaks: vec![0., 0.],
                ..Default::default()
            },
            WheelBuildOptions {
                family_breaks: std::array::from_fn(|_| vec![0., 0.5]),
                ..Default::default()
            },
            WheelBuildOptions {
                caps: WheelCapLimits {
                    trim_cells: 0,
                    ..Default::default()
                },
                ..Default::default()
            },
        ] {
            assert!(finite_wheel_with_certificates(Spec::default(), options).is_err());
        }
        assert!(
            finite_wheel_with_certificates(
                Spec {
                    teeth: 11,
                    ..Default::default()
                },
                Default::default()
            )
            .is_err()
        );
    }
    #[test]
    fn wheel_cap_work_is_bounded() {
        WheelCapLimits::default().validate().unwrap();
        for invalid in [
            WheelCapLimits {
                trim_pairs: 0,
                ..Default::default()
            },
            WheelCapLimits {
                trim_cells: 100001,
                ..Default::default()
            },
            WheelCapLimits {
                domain_cells: 1000001,
                ..Default::default()
            },
            WheelCapLimits {
                agreement_cells: 0,
                ..Default::default()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }
}
