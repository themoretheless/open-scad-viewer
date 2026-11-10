//! Actual edge clock normalization, preserving independent original face clocks.
use super::*;
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
use nurbs_core::{curve::Curve, interval_eval::Interval as I};
mod binding;
#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    pub face: usize,
    pub wire: usize,
    pub coedge: usize,
    pub original: brep_topology::CoedgeTrim,
    pub curve_parameters: [Q; 2],
    pub transferred: brep_topology::CoedgeTrim,
    pub original_curve_distance_upper: f64,
}
pub struct Reclock {
    source: Model,
    model: Model,
    edge: usize,
    domain: [f64; 2],
    key: String,
    limits: Limits,
    evidence: Verification,
    original_distance_upper: f64,
    relations: Vec<Relation>,
}
impl Reclock {
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }
    pub fn evidence(&self) -> &Verification {
        &self.evidence
    }
    pub fn original_distance_upper(&self) -> f64 {
        self.original_distance_upper
    }
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }
    pub fn reverify(&self) -> Result<Verification> {
        let actual = reparameterize(&self.source, self.edge, self.domain, &self.key, self.limits)?;
        if actual.model != self.model || actual.relations != self.relations {
            return Err(error(
                "Saved edge clock differs from original reconstruction",
            ));
        }
        Ok(actual.evidence)
    }
}
fn q(v: f64) -> Q {
    Q::from_float(v).unwrap()
}
fn image(value: f64, source: [f64; 2], target: [f64; 2]) -> Q {
    q(target[0])
        + (q(value) - q(source[0])) * (q(target[1]) - q(target[0])) / (q(source[1]) - q(source[0]))
}
fn stored(value: &Q) -> Result<f64> {
    value
        .to_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| error("Edge clock image is not finite"))
}
fn candidate(source: &Curve, target: [f64; 2]) -> Result<Curve> {
    let d = source.domain();
    let mut actual = source.clone();
    for (k, &old) in actual.knots.iter_mut().zip(&source.knots) {
        *k = if old == d[0] {
            target[0]
        } else if old == d[1] {
            target[1]
        } else {
            stored(&image(old, d, target))?
        };
    }
    for (old, new) in source.knots.windows(2).zip(actual.knots.windows(2)) {
        if (old[0] < old[1]) != (new[0] < new[1]) || (old[0] == old[1]) != (new[0] == new[1]) {
            return Err(error(
                "Reclock rounding collapses or changes the original knot partition",
            ));
        }
    }
    actual.validate()?;
    Ok(actual)
}
/// Change only a regular stored world curve's affine clock. Its complete image,
/// every original authored request and the final actual closed model must pass
/// fresh admission. Face carriers, UV curves and vertex positions are retained.
pub fn reparameterize(
    source: &Model,
    edge: usize,
    domain: [f64; 2],
    key: &str,
    limits: Limits,
) -> Result<Reclock> {
    source.validate()?;
    if edge >= source.edges.len()
        || source.edges[edge].degenerate
        || key.is_empty()
        || key.len() > 128
        || domain.iter().any(|v| !v.is_finite())
        || domain[0] >= domain[1]
        || !(domain[1] - domain[0]).is_finite()
    {
        return Err(error(
            "Choose a regular source edge, finite increasing clock and bounded key",
        ));
    }
    if !(1..=100000).contains(&limits.max_cells)
        || !(1..=100000).contains(&limits.max_pairs)
        || !(1..=1000000).contains(&limits.max_domain_cells)
        || !(100..=2000000).contains(&limits.mass_evaluations)
    {
        return Err(error("Choose bounded edge clock work"));
    }
    let prior = qualification::verify_edge_refinement(source, limits)?;
    let mut work = Work {
        cells: prior.cells,
        pairs: prior.pairs,
        domain_cells: prior.domain_cells,
    };
    let original = &source.edges[edge].curve;
    let actual = candidate(original, domain)?;
    let tolerance = source
        .1
        .local_tolerances_mm
        .get(&source.1.edges[edge])
        .copied()
        .unwrap_or(source.tolerance_mm)
        .min(source.tolerance_mm);
    let agreement = partition::mapped::verify(
        original,
        &actual,
        original.domain(),
        domain,
        tolerance,
        work.remaining(limits)?.max_cells,
    )?;
    work.cells += agreement.cells;
    if !agreement.accepted {
        return Err(error(
            "Original whole edge versus changed clock remains unproven",
        ));
    }
    let mut model = source.clone();
    model.0.edges[edge].curve = actual;
    let relations = binding::transfer(
        source,
        &mut model,
        edge,
        agreement.error_upper,
        tolerance,
        limits,
        &mut work,
    )?;
    let old = source.1.edges[edge];
    let id = TopoId::derive(
        TopoKind::Edge,
        "edge-reclock",
        key,
        "edge",
        old.to_string().as_bytes(),
    );
    model.1.edges[edge] = id;
    if let Some(t) = model.1.local_tolerances_mm.remove(&old) {
        model.1.local_tolerances_mm.insert(id, t);
    }
    model.1.sewed_edges.remove(&old);
    model.1.lineage.push(crate::TopologyLineageRecord {
        operation: "persist".into(),
        entity_kind: "edge".into(),
        parents: vec![old],
        children: vec![id],
    });
    let changed = relations
        .iter()
        .map(|r| r.wire)
        .collect::<std::collections::BTreeSet<_>>();
    for wire in changed {
        let old = source.1.loops[wire];
        let id = TopoId::derive(
            TopoKind::Loop,
            "edge-reclock",
            key,
            &old.to_string(),
            source.1.edges[edge].to_string().as_bytes(),
        );
        model.1.loops[wire] = id;
        if let Some(t) = model.1.local_tolerances_mm.remove(&old) {
            model.1.local_tolerances_mm.insert(id, t);
        }
        model.1.lineage.push(crate::TopologyLineageRecord {
            operation: "persist".into(),
            entity_kind: "loop".into(),
            parents: vec![old],
            children: vec![id],
        });
    }
    model.refresh_change_set(&[source]);
    model.validate()?;
    binding::endpoints(&model, edge, limits, &mut work)?;
    let mut evidence = qualification::verify_edge_refinement(&model, work.remaining(limits)?)?;
    work.include(&mut evidence);
    Ok(Reclock {
        source: source.clone(),
        model,
        edge,
        domain,
        key: key.into(),
        limits,
        evidence,
        original_distance_upper: agreement.error_upper,
        relations,
    })
}
