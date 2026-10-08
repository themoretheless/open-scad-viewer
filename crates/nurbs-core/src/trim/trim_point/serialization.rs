use super::*;
use value_codec::{Serialize, Value, json};

impl Serialize for PointTrim {
    fn to_value(&self) -> Value {
        let mut result = json!({"curve": self.curve,"parameter":self.parameter,
            "parameterInterval":self.parameter_interval,"point":self.point,
            "keptDomain":self.kept_domain,"projection":self.projection});
        match self.projection_space {
            ProjectionSpace::WorldMillimeters => {
                result["distanceUpperMm"] = self.distance_upper.to_value()
            }
            ProjectionSpace::CssPixels => {
                result["distanceUpperPx"] = self.distance_upper.to_value();
                result["projectionSpace"] = "css-pixels".to_value();
            }
        }
        result
    }
}

/// Compatibility adapter for the existing host response.
pub fn trim_at_point(curve: &Curve, point: &[f64], keep: &str, max_distance: f64) -> Result<Value> {
    Ok(trim_at_point_report(curve, point, keep, max_distance)?.to_value())
}

pub fn trim_at_screen_point(
    curve: &Curve,
    point: &[f64],
    matrix: &[[f64; 4]; 2],
    keep: &str,
    radius: f64,
) -> Result<Value> {
    Ok(trim_at_screen_point_report(curve, point, matrix, keep, radius)?.to_value())
}
