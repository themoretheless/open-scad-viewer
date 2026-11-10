//! One-sided original source germs on deduplicated unified graph branches.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalUnifiedPort {
    pub edge: usize,
    pub endpoint: usize,
    pub vertex: usize,
    pub carrier: usize,
    pub source: usize,
    pub cell: usize,
    pub cut: usize,
    pub direction: i8,
    pub ray: LeadingBranchRay,
    /// Exact one-sided displacement signs, including beyond-leading terms.
    pub component_signs: [i8; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalUnifiedPorts {
    graph: Arc<OwnedOriginalUnifiedGraph>,
    max_refinements: usize,
    ports: Vec<OriginalUnifiedPort>,
    vertices: Vec<Vec<usize>>,
}
fn derive(
    graph: &OwnedOriginalUnifiedGraph,
    max_refinements: usize,
) -> Result<(Vec<OriginalUnifiedPort>, Vec<Vec<usize>>)> {
    if max_refinements > 512 {
        return Err(refused("Invalid unified original port refinement budget"));
    }
    graph.reverify_source()?;
    let owner = graph.owner();
    let source_cells = owner
        .dictionary()
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let mut ports = Vec::new();
    let mut vertices = vec![Vec::new(); graph.vertices().len()];
    for (edge, e) in graph.edges().iter().enumerate() {
        let representative = e
            .carriers
            .first()
            .ok_or_else(|| refused("Unified graph edge has no original branch"))?;
        let c = &owner.carriers()[representative.carrier];
        for endpoint in 0..2 {
            let source_end = endpoint ^ usize::from(representative.reversed);
            let cut = c.cuts[source_end];
            let direction = if source_end == 0 { 1 } else { -1 };
            let value = &owner.cuts()[c.source][cut].value;
            let f = decode_polynomial(&value.primitive)?;
            let parameter = decode_polynomial(&value.parameter)?;
            let root = [
                decode_ratio(&value.interval[0])?,
                decode_ratio(&value.interval[1])?,
            ];
            let cell = &source_cells[c.source][c.cell];
            let (order, numerator, signs) = arithmetic::branch_germ::leading(
                &cell.net,
                &cell.domain,
                &parameter,
                &f,
                &root,
                direction,
                max_refinements,
            )?;
            let component_signs = [
                arithmetic::branch_germ::component_sign(
                    &cell.net,
                    &cell.domain,
                    &parameter,
                    &f,
                    &root,
                    direction,
                    0,
                    max_refinements,
                )?,
                arithmetic::branch_germ::component_sign(
                    &cell.net,
                    &cell.domain,
                    &parameter,
                    &f,
                    &root,
                    direction,
                    1,
                    max_refinements,
                )?,
            ];
            let port = OriginalUnifiedPort {
                edge,
                endpoint,
                vertex: e.vertices[endpoint],
                carrier: representative.carrier,
                source: c.source,
                cell: c.cell,
                cut,
                direction,
                ray: LeadingBranchRay {
                    primitive: value.primitive.clone(),
                    isolating_interval: value.interval.clone(),
                    numerator: numerator.map(|p| polynomial(&p)),
                    signs,
                    order,
                },
                component_signs,
            };
            vertices[port.vertex].push(ports.len());
            ports.push(port);
        }
    }
    Ok((ports, vertices))
}
impl OwnedOriginalUnifiedPorts {
    pub fn graph(&self) -> &Arc<OwnedOriginalUnifiedGraph> {
        &self.graph
    }
    /// Exactly two ports per physical edge, including loop edges. Common source
    /// aliases are already deduplicated by the full common-interval proofs.
    pub fn ports(&self) -> &[OriginalUnifiedPort] {
        &self.ports
    }
    pub fn vertices(&self) -> &[Vec<usize>] {
        &self.vertices
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (ports, vertices) = derive(&self.graph, self.max_refinements)?;
        if ports != self.ports || vertices != self.vertices {
            return Err(refused(
                "Unified original branch germ or vertex port membership changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_unified_ports(
    graph: Arc<OwnedOriginalUnifiedGraph>,
    max_refinements: usize,
) -> Result<OwnedOriginalUnifiedPorts> {
    let (ports, vertices) = derive(&graph, max_refinements)?;
    Ok(OwnedOriginalUnifiedPorts {
        graph,
        max_refinements,
        ports,
        vertices,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_germ_direction_cell_and_port_membership_changes() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 2.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 2.], vec![2., 0.]]).unwrap();
        let d = Arc::new(
            own_original_common_dictionary(&[a, b], 1e-12, ParameterOrderLimits::default())
                .unwrap(),
        );
        let c = Arc::new(own_original_source_carriers(d, ParameterOrderLimits::default()).unwrap());
        let g = Arc::new(own_original_unified_graph(c).unwrap());
        let x = own_original_unified_ports(g, 192).unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.ports[0].direction *= -1;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.ports[0].ray.signs = [0, 0];
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.ports[0].cell = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.vertices[0].clear();
        assert!(bad.reverify_source().is_err());
    }
}
