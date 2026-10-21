//! Operand-group periods and explicit Boolean labels from declared base values.
use super::*;
use crate::profile_certificate::algebraic_intersections::decode_ratio;
use num_bigint::BigInt;
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicFillRule {
    NonZero,
    EvenOdd,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicOperand {
    pub wire_weights: Vec<i64>,
    /// Explicit choice of additive constant; a periodic support has no canonical outside.
    pub base_winding: String,
    pub fill: OriginalPeriodicFillRule,
}
/// A bounded DAG; operation references must name earlier program entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicBooleanOp {
    Operand(usize),
    Union(usize, usize),
    Intersection(usize, usize),
    Difference(usize, usize),
    Xor(usize, usize),
    Complement(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicOperandPeriods {
    pub operand: usize,
    pub homology: [String; 2],
    pub period_jumps: [Option<String>; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicMaterialRule {
    base: Arc<OwnedPeriodicRelativeWinding>,
    operands: Vec<OriginalPeriodicOperand>,
    program: Vec<OriginalPeriodicBooleanOp>,
    periods: Vec<OriginalPeriodicOperandPeriods>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicMaterialSample {
    rule: Arc<OwnedPeriodicMaterialRule>,
    query: Arc<OwnedPeriodicRelativeWinding>,
    windings: Vec<String>,
    inside: Vec<bool>,
    selected: bool,
}
fn integer(s: &str) -> Result<BigInt> {
    if s.len() > 4096 {
        return Err(refused("Material integer exceeds exact input allowance"));
    }
    s.parse()
        .map_err(|_| refused("Invalid exact material winding or homology"))
}
fn filled(w: &BigInt, rule: OriginalPeriodicFillRule) -> bool {
    match rule {
        OriginalPeriodicFillRule::NonZero => w != &BigInt::from(0),
        OriginalPeriodicFillRule::EvenOdd => w % BigInt::from(2) != BigInt::from(0),
    }
}
fn period_jumps(
    h: &[BigInt; 2],
    wrapped: [bool; 2],
    fill: OriginalPeriodicFillRule,
) -> Result<[Option<String>; 2]> {
    // Intersect a +U generator with source homology, then a +V generator.
    // On a cylinder the essential boundary parallel to its periodic generator
    // is valid: only the DUAL homology component obstructs a global label.
    let jumps = [-h[1].clone(), h[0].clone()];
    for axis in 0..2 {
        if wrapped[axis] && filled(&jumps[axis], fill) {
            return Err(refused(
                "Operand-group winding is incompatible with a closed support period",
            ));
        }
    }
    Ok(std::array::from_fn(|a| {
        wrapped[a].then(|| jumps[a].to_string())
    }))
}
fn evaluate(program: &[OriginalPeriodicBooleanOp], inside: &[bool]) -> Result<bool> {
    if !(1..=4096).contains(&program.len()) {
        return Err(refused("Invalid bounded periodic Boolean program"));
    }
    let mut values: Vec<bool> = Vec::new();
    for op in program {
        let get = |i: usize| {
            values
                .get(i)
                .copied()
                .ok_or_else(|| refused("Boolean program has a forward or missing reference"))
        };
        let value = match *op {
            OriginalPeriodicBooleanOp::Operand(i) => *inside
                .get(i)
                .ok_or_else(|| refused("Boolean program names a missing operand"))?,
            OriginalPeriodicBooleanOp::Union(a, b) => get(a)? | get(b)?,
            OriginalPeriodicBooleanOp::Intersection(a, b) => get(a)? & get(b)?,
            OriginalPeriodicBooleanOp::Difference(a, b) => get(a)? & !get(b)?,
            OriginalPeriodicBooleanOp::Xor(a, b) => get(a)? ^ get(b)?,
            OriginalPeriodicBooleanOp::Complement(a) => !get(a)?,
        };
        values.push(value);
    }
    Ok(*values.last().unwrap())
}
fn derive_rule(o: &OwnedPeriodicMaterialRule) -> Result<Vec<OriginalPeriodicOperandPeriods>> {
    if !(1..=32).contains(&o.operands.len()) {
        return Err(refused("Invalid periodic material operand count"));
    }
    super::periodic_replay::verify_arc(&o.base, |x| x.reverify_source())?;
    let path = o.base.path();
    if path.len() != 2
        || [decode_ratio(&path[0][0])?, decode_ratio(&path[0][1])?]
            != [decode_ratio(&path[1][0])?, decode_ratio(&path[1][1])?]
        || o.base.deltas().iter().any(|v| v != "0")
    {
        return Err(refused(
            "Material base must be an owned boundary-free zero-length source query",
        ));
    }
    evaluate(&o.program, &vec![false; o.operands.len()])?;
    let owner = o.base.owner();
    let cover = owner.owner().owner().owner().owner().pieces();
    let flux = owner.wire_fluxes();
    let mut periods = Vec::new();
    for (operand, p) in o.operands.iter().enumerate() {
        integer(&p.base_winding)?;
        if p.wire_weights.len() != flux.len() {
            return Err(refused(
                "Material operand must retain every original wire column",
            ));
        }
        let mut h = [BigInt::from(0), BigInt::from(0)];
        for (weight, wire) in p.wire_weights.iter().zip(flux) {
            for axis in 0..2 {
                h[axis] += BigInt::from(*weight) * integer(&wire.reconstructed[axis])?;
            }
        }
        periods.push(OriginalPeriodicOperandPeriods {
            operand,
            period_jumps: period_jumps(&h, cover.wrapped(), p.fill)?,
            homology: h.map(|v| v.to_string()),
        });
    }
    Ok(periods)
}
impl OwnedPeriodicMaterialRule {
    pub fn base(&self) -> &Arc<OwnedPeriodicRelativeWinding> {
        &self.base
    }
    pub fn operands(&self) -> &[OriginalPeriodicOperand] {
        &self.operands
    }
    pub fn program(&self) -> &[OriginalPeriodicBooleanOp] {
        &self.program
    }
    pub fn periods(&self) -> &[OriginalPeriodicOperandPeriods] {
        &self.periods
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        if derive_rule(self)? != self.periods {
            return Err(refused(
                "Material operand periods or original source changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_material_rule(
    base: Arc<OwnedPeriodicRelativeWinding>,
    operands: &[OriginalPeriodicOperand],
    program: &[OriginalPeriodicBooleanOp],
) -> Result<OwnedPeriodicMaterialRule> {
    let _replay = super::periodic_replay::scope();
    let mut o = OwnedPeriodicMaterialRule {
        base,
        operands: operands.to_vec(),
        program: program.to_vec(),
        periods: Vec::new(),
    };
    o.periods = derive_rule(&o)?;
    Ok(o)
}
fn derive_sample(o: &OwnedPeriodicMaterialSample) -> Result<(Vec<String>, Vec<bool>, bool)> {
    super::periodic_replay::verify_arc(&o.rule, |x| x.reverify_source())?;
    super::periodic_replay::verify_arc(&o.query, |x| x.reverify_source())?;
    if !Arc::ptr_eq(o.rule.base.owner(), o.query.owner()) {
        return Err(refused(
            "Material query belongs to a different original physical owner",
        ));
    }
    for axis in 0..2 {
        if decode_ratio(&o.rule.base.path()[0][axis])? != decode_ratio(&o.query.path()[0][axis])? {
            return Err(refused(
                "Material path does not start at the declared base point",
            ));
        }
    }
    let delta = o
        .query
        .deltas()
        .iter()
        .map(|v| integer(v))
        .collect::<Result<Vec<_>>>()?;
    let mut windings = Vec::new();
    let mut inside = Vec::new();
    for p in &o.rule.operands {
        if p.wire_weights.len() != delta.len() {
            return Err(refused("Material query lost original wire columns"));
        }
        let mut value = integer(&p.base_winding)?;
        for (weight, d) in p.wire_weights.iter().zip(&delta) {
            value += BigInt::from(*weight) * d;
        }
        inside.push(filled(&value, p.fill));
        windings.push(value.to_string());
    }
    let selected = evaluate(&o.rule.program, &inside)?;
    Ok((windings, inside, selected))
}
impl OwnedPeriodicMaterialSample {
    pub fn rule(&self) -> &Arc<OwnedPeriodicMaterialRule> {
        &self.rule
    }
    pub fn query(&self) -> &Arc<OwnedPeriodicRelativeWinding> {
        &self.query
    }
    pub fn windings(&self) -> &[String] {
        &self.windings
    }
    pub fn operand_inside(&self) -> &[bool] {
        &self.inside
    }
    pub fn selected(&self) -> bool {
        self.selected
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (w, i, s) = derive_sample(self)?;
        if w != self.windings || i != self.inside || s != self.selected {
            return Err(refused(
                "Material query winding, operand labels or Boolean selection changed",
            ));
        }
        Ok(())
    }
}
/// A point label from declared base constants and original source crossings.
/// Does not prove connected regions, natural-boundary edges or publish faces.
pub fn own_periodic_material_sample(
    rule: Arc<OwnedPeriodicMaterialRule>,
    query: Arc<OwnedPeriodicRelativeWinding>,
) -> Result<OwnedPeriodicMaterialSample> {
    let _replay = super::periodic_replay::scope();
    let mut o = OwnedPeriodicMaterialSample {
        rule,
        query,
        windings: Vec::new(),
        inside: Vec::new(),
        selected: false,
    };
    let (w, i, s) = derive_sample(&o)?;
    o.windings = w;
    o.inside = i;
    o.selected = s;
    Ok(o)
}

// The field is private: only this module can issue the capability, immediately
// after replaying either the exact rule or the actual shore owner containing it.
// It borrows the immutable rule; prepared views retain its actual Arc owners.
pub(super) struct VerifiedMaterialRule<'a> {
    rule: &'a Arc<OwnedPeriodicMaterialRule>,
}
impl VerifiedMaterialRule<'_> {
    pub(super) fn source_owner(&self) -> &Arc<OwnedPeriodicWireJumps> {
        self.rule.base.owner()
    }
}
/// Immutable batch-local replay authority; callers cannot supply cached deltas.
pub(super) struct MaterialView {
    rule: Arc<OwnedPeriodicMaterialRule>,
    winding: super::periodic_relative_winding::RelativeWindingView,
}
impl MaterialView {
    #[cfg(test)]
    pub(super) fn new(
        rule: Arc<OwnedPeriodicMaterialRule>,
        limits: ParameterOrderLimits,
    ) -> Result<Self> {
        rule.reverify_source()?;
        let winding =
            super::periodic_relative_winding::RelativeWindingView::from_verified_material(
                &VerifiedMaterialRule { rule: &rule },
                limits,
            )?;
        Ok(Self { rule, winding })
    }
    /// Fresh replay of the complete shore owner also replays THIS actual rule
    /// and source owner through support selection. Do not re-run those same
    /// immutable subproofs while preparing this one batch.
    pub(super) fn for_shores(
        shores: &OwnedPeriodicNativeChartShores,
        limits: ParameterOrderLimits,
    ) -> Result<Self> {
        shores.reverify_source()?;
        let corners = shores.owner();
        let copies = corners.owner();
        let incidence = copies.owner();
        let traces = incidence.owner();
        let regions = traces.owner();
        let lifts = regions.owner().owner();
        let selection = lifts.owner().owner();
        let rule = selection.rule().material();
        let winding =
            super::periodic_relative_winding::RelativeWindingView::from_verified_material(
                &VerifiedMaterialRule { rule },
                limits,
            )?;
        Ok(Self {
            rule: rule.clone(),
            winding,
        })
    }
    pub(super) fn rule(&self) -> &Arc<OwnedPeriodicMaterialRule> {
        &self.rule
    }
    pub(super) fn clear(&self, endpoints: &[[ExactRatio; 2]; 2], max_work: usize) -> Result<bool> {
        self.winding.clear(endpoints, max_work)
    }
    pub(super) fn sample(
        &self,
        path: &[[ExactRatio; 2]],
        max_work: usize,
    ) -> Result<OwnedPeriodicMaterialSample> {
        let query = Arc::new(self.winding.query(path, max_work)?);
        for axis in 0..2 {
            if decode_ratio(&self.rule.base.path()[0][axis])?
                != decode_ratio(&query.path()[0][axis])?
            {
                return Err(refused(
                    "Material path does not start at the declared base point",
                ));
            }
        }
        let (windings, inside, selected) = self.labels(&query)?;
        Ok(OwnedPeriodicMaterialSample {
            rule: self.rule.clone(),
            query,
            windings,
            inside,
            selected,
        })
    }
    fn labels(
        &self,
        query: &OwnedPeriodicRelativeWinding,
    ) -> Result<(Vec<String>, Vec<bool>, bool)> {
        let rule = &self.rule;
        let delta = query
            .deltas()
            .iter()
            .map(|v| integer(v))
            .collect::<Result<Vec<_>>>()?;
        let mut windings = Vec::new();
        let mut inside = Vec::new();
        for p in &rule.operands {
            if p.wire_weights.len() != delta.len() {
                return Err(refused("Material query lost original wire columns"));
            }
            let mut value = integer(&p.base_winding)?;
            for (weight, d) in p.wire_weights.iter().zip(&delta) {
                value += BigInt::from(*weight) * d;
            }
            inside.push(filled(&value, p.fill));
            windings.push(value.to_string());
        }
        let selected = evaluate(&rule.program, &inside)?;
        Ok((windings, inside, selected))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn point(x: &str, y: &str) -> [ExactRatio; 2] {
        [x, y].map(|v| {
            let (a, b) = v.split_once('/').unwrap_or((v, "1"));
            [a.to_string(), b.to_string()]
        })
    }
    #[test]
    fn dual_periods_distinguish_cylinder_separation_torus_obstruction_and_parity() {
        let h = [BigInt::from(1), BigInt::from(0)];
        assert!(period_jumps(&h, [true, false], OriginalPeriodicFillRule::NonZero).is_ok());
        assert!(period_jumps(&h, [true, true], OriginalPeriodicFillRule::NonZero).is_err());
        let twice = [BigInt::from(-2), BigInt::from(0)];
        assert!(period_jumps(&twice, [true, true], OriginalPeriodicFillRule::EvenOdd).is_ok());
        assert!(period_jumps(&twice, [true, true], OriginalPeriodicFillRule::NonZero).is_err());
        assert!(
            period_jumps(
                &[BigInt::from(0), BigInt::from(1)],
                [true, true],
                OriginalPeriodicFillRule::EvenOdd
            )
            .is_err()
        );
        assert!(
            period_jumps(
                &[BigInt::from(0), BigInt::from(0)],
                [true, true],
                OriginalPeriodicFillRule::NonZero
            )
            .is_ok()
        );
    }
    #[test]
    fn boolean_dag_has_operand_semantics_and_rejects_malformed_references() {
        use OriginalPeriodicBooleanOp::*;
        for a in [false, true] {
            for b in [false, true] {
                for (op, want) in [
                    (Union(0, 1), a | b),
                    (Intersection(0, 1), a & b),
                    (Difference(0, 1), a & !b),
                    (Xor(0, 1), a ^ b),
                ] {
                    assert_eq!(
                        evaluate(&[Operand(0), Operand(1), op], &[a, b]).unwrap(),
                        want
                    );
                }
                assert_eq!(evaluate(&[Operand(0), Complement(0)], &[a, b]).unwrap(), !a);
            }
        }
        assert!(evaluate(&[Union(0, 0)], &[true]).is_err());
        assert!(evaluate(&[Operand(1)], &[true]).is_err());
        assert!(evaluate(&[], &[true]).is_err());
    }
    #[test]
    fn actual_cylinder_difference_does_not_fill_b_only_space_and_grouped_band_keeps_essential_wires()
     {
        use OriginalPeriodicBooleanOp::*;
        let line = |a, b, v| Curve::from_polyline(vec![vec![a, v], vec![b, v]]).unwrap();
        let owner = Arc::new(super::super::periodic_wire_jumps::tests::own(
            &[line(0., 4., 0.75), line(4., 0., 0.25)],
            &[
                vec![OriginalPeriodicSourceUse {
                    source: 0,
                    reversed: false,
                }],
                vec![OriginalPeriodicSourceUse {
                    source: 1,
                    reversed: false,
                }],
            ],
        ));
        let a = point("1/2", "1/8");
        let base = Arc::new(
            own_periodic_relative_winding(
                owner.clone(),
                &[a.clone(), a.clone()],
                ParameterOrderLimits::default(),
                100,
            )
            .unwrap(),
        );
        let operands = [
            OriginalPeriodicOperand {
                wire_weights: vec![1, 0],
                base_winding: "0".into(),
                fill: OriginalPeriodicFillRule::NonZero,
            },
            OriginalPeriodicOperand {
                wire_weights: vec![0, 1],
                base_winding: "1".into(),
                fill: OriginalPeriodicFillRule::NonZero,
            },
        ];
        let rule = Arc::new(
            own_periodic_material_rule(
                base.clone(),
                &operands,
                &[Operand(0), Operand(1), Difference(0, 1)],
            )
            .unwrap(),
        );
        assert_eq!(rule.periods()[0].homology, ["1", "0"]);
        let below = own_periodic_material_sample(rule.clone(), base.clone()).unwrap();
        assert_eq!(below.windings(), &["0", "1"]);
        assert!(!below.selected());
        below.reverify_source().unwrap();
        let query = |y| {
            Arc::new(
                own_periodic_relative_winding(
                    owner.clone(),
                    &[a.clone(), point("1/2", y)],
                    ParameterOrderLimits::default(),
                    100,
                )
                .unwrap(),
            )
        };
        let middle_query = query("1/2");
        let middle = own_periodic_material_sample(rule.clone(), middle_query.clone()).unwrap();
        assert_eq!(middle.operand_inside(), &[false, false]);
        assert!(!middle.selected());
        let upper_query = query("7/8");
        let upper = own_periodic_material_sample(rule.clone(), upper_query.clone()).unwrap();
        assert_eq!(upper.windings(), &["1", "0"]);
        assert!(upper.selected());
        upper.reverify_source().unwrap();
        let batch = MaterialView::new(rule.clone(), ParameterOrderLimits::default()).unwrap();
        for y in ["1/8", "1/2", "7/8"] {
            let sample = batch.sample(&[a.clone(), point("1/2", y)], 100).unwrap();
            let independent = own_periodic_material_sample(rule.clone(), query(y)).unwrap();
            assert_eq!(sample.windings(), independent.windings());
            assert_eq!(sample.operand_inside(), independent.operand_inside());
            assert_eq!(sample.selected(), independent.selected());
            sample.reverify_source().unwrap();
        }
        assert!(
            batch
                .sample(&[point("1/2", "1/2"), point("1/2", "7/8")], 100)
                .is_err()
        );
        assert!(batch.sample(&[a.clone(), point("0", "1/2")], 100).is_err());
        assert!(batch.sample(&[a.clone(), point("1/2", "1/2")], 1).is_err());

        let band = Arc::new(
            own_periodic_material_rule(
                base.clone(),
                &[OriginalPeriodicOperand {
                    wire_weights: vec![1, 1],
                    base_winding: "0".into(),
                    fill: OriginalPeriodicFillRule::NonZero,
                }],
                &[Operand(0)],
            )
            .unwrap(),
        );
        assert_eq!(band.periods()[0].homology, ["0", "0"]);
        assert!(
            own_periodic_material_sample(band.clone(), middle_query)
                .unwrap()
                .selected()
        );
        assert!(
            !own_periodic_material_sample(band.clone(), upper_query)
                .unwrap()
                .selected()
        );
        let mut bad = upper.clone();
        bad.selected = false;
        assert!(bad.reverify_source().is_err());
        let mut bad = upper.clone();
        bad.windings[0] = "0".into();
        assert!(bad.reverify_source().is_err());
        let mut bad = (*rule).clone();
        bad.periods[0].homology[0] = "0".into();
        assert!(bad.reverify_source().is_err());
        assert!(MaterialView::new(Arc::new(bad), ParameterOrderLimits::default()).is_err());
        let mut invalid = operands.clone();
        invalid[0].wire_weights.pop();
        assert!(own_periodic_material_rule(base.clone(), &invalid, &[Operand(0)]).is_err());
        assert!(own_periodic_material_rule(base.clone(), &operands, &[Difference(0, 1)]).is_err());
        let wrong = Arc::new(
            own_periodic_relative_winding(
                owner.clone(),
                &[point("1/2", "1/2"), point("1/2", "7/8")],
                ParameterOrderLimits::default(),
                100,
            )
            .unwrap(),
        );
        assert!(own_periodic_material_sample(rule, wrong).is_err());
        assert!(own_periodic_material_rule(query("1/2"), &operands, &[Operand(0)]).is_err());
    }
}
