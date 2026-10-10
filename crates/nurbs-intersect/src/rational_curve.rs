use nurbs_core::{Result, curve::Curve};

/// Bit-exact authority for a rational NURBS curve.  This deliberately records
/// the complete definition, rather than endpoint or sample-point surrogates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalCurveDefinition {
    pub degree: usize,
    pub knots: Vec<u64>,
    pub control_points: Vec<Vec<u64>>,
    pub weights: Vec<u64>,
    pub periodic: bool,
}

impl RationalCurveDefinition {
    pub fn from_curve(curve: &Curve) -> Result<Self> {
        curve.validate()?;
        Ok(Self {
            degree: curve.degree,
            knots: curve.knots.iter().map(|v| v.to_bits()).collect(),
            control_points: curve
                .control_points
                .iter()
                .map(|point| point.iter().map(|v| v.to_bits()).collect())
                .collect(),
            weights: curve.weights.iter().map(|v| v.to_bits()).collect(),
            periodic: curve.periodic,
        })
    }
}
