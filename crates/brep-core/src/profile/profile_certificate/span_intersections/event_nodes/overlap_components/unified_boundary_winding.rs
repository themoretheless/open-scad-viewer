//! Exact winding of selected unified boundaries, counting each common edge once.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use arithmetic::branch_neighbourhood::ParameterValue;
#[derive(Clone, Debug)]
pub struct OwnedOriginalUnifiedBoundaryWinding {
    owner: Arc<OwnedOriginalUnifiedFilled>,
    point: [ExactRatio; 2],
    limits: ParameterOrderLimits,
    windings: Vec<i64>,
}
pub(super) fn evaluate_verified(
    owner: &OwnedOriginalUnifiedFilled,
    point: &[ExactRatio; 2],
    limits: ParameterOrderLimits,
) -> Result<Vec<i64>> {
    if limits.max_refinements > 512 || !(1..=128).contains(&limits.max_resultant_size) {
        return Err(refused("Invalid unified boundary winding limits"));
    }
    let point = [decode_ratio(&point[0])?, decode_ratio(&point[1])?];
    let ports = owner.cycles().owner().owner();
    let graph = ports.graph();
    let carriers = graph.owner();
    let original = carriers
        .dictionary()
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let mut cache = vec![None; graph.edges().len()];
    let mut windings = Vec::new();
    for boundary in owner.boundaries() {
        let mut total = 0i64;
        for &dart in boundary {
            let port = &ports.ports()[dart];
            if cache[port.edge].is_none() {
                let use_ = graph.edges()[port.edge].carriers[0];
                let c = &carriers.carriers()[use_.carrier];
                let decode = |cut: usize| -> Result<ParameterValue> {
                    let v = &carriers.cuts()[c.source][cut].value;
                    Ok(ParameterValue {
                        primitive: decode_polynomial(&v.primitive)?,
                        value: decode_polynomial(&v.parameter)?,
                        interval: [decode_ratio(&v.interval[0])?, decode_ratio(&v.interval[1])?],
                    })
                };
                let bounds = [decode(c.cuts[0])?, decode(c.cuts[1])?];
                let cell = &original[c.source][c.cell];
                let value = arithmetic::selected_winding::contribution(
                    &cell.net,
                    &cell.domain,
                    &bounds,
                    &point,
                    limits.max_refinements,
                    limits.max_resultant_size,
                )?;
                cache[port.edge] = Some(if use_.reversed {
                    value
                        .checked_neg()
                        .ok_or_else(|| refused("Unified boundary orientation overflow"))?
                } else {
                    value
                });
            }
            let value = cache[port.edge].unwrap();
            let signed = if port.endpoint == 0 {
                value
            } else {
                value
                    .checked_neg()
                    .ok_or_else(|| refused("Unified boundary reverse contribution overflow"))?
            };
            total = total
                .checked_add(signed)
                .ok_or_else(|| refused("Unified boundary winding sum overflow"))?;
        }
        windings.push(total);
    }
    Ok(windings)
}
impl OwnedOriginalUnifiedBoundaryWinding {
    pub fn owner(&self) -> &Arc<OwnedOriginalUnifiedFilled> {
        &self.owner
    }
    pub fn point(&self) -> &[ExactRatio; 2] {
        &self.point
    }
    pub fn windings(&self) -> &[i64] {
        &self.windings
    }
    pub fn reverify_source(&self) -> Result<()> {
        self.owner.reverify_source()?;
        if self.windings != evaluate_verified(&self.owner, &self.point, self.limits)? {
            return Err(refused("Unified selected boundary winding changed"));
        }
        Ok(())
    }
}
pub fn own_original_unified_boundary_winding(
    owner: Arc<OwnedOriginalUnifiedFilled>,
    point: [ExactRatio; 2],
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalUnifiedBoundaryWinding> {
    owner.reverify_source()?;
    let windings = evaluate_verified(&owner, &point, limits)?;
    Ok(OwnedOriginalUnifiedBoundaryWinding {
        owner,
        point,
        limits,
        windings,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_selected_winding_value_and_query_rebinding() {
        let a = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ])
        .unwrap();
        let loops = vec![vec![a.clone()]];
        let d = Arc::new(
            own_original_common_dictionary(&[a], 1e-12, ParameterOrderLimits::default()).unwrap(),
        );
        let c = Arc::new(own_original_source_carriers(d, ParameterOrderLimits::default()).unwrap());
        let g = Arc::new(own_original_unified_graph(c).unwrap());
        let p = Arc::new(own_original_unified_ports(g.clone(), 192).unwrap());
        let r =
            Arc::new(own_original_unified_rotation(p, ContactPortOrderLimits::default()).unwrap());
        let cycles = Arc::new(own_original_unified_cycles(r).unwrap());
        let sides = own_original_unified_sides_automatically(
            g,
            &loops,
            ParameterOrderLimits::default(),
            32,
        )
        .unwrap();
        let f = Arc::new(own_original_unified_filled(cycles, sides, &[1]).unwrap());
        let q = |n: &str, d: &str| [n.to_string(), d.to_string()];
        let x = own_original_unified_boundary_winding(
            f,
            [q("1", "2"), q("1", "2")],
            ParameterOrderLimits::default(),
        )
        .unwrap();
        x.reverify_source().unwrap();
        assert_eq!(x.windings(), &[1]);
        let mut bad = x.clone();
        bad.windings[0] = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.point[0] = q("2", "1");
        assert!(bad.reverify_source().is_err());
    }
}
