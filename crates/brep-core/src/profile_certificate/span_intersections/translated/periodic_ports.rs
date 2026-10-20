//! Exactly two outgoing one-sided original germs per physical quotient edge.
use super::*;
use crate::profile_certificate::algebraic_intersections::{
    decode_polynomial, decode_ratio, polynomial,
};
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicPort {
    pub edge: usize,
    pub end: usize,
    pub vertex: usize,
    pub carrier: usize,
    pub piece: usize,
    pub source: usize,
    pub cell: usize,
    pub interval: usize,
    pub cut: usize,
    pub chart_use: usize,
    pub direction: i8,
    pub ray: LeadingBranchRay,
    pub component_signs: [i8; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicPorts {
    owner: Arc<OwnedPeriodicVertexCharts>,
    max_refinements: usize,
    ports: Vec<OriginalPeriodicPort>,
    vertices: Vec<Vec<usize>>,
}
fn derive(o: &OwnedPeriodicPorts) -> Result<(Vec<OriginalPeriodicPort>, Vec<Vec<usize>>)> {
    if o.max_refinements > 512 {
        return Err(refused(
            "Invalid original quotient port germ refinement allowance",
        ));
    }
    o.owner.reverify_source()?;
    let graph = o.owner.owner();
    let cover = graph.owner().owner().pieces();
    let mut original = Vec::new();
    for source in cover.sources() {
        original.push(cells(source)?);
    }
    let mut ports = Vec::new();
    let mut vertices = vec![Vec::new(); graph.vertices().len()];
    for (edge, e) in graph.edges().iter().enumerate() {
        let representative = e
            .carriers
            .first()
            .ok_or_else(|| refused("Physical quotient edge has no original source carrier"))?;
        let piece = &cover.pieces()[representative.piece];
        if piece.source != representative.source || piece.cell != representative.cell {
            return Err(refused(
                "Quotient port carrier lost actual original cell identity",
            ));
        }
        for end in 0..2 {
            let source_end = end ^ usize::from(representative.reversed);
            let cut = representative.interval + source_end;
            let vertex = e.vertices[end];
            let source_ref = OriginalPeriodicChartCutRef {
                piece: representative.piece,
                cut,
            };
            let chart = &o.owner.charts()[vertex];
            let chart_use = chart
                .uses
                .iter()
                .position(|u| u.source == source_ref)
                .ok_or_else(|| {
                    refused("Original quotient port has no proved local chart transport")
                })?;
            let value = &chart.uses[chart_use].parameter;
            let f = decode_polynomial(&value.primitive)?;
            let parameter = decode_polynomial(&value.parameter)?;
            let root = [
                decode_ratio(&value.interval[0])?,
                decode_ratio(&value.interval[1])?,
            ];
            let cell = &original[piece.source][piece.cell];
            let direction = if source_end == 0 { 1 } else { -1 };
            // Constant chart translations cancel in every rational displacement
            // derivative D^k(X+cW)W-(X+cW)D^kW. Use unchanged ORIGINAL cells.
            let (order, numerator, signs) = arithmetic::branch_germ::leading(
                &cell.net,
                &cell.domain,
                &parameter,
                &f,
                &root,
                direction,
                o.max_refinements,
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
                    o.max_refinements,
                )?,
                arithmetic::branch_germ::component_sign(
                    &cell.net,
                    &cell.domain,
                    &parameter,
                    &f,
                    &root,
                    direction,
                    1,
                    o.max_refinements,
                )?,
            ];
            vertices[vertex].push(ports.len());
            ports.push(OriginalPeriodicPort {
                edge,
                end,
                vertex,
                carrier: 0,
                piece: representative.piece,
                source: piece.source,
                cell: piece.cell,
                interval: representative.interval,
                cut,
                chart_use,
                direction,
                ray: LeadingBranchRay {
                    primitive: value.primitive.clone(),
                    isolating_interval: value.interval.clone(),
                    numerator: numerator.map(|p| polynomial(&p)),
                    signs,
                    order,
                },
                component_signs,
            });
        }
    }
    for (vertex, v) in graph.vertices().iter().enumerate() {
        if vertices[vertex].len() != v.incidences.len() {
            return Err(refused(
                "Physical quotient port census omits endpoint incidences",
            ));
        }
        for incidence in &v.incidences {
            let port = ports
                .get(incidence.edge * 2 + incidence.end)
                .ok_or_else(|| refused("Missing physical edge endpoint port"))?;
            if port.vertex != vertex || port.edge != incidence.edge || port.end != incidence.end {
                return Err(refused(
                    "Physical quotient port is attached to a different endpoint",
                ));
            }
        }
    }
    Ok((ports, vertices))
}
impl OwnedPeriodicPorts {
    pub fn owner(&self) -> &Arc<OwnedPeriodicVertexCharts> {
        &self.owner
    }
    pub fn ports(&self) -> &[OriginalPeriodicPort] {
        &self.ports
    }
    pub fn vertices(&self) -> &[Vec<usize>] {
        &self.vertices
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (p, v) = derive(self)?;
        if p != self.ports || v != self.vertices {
            return Err(refused(
                "Original physical quotient germ, chart or endpoint ownership changed",
            ));
        }
        Ok(())
    }
}
/// Every physical edge has exactly two ports, even a loop. All source aliases
/// stay in the graph; they do not manufacture duplicate geometric branches.
/// Higher-order stationary germs are retained, not replaced by sampled tangents.
pub fn own_periodic_ports(
    owner: Arc<OwnedPeriodicVertexCharts>,
    max_refinements: usize,
) -> Result<OwnedPeriodicPorts> {
    let mut o = OwnedPeriodicPorts {
        owner,
        max_refinements,
        ports: Vec::new(),
        vertices: Vec::new(),
    };
    let (ports, vertices) = derive(&o)?;
    o.ports = ports;
    o.vertices = vertices;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    use periodic_graph::quotient_tests::{aligned, sources};
    fn own(a: Curve) -> OwnedPeriodicPorts {
        let (base, seams) = sources(&[a]);
        let graph = Arc::new(
            own_periodic_quotient_graph(aligned(base, seams), ParameterOrderLimits::default())
                .unwrap(),
        );
        let charts = Arc::new(own_periodic_vertex_charts(graph, 192, 10000).unwrap());
        own_periodic_ports(charts, 192).unwrap()
    }
    #[test]
    fn stationary_loop_start_keeps_second_order_ray_and_distinct_reverse_endpoint() {
        let a = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 6., 6., 6.],
            control_points: vec![vec![0., 0.5], vec![0., 0.5], vec![4., 0.5]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let o = own(a);
        o.reverify_source().unwrap();
        assert_eq!(o.ports().len(), 2);
        assert_eq!(o.vertices(), &[vec![0, 1]]);
        assert_eq!(o.ports()[0].ray.order, 2);
        assert_eq!(o.ports()[1].ray.order, 1);
        assert_eq!(o.ports()[0].ray.signs, [1, 0]);
        assert_eq!(o.ports()[1].ray.signs, [-1, 0]);
        assert_eq!(o.ports()[0].direction, 1);
        assert_eq!(o.ports()[1].direction, -1);
        assert_eq!(o.ports()[0].vertex, o.ports()[1].vertex);
        assert_ne!(o.ports()[0].chart_use, o.ports()[1].chart_use);
        assert!(own_periodic_ports(o.owner().clone(), 513).is_err());
        let mut bad = o.clone();
        bad.ports[0].ray.order = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.ports[1].direction = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.vertices[0].pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.ports[1].chart_use = 0;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn common_double_traversal_has_two_ports_for_two_original_edge_carriers() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![8., 0.5]]).unwrap();
        let o = own(a);
        o.reverify_source().unwrap();
        assert_eq!(o.owner().owner().edges().len(), 1);
        assert_eq!(o.owner().owner().edges()[0].carriers.len(), 2);
        assert_eq!(o.ports().len(), 2);
        assert_eq!(o.vertices()[0].len(), 2);
        assert_eq!(o.ports()[0].ray.signs, [1, 0]);
        assert_eq!(o.ports()[1].ray.signs, [-1, 0]);
        assert!(o.ports().iter().all(|p| p.ray.order == 1));
        let local = &o.owner().charts()[0];
        assert_eq!(local.uses[o.ports()[1].chart_use].period_shift, ["-1", "0"]);
    }
}
