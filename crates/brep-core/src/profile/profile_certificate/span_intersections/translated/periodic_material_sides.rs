//! Original-source side probes and Boolean boundary selection in an actual chart.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio, ratio};
use arithmetic::branch_neighbourhood::ParameterValue;
use num_bigint::BigInt;
use num_traits::Signed;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSideProbes {
    transported: bool,
    lifted_queries: [[ExactRatio; 2]; 2],
    period_shifts: [[String; 2]; 2],
    views: Vec<OriginalPeriodicSideView>,
    owner: Arc<OwnedPeriodicWireJumps>,
    edge: usize,
    radius: ExactRatio,
    limits: ParameterOrderLimits,
    parameter: ExactRatio,
    queries: [[ExactRatio; 2]; 2],
    jumps: Vec<String>,
}
/// Compatibility facade for the strict chart constructor.
pub type OwnedPeriodicChartSideProbes = OwnedPeriodicSideProbes;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSideView {
    pub piece: usize,
    pub period_shift: [String; 2],
}
pub(in super::super) type ProbeData = (
    ExactRatio,
    [[ExactRatio; 2]; 2],
    [[ExactRatio; 2]; 2],
    [[String; 2]; 2],
    Vec<OriginalPeriodicSideView>,
    Vec<String>,
);
fn bound(p: &OriginalSourceParameter, domain: Option<&[Q; 2]>) -> Result<ParameterValue> {
    let mut value = decode_polynomial(&p.parameter)?;
    if let Some(d) = domain {
        if value.is_empty() {
            value.push(-d[0].clone());
        } else {
            value[0] -= &d[0];
        }
        for x in &mut value {
            *x /= &d[1] - &d[0];
        }
    }
    Ok(ParameterValue {
        primitive: decode_polynomial(&p.primitive)?,
        value,
        interval: [decode_ratio(&p.interval[0])?, decode_ratio(&p.interval[1])?],
    })
}
fn derive_probes(o: &OwnedPeriodicSideProbes) -> Result<ProbeData> {
    source_probe_data(&o.owner, o.edge, &o.radius, o.limits, o.transported, false)
}
/// Shared complete source census. Allowing a natural exterior returns DATA,
/// never an ordinary qualified side owner; the typed support caller must prove
/// the actual natural carrier and the inward/outward half-space separately.
pub(in super::super) fn source_probe_data(
    owner: &Arc<OwnedPeriodicWireJumps>,
    edge: usize,
    radius: &ExactRatio,
    limits: ParameterOrderLimits,
    transported: bool,
    allow_natural: bool,
) -> Result<ProbeData> {
    struct Request<'a> {
        owner: &'a Arc<OwnedPeriodicWireJumps>,
        edge: usize,
        radius: &'a ExactRatio,
        limits: ParameterOrderLimits,
        transported: bool,
    }
    let o = Request {
        owner,
        edge,
        radius,
        limits,
        transported,
    };

    if o.limits.max_refinements > 512 || !(1..=128).contains(&o.limits.max_resultant_size) {
        return Err(refused("Invalid periodic side parameter allowance"));
    }
    super::periodic_replay::verify_arc(o.owner, |x| x.reverify_source())?;
    let graph = o.owner.owner().owner();
    let aligned = graph.owner();
    let cover = aligned.owner().pieces();
    let e = graph
        .edges()
        .get(o.edge)
        .ok_or_else(|| refused("Periodic side names a missing physical edge"))?;
    let c = e
        .carriers
        .first()
        .ok_or_else(|| refused("Periodic side has no original carrier"))?;
    let original = cover
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let selected = &original[c.source][c.cell];
    let parameter = arithmetic::carrier_sides::interior(
        &[(selected.net.clone(), selected.domain.clone())],
        [
            bound(&aligned.cuts()[c.piece][c.interval].value, None)?,
            bound(&aligned.cuts()[c.piece][c.interval + 1].value, None)?,
        ],
        o.limits.max_refinements,
    )?;
    let local = (&parameter - &selected.domain[0]) / (&selected.domain[1] - &selected.domain[0]);
    let chart = cover.chart().map(|d| d.map(|v| Q::from_float(v).unwrap()));
    let periods = chart.each_ref().map(|d| &d[1] - &d[0]);
    let wrapped = cover.wrapped();
    let mut tiles = vec![[0i8, 0i8]];
    if o.transported {
        for axis in 0..2 {
            if wrapped[axis] {
                tiles = tiles
                    .into_iter()
                    .flat_map(|t| {
                        [0, -1, 1].map(|v| {
                            let mut n = t;
                            n[axis] = v;
                            n
                        })
                    })
                    .collect();
            }
        }
    }
    let mut nets = Vec::new();
    let mut intervals = Vec::new();
    let mut views = Vec::new();
    for tile in &tiles {
        for (piece, p) in cover.pieces().iter().enumerate() {
            let cell = &original[p.source][p.cell];
            let offset = [
                decode_ratio(&p.translation[0])? + Q::from_integer(tile[0].into()) * &periods[0],
                decode_ratio(&p.translation[1])? + Q::from_integer(tile[1].into()) * &periods[1],
            ];
            nets.push(
                cell.net
                    .iter()
                    .map(|v| {
                        [
                            &v[0] + &offset[0] * &v[2],
                            &v[1] + &offset[1] * &v[2],
                            v[2].clone(),
                        ]
                    })
                    .collect(),
            );
            intervals.push([
                bound(&cover.cuts()[p.source][p.cuts[0]].value, Some(&cell.domain))?,
                bound(&cover.cuts()[p.source][p.cuts[1]].value, Some(&cell.domain))?,
            ]);
            views.push(OriginalPeriodicSideView {
                piece,
                period_shift: tile.map(|n| n.to_string()),
            });
        }
    }
    let mut centers = Vec::new();
    let mut groups = Vec::new();
    for (group, c) in e.carriers.iter().enumerate() {
        let p = &cover.pieces()[c.piece];
        if p.source != c.source || p.cell != c.cell {
            return Err(refused("Periodic side carrier lost its original cell"));
        }
        let cell = &original[c.source][c.cell];
        let bounds = [
            bound(
                &aligned.cuts()[c.piece][c.interval].value,
                Some(&cell.domain),
            )?,
            bound(
                &aligned.cuts()[c.piece][c.interval + 1].value,
                Some(&cell.domain),
            )?,
        ];
        for tile in 0..tiles.len() {
            centers.push(arithmetic::carrier_sides::CenterInterval {
                cell: tile * cover.pieces().len() + c.piece,
                bounds: bounds.clone(),
            });
            groups.push(group);
        }
    }
    let mut lifted = arithmetic::carrier_sides::probes_with_owned_center_groups(
        &nets,
        c.piece,
        &local,
        &decode_ratio(&o.radius)?,
        o.limits.max_refinements,
        &centers,
        o.limits.max_resultant_size,
        &intervals,
        &groups,
    )?;
    if c.reversed {
        lifted.swap(0, 1);
    }
    // Completeness of the finite tile census: every actual canonical piece is
    // inside the closed source chart. Further tiles are >=one full period
    // away from its center, while this segment stays <half a period away.
    for axis in 0..2 {
        let center = (&lifted[0][axis] + &lifted[1][axis]) / Q::from_integer(2.into());
        if center < chart[axis][0] || center > chart[axis][1] {
            return Err(refused(
                "Periodic side center escaped its actual original source chart",
            ));
        }
        if o.transported && wrapped[axis] {
            let extent = (&lifted[0][axis] - &lifted[1][axis]).abs();
            if extent >= periods[axis] {
                return Err(refused(
                    "Periodic side exceeds the complete adjacent-tile radius allowance",
                ));
            }
        }
    }
    let mut canonical = lifted.clone();
    let mut shifts = std::array::from_fn(|_| std::array::from_fn(|_| "0".to_string()));
    for side in 0..2 {
        for axis in 0..2 {
            if o.transported && wrapped[axis] {
                let k = ((&canonical[side][axis] - &chart[axis][0]) / &periods[axis])
                    .floor()
                    .to_integer();
                canonical[side][axis] -= Q::from_integer(k.clone()) * &periods[axis];
                shifts[side][axis] = k.to_string();
            }
            if canonical[side][axis] <= chart[axis][0] || canonical[side][axis] >= chart[axis][1] {
                if allow_natural
                    && !wrapped[axis]
                    && (canonical[side][axis] < chart[axis][0]
                        || canonical[side][axis] > chart[axis][1])
                {
                    continue;
                }

                return Err(refused(
                    "Periodic side query needs another physical chart or natural support boundary owner",
                ));
            }
        }
    }
    let mut jumps = vec!["0".to_string(); o.owner.wire_fluxes().len()];
    for j in &o.owner.edges()[o.edge].jumps {
        jumps[j.wire] = j.delta.clone();
    }
    Ok((
        ratio(&parameter),
        canonical.map(|p| p.map(|q| ratio(&q))),
        lifted.map(|p| p.map(|q| ratio(&q))),
        shifts,
        views,
        jumps,
    ))
}
impl OwnedPeriodicSideProbes {
    pub fn owner(&self) -> &Arc<OwnedPeriodicWireJumps> {
        &self.owner
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn parameter(&self) -> &ExactRatio {
        &self.parameter
    }
    pub fn queries(&self) -> &[[ExactRatio; 2]; 2] {
        &self.queries
    }
    pub fn lifted_queries(&self) -> &[[ExactRatio; 2]; 2] {
        &self.lifted_queries
    }
    pub fn period_shifts(&self) -> &[[String; 2]; 2] {
        &self.period_shifts
    }
    pub fn views(&self) -> &[OriginalPeriodicSideView] {
        &self.views
    }
    pub fn transported(&self) -> bool {
        self.transported
    }
    pub fn jumps(&self) -> &[String] {
        &self.jumps
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let actual = derive_probes(self)?;
        if actual
            != (
                self.parameter.clone(),
                self.queries.clone(),
                self.lifted_queries.clone(),
                self.period_shifts.clone(),
                self.views.clone(),
                self.jumps.clone(),
            )
        {
            return Err(refused(
                "Original periodic side parameter, probes or wire jumps changed",
            ));
        }
        Ok(())
    }
}
/// Strict chart-local facade. It never wraps a shore-crossing query.
pub fn own_periodic_chart_side_probes(
    owner: Arc<OwnedPeriodicWireJumps>,
    edge: usize,
    radius: ExactRatio,
    limits: ParameterOrderLimits,
) -> Result<OwnedPeriodicChartSideProbes> {
    build_probes(owner, edge, radius, limits, false)
}
/// Actual support-bound quotient probes with a COMPLETE adjacent-tile census.
/// Universal-cover views are exact UV expressions; S is evaluated only on its
/// original canonical domain. Natural unwrapped support boundaries still refuse.
pub fn own_periodic_physical_side_probes(
    owner: Arc<OwnedPeriodicWireJumps>,
    edge: usize,
    radius: ExactRatio,
    limits: ParameterOrderLimits,
) -> Result<OwnedPeriodicSideProbes> {
    build_probes(owner, edge, radius, limits, true)
}
fn build_probes(
    owner: Arc<OwnedPeriodicWireJumps>,
    edge: usize,
    radius: ExactRatio,
    limits: ParameterOrderLimits,
    transported: bool,
) -> Result<OwnedPeriodicSideProbes> {
    let _replay = super::periodic_replay::scope();
    let zero = std::array::from_fn(|_| std::array::from_fn(|_| ["0".into(), "1".into()]));
    let mut o = OwnedPeriodicSideProbes {
        owner,
        edge,
        radius,
        limits,
        transported,
        parameter: ["0".into(), "1".into()],
        queries: zero,
        lifted_queries: std::array::from_fn(|_| std::array::from_fn(|_| ["0".into(), "1".into()])),
        period_shifts: std::array::from_fn(|_| std::array::from_fn(|_| "0".into())),
        views: Vec::new(),
        jumps: Vec::new(),
    };
    let (p, q, l, k, v, j) = derive_probes(&o)?;
    o.parameter = p;
    o.queries = q;
    o.lifted_queries = l;
    o.period_shifts = k;
    o.views = v;
    o.jumps = j;
    Ok(o)
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicMaterialSides {
    probes: Arc<OwnedPeriodicChartSideProbes>,
    samples: [Arc<OwnedPeriodicMaterialSample>; 2],
    operand_jumps: Vec<String>,
    wire_values: [Vec<String>; 2],
    operand_values: [Vec<String>; 2],
    direction: Option<i8>,
}
fn integer(v: &str) -> Result<BigInt> {
    v.parse()
        .map_err(|_| refused("Invalid exact periodic side winding"))
}
type SideData = (Vec<String>, Option<i8>, [Vec<String>; 2], [Vec<String>; 2]);
fn derive_sides(o: &OwnedPeriodicMaterialSides) -> Result<SideData> {
    super::periodic_replay::verify_arc(&o.probes, |x| x.reverify_source())?;
    for s in &o.samples {
        super::periodic_replay::verify_arc(s, |x| x.reverify_source())?;
    }
    if !Arc::ptr_eq(o.samples[0].rule(), o.samples[1].rule()) {
        return Err(refused(
            "Periodic material sides use different operand rules",
        ));
    }
    for side in 0..2 {
        let s = &o.samples[side];
        if !Arc::ptr_eq(s.query().owner(), o.probes.owner()) {
            return Err(refused(
                "Periodic side query belongs to a different original owner",
            ));
        }
        let endpoint = s
            .query()
            .path()
            .last()
            .ok_or_else(|| refused("Periodic side query has no endpoint"))?;
        for axis in 0..2 {
            if decode_ratio(&endpoint[axis])? != decode_ratio(&o.probes.queries()[side][axis])? {
                return Err(refused(
                    "Material side query does not end at the proved original probe",
                ));
            }
        }
    }
    let mut wire_values: [Vec<String>; 2] = std::array::from_fn(|_| Vec::new());
    for side in 0..2 {
        let k = [
            integer(&o.probes.period_shifts()[side][0])?,
            integer(&o.probes.period_shifts()[side][1])?,
        ];
        for (wire, flux) in o.probes.owner().wire_fluxes().iter().enumerate() {
            let h = [
                integer(&flux.reconstructed[0])?,
                integer(&flux.reconstructed[1])?,
            ];
            // A +U deck period intersects -H_v; a +V period intersects H_u.
            let value =
                integer(&o.samples[side].query().deltas()[wire])? - &k[0] * &h[1] + &k[1] * &h[0];
            wire_values[side].push(value.to_string());
        }
    }
    for wire in 0..o.probes.jumps().len() {
        if integer(&wire_values[0][wire])? - integer(&wire_values[1][wire])?
            != integer(&o.probes.jumps()[wire])?
        {
            return Err(refused(
                "Transported periodic sides disagree with the original source wire jump",
            ));
        }
    }
    let mut operand_values: [Vec<String>; 2] = std::array::from_fn(|_| Vec::new());
    for side in 0..2 {
        for (operand, p) in o.samples[side].rule().operands().iter().enumerate() {
            let mut value = integer(&p.base_winding)?;
            for (w, v) in p.wire_weights.iter().zip(&wire_values[side]) {
                value += BigInt::from(*w) * integer(v)?;
            }
            let inside = match p.fill {
                OriginalPeriodicFillRule::NonZero => value != BigInt::from(0),
                OriginalPeriodicFillRule::EvenOdd => &value % BigInt::from(2) != BigInt::from(0),
            };
            if inside != o.samples[side].operand_inside()[operand] {
                return Err(refused(
                    "Transported operand fill changes across a qualified support period",
                ));
            }
            operand_values[side].push(value.to_string());
        }
    }
    let mut jumps = Vec::new();
    for (operand, p) in o.samples[0].rule().operands().iter().enumerate() {
        let mut expected = BigInt::from(0);
        for (w, j) in p.wire_weights.iter().zip(o.probes.jumps()) {
            expected += BigInt::from(*w) * integer(j)?;
        }
        if integer(&operand_values[0][operand])? - integer(&operand_values[1][operand])? != expected
        {
            return Err(refused(
                "Periodic operand side jump lost transported source multiplicity",
            ));
        }
        jumps.push(expected.to_string());
    }
    let direction = match (o.samples[0].selected(), o.samples[1].selected()) {
        (true, false) => Some(1),
        (false, true) => Some(-1),
        _ => None,
    };
    Ok((jumps, direction, wire_values, operand_values))
}
impl OwnedPeriodicMaterialSides {
    pub fn probes(&self) -> &Arc<OwnedPeriodicChartSideProbes> {
        &self.probes
    }
    pub fn samples(&self) -> &[Arc<OwnedPeriodicMaterialSample>; 2] {
        &self.samples
    }
    pub fn operand_jumps(&self) -> &[String] {
        &self.operand_jumps
    }
    pub fn transported_wire_values(&self) -> &[Vec<String>; 2] {
        &self.wire_values
    }
    pub fn transported_operand_values(&self) -> &[Vec<String>; 2] {
        &self.operand_values
    }
    /// Retain canonical (+1) or reverse (-1) so material stays on the left.
    pub fn boundary_direction(&self) -> Option<i8> {
        self.direction
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (j, d, w, v) = derive_sides(self)?;
        if j != self.operand_jumps
            || d != self.direction
            || w != self.wire_values
            || v != self.operand_values
        {
            return Err(refused(
                "Periodic material side jump or boundary direction changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_material_sides(
    probes: Arc<OwnedPeriodicChartSideProbes>,
    samples: [Arc<OwnedPeriodicMaterialSample>; 2],
) -> Result<OwnedPeriodicMaterialSides> {
    let _replay = super::periodic_replay::scope();
    let mut o = OwnedPeriodicMaterialSides {
        probes,
        samples,
        operand_jumps: Vec::new(),
        wire_values: std::array::from_fn(|_| Vec::new()),
        operand_values: std::array::from_fn(|_| Vec::new()),
        direction: None,
    };
    let (j, d, w, v) = derive_sides(&o)?;
    o.wire_values = w;
    o.operand_values = v;
    o.operand_jumps = j;
    o.direction = d;
    Ok(o)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSelectedEdge {
    pub edge: usize,
    pub direction: i8,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicBoundarySelection {
    rule: Arc<OwnedPeriodicMaterialRule>,
    sides: Vec<Arc<OwnedPeriodicMaterialSides>>,
    selected: Vec<OriginalPeriodicSelectedEdge>,
}
fn derive_selection(
    o: &OwnedPeriodicBoundarySelection,
) -> Result<Vec<OriginalPeriodicSelectedEdge>> {
    super::periodic_replay::verify_arc(&o.rule, |x| x.reverify_source())?;
    let graph = o.rule.base().owner().owner().owner();
    if o.sides.len() != graph.edges().len() || o.sides.len() > 1000000 {
        return Err(refused(
            "Periodic boundary selection must own every physical edge side pair",
        ));
    }
    let mut selected = Vec::new();
    let mut flow = vec![0i64; graph.vertices().len()];
    for (edge, sides) in o.sides.iter().enumerate() {
        super::periodic_replay::verify_arc(sides, |x| x.reverify_source())?;
        if sides.probes().edge() != edge || !Arc::ptr_eq(sides.samples()[0].rule(), &o.rule) {
            return Err(refused(
                "Periodic boundary selection has missing, repeated or unrelated edge labels",
            ));
        }
        if let Some(direction) = sides.boundary_direction() {
            let ends = graph.edges()[edge].vertices;
            flow[ends[0]] += i64::from(direction);
            flow[ends[1]] -= i64::from(direction);
            selected.push(OriginalPeriodicSelectedEdge { edge, direction });
        }
    }
    if flow.iter().any(|v| *v != 0) {
        return Err(refused(
            "Selected periodic boundary is not conserved at every physical vertex",
        ));
    }
    Ok(selected)
}
impl OwnedPeriodicBoundarySelection {
    pub fn rule(&self) -> &Arc<OwnedPeriodicMaterialRule> {
        &self.rule
    }
    pub fn sides(&self) -> &[Arc<OwnedPeriodicMaterialSides>] {
        &self.sides
    }
    pub fn selected(&self) -> &[OriginalPeriodicSelectedEdge] {
        &self.selected
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        if derive_selection(self)? != self.selected {
            return Err(refused(
                "Complete original periodic Boolean boundary selection changed",
            ));
        }
        Ok(())
    }
}
/// Complete original graph edge selection, with material on the left and
/// conserved physical-vertex flow. Natural support boundaries and connected
/// physical faces still need their own owners; this does not publish faces.
pub fn own_periodic_boundary_selection(
    rule: Arc<OwnedPeriodicMaterialRule>,
    sides: &[Arc<OwnedPeriodicMaterialSides>],
) -> Result<OwnedPeriodicBoundarySelection> {
    let _replay = super::periodic_replay::scope();
    let mut o = OwnedPeriodicBoundarySelection {
        rule,
        sides: sides.to_vec(),
        selected: Vec::new(),
    };
    o.selected = derive_selection(&o)?;
    Ok(o)
}
#[cfg(test)]
#[path = "periodic_material_sides/tests/tests_integration.rs"]
mod tests;
