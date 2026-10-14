//! Host encoding of native domain and distance reports.
use super::*;
use value_codec::{Value, json};

impl SurfaceDistance {
    pub fn to_value(&self) -> Value {
        json!({"method":"interval-tensor-de-boor-pair-subdivision","scope":"untrimmed-surfaces","distanceIntervalMm":self.distance_interval_mm,"parameters":self.parameters,"points":self.points,"pointEnclosures":self.point_enclosures,"converged":self.converged,"reason":self.reason,"cells":self.cells,"maxCells":self.max_cells,"toleranceMm":self.tolerance_mm})
    }
}
impl value_codec::Serialize for SurfaceDistance {
    fn to_value(&self) -> Value {
        SurfaceDistance::to_value(self)
    }
}
