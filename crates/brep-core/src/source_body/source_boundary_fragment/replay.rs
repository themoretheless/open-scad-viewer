//! Native counterpart of original-definition fragment restoration.
use super::*;
impl Fragment {
    /// Every crossing and carrier is freshly admitted. Mapping work is shared
    /// over both ends, with disjoint reserved carrier allowances as in restore.
    pub fn requalify(&self, max_mapping_cells: usize) -> Result<Self> {
        if !(1..=100000).contains(&max_mapping_cells) {
            return Err(error("Unsupported fragment definition or mapping budget"));
        }
        let mut remaining = max_mapping_cells;
        let mut carrier_left = self
            .endpoints
            .iter()
            .filter(|e| matches!(e, Endpoint::Carrier { .. }))
            .count();
        let carrier_count = carrier_left.max(1);
        let mut endpoints = Vec::with_capacity(2);
        for endpoint in &self.endpoints {
            let endpoint = match endpoint {
                Endpoint::Parameter(t) => Endpoint::Parameter(*t),
                Endpoint::Crossing { point, role } => {
                    if remaining == 0 {
                        return Err(error("Source endpoint mapping work exhausted"));
                    }
                    let report = point.requalify(remaining)?;
                    remaining -= report.mapping_cells;
                    let point = report
                        .point
                        .ok_or_else(|| error("Imported crossing endpoint is unqualified"))?;
                    Endpoint::Crossing { point, role: *role }
                }
                Endpoint::Carrier { point, side } => {
                    let allowance = remaining / carrier_left;
                    if allowance == 0 {
                        return Err(error("Carrier restoration work exhausted"));
                    }
                    if *side > 1 {
                        return Err(error("Require an incident carrier side"));
                    }
                    let report = point.requalify(crate::source_carrier_point::RestoreLimits {
                        edge: crate::source_shared_edge::ReplayLimits {
                            mapping_cells_per_use: allowance,
                            exact_work: 100_000_000 / carrier_count as u64,
                            driver_cells: 100000 / carrier_count,
                        },
                        root_mapping_cells: allowance,
                        spans: 100000 / carrier_count,
                    })?;
                    remaining -= allowance;
                    carrier_left -= 1;
                    let point = report
                        .point
                        .ok_or_else(|| error("Imported carrier endpoint is unqualified"))?;
                    Endpoint::Carrier {
                        point: Box::new(point),
                        side: *side,
                    }
                }
            };
            endpoints.push(endpoint);
        }
        Self::new(
            &self.surface,
            &self.curve,
            endpoints.remove(0),
            endpoints.remove(0),
        )
    }
}
