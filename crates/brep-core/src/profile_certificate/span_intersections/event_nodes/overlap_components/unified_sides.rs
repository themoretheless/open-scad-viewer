//! Exact side winding on common and single-source physical graph edges.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use arithmetic::branch_neighbourhood::ParameterValue;
#[derive(Clone, Debug)]
pub struct OwnedOriginalUnifiedSides {
    graph: Arc<OwnedOriginalUnifiedGraph>,
    loops: Vec<Vec<Curve>>,
    edge: usize,
    parameter: ExactRatio,
    radius: ExactRatio,
    limits: ParameterOrderLimits,
    certificates: [PointWindingCertificate; 2],
    jumps: Vec<i64>,
}
fn bound(v: &OriginalSourceParameter) -> Result<ParameterValue> {
    Ok(ParameterValue {
        primitive: decode_polynomial(&v.primitive)?,
        value: decode_polynomial(&v.parameter)?,
        interval: [decode_ratio(&v.interval[0])?, decode_ratio(&v.interval[1])?],
    })
}
fn derive(
    graph: &OwnedOriginalUnifiedGraph,
    loops: &[Vec<Curve>],
    edge: usize,
    parameter: &ExactRatio,
    radius: &ExactRatio,
    limits: ParameterOrderLimits,
) -> Result<([PointWindingCertificate; 2], Vec<i64>)> {
    if limits.max_refinements > 512 || !(1..=128).contains(&limits.max_resultant_size) {
        return Err(refused("Invalid unified side proof budget"));
    }
    graph.reverify_source()?;
    crate::profile_certificate::point_winding::validate_source_wires(loops)?;
    let owner = graph.owner();
    if loops.iter().flatten().cloned().collect::<Vec<_>>() != owner.dictionary().sources() {
        return Err(refused(
            "Unified side proof does not own the full original wire census",
        ));
    }
    let e = graph
        .edges()
        .get(edge)
        .ok_or_else(|| refused("Unknown unified side edge"))?;
    let representative = e.carriers[0];
    let c = &owner.carriers()[representative.carrier];
    let parameter = decode_ratio(parameter)?;
    for end in 0..2 {
        let b = bound(&owner.cuts()[c.source][c.cuts[end]].value)?;
        let cmp = arithmetic::parameter_order::compare(
            &b.primitive,
            &b.value,
            &b.interval,
            &vec![Q::zero(), Q::one()],
            &vec![parameter.clone()],
            &[Q::zero(), Q::zero()],
            limits.max_refinements,
            limits.max_resultant_size,
        )?;
        if cmp
            != if end == 0 {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        {
            return Err(refused(
                "Unified side parameter is outside its original atomic carrier",
            ));
        }
    }
    let mut nets = Vec::new();
    let mut source_cells = Vec::new();
    let mut offsets = Vec::new();
    for curve in owner.dictionary().sources() {
        let cs = cells(curve)?;
        offsets.push(nets.len());
        nets.extend(cs.iter().map(|c| c.net.clone()));
        source_cells.push(cs);
    }
    let mut centers = Vec::new();
    let mut jumps = vec![0i64; loops.len()];
    let wires = loops
        .iter()
        .enumerate()
        .flat_map(|(wire, curves)| curves.iter().map(move |_| wire))
        .collect::<Vec<_>>();
    for use_ in &e.carriers {
        let c = &owner.carriers()[use_.carrier];
        let cell = &source_cells[c.source][c.cell];
        let mut bounds = [
            bound(&owner.cuts()[c.source][c.cuts[0]].value)?,
            bound(&owner.cuts()[c.source][c.cuts[1]].value)?,
        ];
        for b in &mut bounds {
            if b.value.is_empty() {
                b.value.push(-cell.domain[0].clone());
            } else {
                b.value[0] -= &cell.domain[0];
            }
            for v in &mut b.value {
                *v /= &cell.domain[1] - &cell.domain[0];
            }
        }
        centers.push(arithmetic::carrier_sides::CenterInterval {
            cell: offsets[c.source] + c.cell,
            bounds,
        });
        let wire = wires[c.source];
        jumps[wire] = jumps[wire]
            .checked_add(if use_.reversed { -1 } else { 1 })
            .ok_or_else(|| refused("Unified original winding jump overflow"))?;
    }
    let cell = &source_cells[c.source][c.cell];
    let local = (&parameter - &cell.domain[0]) / (&cell.domain[1] - &cell.domain[0]);
    let mut queries = arithmetic::carrier_sides::probes_with_centers(
        &nets,
        offsets[c.source] + c.cell,
        &local,
        &decode_ratio(radius)?,
        limits.max_refinements,
        &centers,
        limits.max_resultant_size,
    )?;
    if representative.reversed {
        queries.swap(0, 1);
    }
    let certificates = [
        certify_rational_point_winding(loops, queries[0].clone().map(|q| ratio(&q)))?,
        certify_rational_point_winding(loops, queries[1].clone().map(|q| ratio(&q)))?,
    ];
    for wire in 0..loops.len() {
        if certificates[0].windings[wire].checked_sub(certificates[1].windings[wire])
            != Some(jumps[wire])
        {
            return Err(refused(
                "Unified original side winding disagrees with complete source contributions",
            ));
        }
    }
    Ok((certificates, jumps))
}
impl OwnedOriginalUnifiedSides {
    pub fn graph(&self) -> &Arc<OwnedOriginalUnifiedGraph> {
        &self.graph
    }
    pub fn loops(&self) -> &[Vec<Curve>] {
        &self.loops
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    /// Left/right relative to the common graph edge's canonical orientation.
    pub fn certificates(&self) -> &[PointWindingCertificate; 2] {
        &self.certificates
    }
    pub fn jumps(&self) -> &[i64] {
        &self.jumps
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (c, j) = derive(
            &self.graph,
            &self.loops,
            self.edge,
            &self.parameter,
            &self.radius,
            self.limits,
        )?;
        if c != self.certificates || j != self.jumps {
            return Err(refused(
                "Unified original side winding source or jump changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_unified_sides(
    graph: Arc<OwnedOriginalUnifiedGraph>,
    loops: &[Vec<Curve>],
    edge: usize,
    parameter: ExactRatio,
    radius: ExactRatio,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalUnifiedSides> {
    let (certificates, jumps) = derive(&graph, loops, edge, &parameter, &radius, limits)?;
    Ok(OwnedOriginalUnifiedSides {
        graph,
        loops: loops.to_vec(),
        edge,
        parameter,
        radius,
        limits,
        certificates,
        jumps,
    })
}
pub fn own_original_unified_sides_automatically(
    graph: Arc<OwnedOriginalUnifiedGraph>,
    loops: &[Vec<Curve>],
    limits: ParameterOrderLimits,
    radius_halvings: usize,
) -> Result<Vec<OwnedOriginalUnifiedSides>> {
    graph.reverify_source()?;
    crate::profile_certificate::point_winding::validate_source_wires(loops)?;
    if radius_halvings > 512
        || limits.max_refinements > 512
        || !(1..=128).contains(&limits.max_resultant_size)
        || loops.iter().flatten().cloned().collect::<Vec<_>>()
            != graph.owner().dictionary().sources()
    {
        return Err(refused(
            "Invalid automatic unified winding census or budgets",
        ));
    }
    let mut out = Vec::new();
    for (edge, e) in graph.edges().iter().enumerate() {
        let c = &graph.owner().carriers()[e.carriers[0].carrier];
        let actual = cells(&graph.owner().dictionary().sources()[c.source])?;
        let cell = &actual[c.cell];
        let parameter = arithmetic::carrier_sides::interior(
            &[(cell.net.clone(), cell.domain.clone())],
            [
                bound(&graph.owner().cuts()[c.source][c.cuts[0]].value)?,
                bound(&graph.owner().cuts()[c.source][c.cuts[1]].value)?,
            ],
            limits.max_refinements,
        )?;
        let mut radius = Q::one();
        let mut proved = None;
        for step in 0..=radius_halvings {
            if let Ok(side) = own_original_unified_sides(
                graph.clone(),
                loops,
                edge,
                ratio(&parameter),
                ratio(&radius),
                limits,
            ) {
                proved = Some(side);
                break;
            }
            if step < radius_halvings {
                radius /= Q::from_integer(2.into());
            }
        }
        out.push(proved.ok_or_else(|| {
            refused("Unified original side proof remains unresolved within radius budget")
        })?);
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_side_winding_jump_and_radius_tampering() {
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
        let graph = Arc::new(own_original_unified_graph(c).unwrap());
        let sides = own_original_unified_sides_automatically(
            graph,
            &loops,
            ParameterOrderLimits::default(),
            32,
        )
        .unwrap();
        let x = &sides[0];
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.jumps[0] += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.certificates[0].windings[0] += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.radius = ["99".into(), "1".into()];
        assert!(bad.reverify_source().is_err());
    }
}
