//! Exact relation of regularized nonzero-winding regions of ORIGINAL curves.
//! A complete shared graph/side/vertex-sector census is mandatory. This is not
//! a sampled boundary-inclusion test or a three-dimensional volume certificate.
use super::*;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct Limits {
    pub order: ParameterOrderLimits,
    pub ports: ContactPortOrderLimits,
    pub parameter_refinements: usize,
    pub radius_halvings: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            order: ParameterOrderLimits::default(),
            ports: ContactPortOrderLimits::default(),
            parameter_refinements: 192,
            radius_halvings: 32,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sector {
    pub edge: usize,
    pub side: usize,
    pub point: [ExactRatio; 2],
    pub source_winding: i64,
    pub result_winding: i64,
}
pub struct Relation {
    source: Vec<Vec<Curve>>,
    result: Vec<Vec<Curve>>,
    source_fill: OwnedOriginalUnifiedFilled,
    result_fill: OwnedOriginalUnifiedFilled,
    added: Vec<Sector>,
    removed: Vec<Sector>,
}
pub struct NoAddition {
    relation: Relation,
}
impl NoAddition {
    pub fn relation(&self) -> &Relation {
        &self.relation
    }
}
fn sectors(
    a: &OwnedOriginalUnifiedFilled,
    b: &OwnedOriginalUnifiedFilled,
) -> Result<(Vec<Sector>, Vec<Sector>)> {
    if !Arc::ptr_eq(a.cycles(), b.cycles()) || a.winding().len() != b.winding().len() {
        return Err(refused(
            "Material relation needs the same complete original graph",
        ));
    }
    let mut added = Vec::new();
    let mut removed = Vec::new();
    for (edge, (old, new)) in a.winding().iter().zip(b.winding()).enumerate() {
        for side in 0..2 {
            let s = Sector {
                edge,
                side,
                point: a.sides()[edge].certificates()[side].point.clone(),
                source_winding: old[side],
                result_winding: new[side],
            };
            if old[side] == 0 && new[side] != 0 {
                added.push(s);
            } else if old[side] != 0 && new[side] == 0 {
                removed.push(s);
            }
        }
    }
    Ok((added, removed))
}
impl Relation {
    pub fn source(&self) -> &[Vec<Curve>] {
        &self.source
    }
    pub fn result(&self) -> &[Vec<Curve>] {
        &self.result
    }
    pub fn added(&self) -> &[Sector] {
        &self.added
    }
    pub fn removed(&self) -> &[Sector] {
        &self.removed
    }
    pub fn result_is_subset(&self) -> bool {
        self.added.is_empty()
    }
    pub fn source_is_subset(&self) -> bool {
        self.removed.is_empty()
    }
    pub fn edge_count(&self) -> usize {
        self.source_fill.winding().len()
    }
    pub fn reverify(&self) -> Result<()> {
        self.source_fill.reverify_source()?;
        self.result_fill.reverify_source()?;
        let (a, r) = sectors(&self.source_fill, &self.result_fill)?;
        if a != self.added || r != self.removed {
            return Err(refused("Material sector labels changed"));
        }
        Ok(())
    }
    pub fn require_no_addition(self) -> Result<NoAddition> {
        if !self.added.is_empty() {
            return Err(refused("Result contains exact added-material sectors"));
        }
        Ok(NoAddition { relation: self })
    }
}
/// Both regions are regular closures of nonzero winding of bounded closed
/// original wires. Every nonempty added/removed open component has a graph
/// boundary side; complete original cuts, rotation sectors and exact side
/// winding therefore decide inclusion throughout the plane, including holes.
/// Isolation width only controls root enclosures; it is NOT a geometry tolerance.
pub fn inspect(
    source: &[Vec<Curve>],
    result: &[Vec<Curve>],
    width: f64,
    limits: Limits,
) -> Result<Relation> {
    if source.is_empty() || result.is_empty() {
        return Err(refused(
            "Material relation currently requires two nonempty wire groups",
        ));
    }
    let loops = source.iter().chain(result).cloned().collect::<Vec<_>>();
    point_winding::validate_source_wires(&loops)?;
    let curves = loops.iter().flatten().cloned().collect::<Vec<_>>();
    let dictionary = Arc::new(own_original_common_dictionary(
        &curves,
        width,
        limits.order,
    )?);
    let carriers = Arc::new(own_original_source_carriers(dictionary, limits.order)?);
    let graph = Arc::new(own_original_unified_graph(carriers)?);
    let ports = Arc::new(own_original_unified_ports(
        graph.clone(),
        limits.parameter_refinements,
    )?);
    let rotation = Arc::new(own_original_unified_rotation(ports, limits.ports)?);
    let cycles = Arc::new(own_original_unified_cycles(rotation)?);
    let sides = own_original_unified_sides_automatically(
        graph,
        &loops,
        limits.order,
        limits.radius_halvings,
    )?;
    let source_coefficients = (0..loops.len())
        .map(|i| i32::from(i < source.len()))
        .collect::<Vec<_>>();
    let result_coefficients = (0..loops.len())
        .map(|i| i32::from(i >= source.len()))
        .collect::<Vec<_>>();
    let source_fill =
        own_original_unified_filled(cycles.clone(), sides.clone(), &source_coefficients)?;
    let result_fill = own_original_unified_filled(cycles, sides, &result_coefficients)?;
    let (added, removed) = sectors(&source_fill, &result_fill)?;
    Ok(Relation {
        source: source.to_vec(),
        result: result.to_vec(),
        source_fill,
        result_fill,
        added,
        removed,
    })
}
