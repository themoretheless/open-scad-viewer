//! Exact common quadratic jets and facewise curvature limits at owned graph
//! poles. Ordinary seam G1/G2 and whole-neighborhood C2 remain separate.
use super::*;
pub struct PoleCurvatureLimits {
    pub graphs: GraphLimits,
    pub jet_work: u64,
}
pub struct PoleCurvatureAtlas<'a, E: Carrier> {
    graphs: PoleGraphAtlas<'a, E>,
    jets: Vec<(Address, normals::QuadraticJet<'a>)>,
}
impl<'a, E: Carrier> PoleCurvatureAtlas<'a, E> {
    pub fn graphs(&self) -> &PoleGraphAtlas<'a, E> {
        &self.graphs
    }
    pub fn jets(&self) -> &[(Address, normals::QuadraticJet<'a>)] {
        &self.jets
    }
}
pub struct PoleCurvatureReport<'a, E: Carrier> {
    pub certificate: Option<PoleCurvatureAtlas<'a, E>>,
    pub graph_exact_work: u64,
    pub jet_work: u64,
    pub completed_poles: usize,
    pub comparisons: usize,
    pub uncertain_vertex: Option<usize>,
    pub reason: &'static str,
}
/// Freshly qualify the graph atlas, every original complete angular two-jet,
/// and exact agreement across ALL incident charts. The complete quadratic jet
/// identity leaves O(u³) analytic residual. First-order transverse graph
/// collars (normal factor exactly u) give uniformly controlled polar inverses;
/// the facewise graph Hessian residual is O(u). No sampled curvature or caller
/// Hessian candidate authorizes the result. All jet queries/comparisons share
/// ONE additional exact-work allowance; failure never yields a partial atlas.
pub fn qualify_body_pole_curvature<E: Carrier>(
    body: &Body<E>,
    limits: PoleCurvatureLimits,
) -> Result<PoleCurvatureReport<'_, E>> {
    if !(1..=100000000).contains(&limits.jet_work) {
        return Err(Error::new(
            "BREP_SOURCE_POLE_CURVATURE",
            "Choose bounded exact pole jet work",
        ));
    }
    let r = qualify_body_pole_graphs(body, limits.graphs)?;
    let mut out = PoleCurvatureReport {
        certificate: None,
        graph_exact_work: r.exact_work,
        jet_work: 0,
        completed_poles: 0,
        comparisons: 0,
        uncertain_vertex: r.uncertain_vertex,
        reason: r.reason,
    };
    let Some(graphs) = r.certificate else {
        return Ok(out);
    };
    let shell = body.geometry().shell();
    let projections = graphs.charts().transverse().projections();
    let mut jets: Vec<(Address, normals::QuadraticJet<'_>)> = Vec::new();
    let mut first = BTreeMap::<usize, usize>::new();
    for (i, (address, chart)) in graphs.charts().charts().iter().enumerate() {
        let vertex = shell.vertices()[address.face][address.wire][address.edge][0];
        out.uncertain_vertex = Some(vertex);
        if projections[i].0 != *address || projections[i].1.factor_order() != 1 {
            out.reason = "pole-curvature-first-order-inverse-unproven";
            return Ok(out);
        }
        if out.jet_work == limits.jet_work {
            out.reason = "pole-curvature-jet-work-limit";
            return Ok(out);
        }
        let source = shell.poles()[i].1.source().surface();
        if !std::ptr::eq(source, chart.surface()) {
            out.reason = "pole-curvature-original-ownership-unproven";
            return Ok(out);
        }
        let r = normals::prepare_quadratic_jet(
            source,
            chart.boundary(),
            chart.dropped_axis(),
            limits.jet_work - out.jet_work,
        )?;
        out.jet_work += r.work;
        let Some(jet) = r.certificate else {
            out.reason = r.reason;
            return Ok(out);
        };
        if jet.point() != shell.poles()[i].1.point() {
            out.reason = "pole-curvature-point-ownership-unproven";
            return Ok(out);
        }
        if let Some(&previous) = first.get(&vertex) {
            if out.jet_work == limits.jet_work {
                out.reason = "pole-curvature-jet-work-limit";
                return Ok(out);
            }
            let r = normals::compare_quadratic_jets(
                &jets[previous].1,
                &jet,
                limits.jet_work - out.jet_work,
            )?;
            out.jet_work += r.work;
            out.comparisons += 1;
            if r.same != Some(true) {
                out.reason = "pole-curvature-common-exact-jet-unproven";
                return Ok(out);
            }
        } else {
            first.insert(vertex, jets.len());
        }
        jets.push((*address, jet));
        out.completed_poles += 1;
    }
    out.certificate = Some(PoleCurvatureAtlas { graphs, jets });
    out.uncertain_vertex = None;
    out.reason = "original-pole-quadratic-jets-and-facewise-curvature-limits-qualified";
    Ok(out)
}
