//! Host encoding of native domain and distance reports.
use super::*;
use value_codec::{Value, json};

impl CurveDistance {
    pub fn to_value(&self) -> Value {
        json!({"method":"interval-de-boor-pair-subdivision","distanceIntervalMm":self.distance_interval_mm,"parameters":self.parameters,"points":self.points,"pointEnclosures":self.point_enclosures,"converged":self.converged,"reason":self.reason,"cells":self.cells,"maxCells":self.max_cells,"toleranceMm":self.tolerance_mm})
    }
}
impl value_codec::Serialize for CurveDistance {
    fn to_value(&self) -> Value {
        CurveDistance::to_value(self)
    }
}
