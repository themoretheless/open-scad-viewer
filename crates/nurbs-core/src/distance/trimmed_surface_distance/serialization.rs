//! Host encoding of native domain and distance reports.
use super::*;
use value_codec::{Value, json};

impl TrimmedDistance {
    pub fn to_value(&self) -> Value {
        json!({"method":"interval-trimmed-surface-subdivision","scope":"trimmed-surfaces-bounded-joins","distanceIntervalMm":[self.lower_bound_mm,self.upper_bound_mm],"parameters":self.parameters,"points":self.points,"pointEnclosures":self.point_enclosures,"converged":self.converged,"reason":self.reason,"cells":self.cells,"maxCells":self.max_cells,"domainCells":self.domain_cells,"maxDomainCells":self.max_domain_cells,"toleranceMm":self.tolerance_mm})
    }
}
impl value_codec::Serialize for TrimmedDistance {
    fn to_value(&self) -> Value {
        TrimmedDistance::to_value(self)
    }
}
