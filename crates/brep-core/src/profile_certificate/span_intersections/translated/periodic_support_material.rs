//! Support-clipped Boolean labels; exterior points have NO authored sample.
use super::*;
use crate::profile_certificate::algebraic_intersections::decode_ratio;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicNaturalCarrier {
    pub boundary: usize,
    pub carrier: usize,
    pub inward_left: bool,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportSideProbes {
    support: Arc<OwnedPeriodicSupportBoundaries>,
    owner: Arc<OwnedPeriodicWireJumps>,
    edge: usize,
    radius: ExactRatio,
    limits: ParameterOrderLimits,
    data: periodic_material_sides::ProbeData,
    exterior: [bool; 2],
    natural: Vec<OriginalPeriodicNaturalCarrier>,
}
fn same_wires(support: &OwnedPeriodicSupportBoundaries, owner: &OwnedPeriodicWireJumps) -> bool {
    Arc::ptr_eq(
        support.augmented(),
        owner.owner().owner().seam_contacts().embedding().wires(),
    )
}
fn derive_probes(
    o: &OwnedPeriodicSupportSideProbes,
) -> Result<(
    periodic_material_sides::ProbeData,
    [bool; 2],
    Vec<OriginalPeriodicNaturalCarrier>,
)> {
    let _replay = super::periodic_replay::scope();
    o.support.reverify_source()?;
    if !same_wires(&o.support, &o.owner) {
        return Err(refused(
            "Support probes do not own the augmented original wire census",
        ));
    }
    let data = periodic_material_sides::source_probe_data(
        &o.owner, o.edge, &o.radius, o.limits, true, true,
    )?;
    let graph = o.owner.owner().owner();
    let e = &graph.edges()[o.edge];
    let cover = graph.owner().owner().pieces();
    let chart = cover.chart().map(|d| d.map(|v| Q::from_float(v).unwrap()));
    let wrapped = cover.wrapped();
    let mut exterior = [false; 2];
    for side in 0..2 {
        for axis in 0..2 {
            let q = decode_ratio(&data.1[side][axis])?;
            if !wrapped[axis] && (q < chart[axis][0] || q > chart[axis][1]) {
                exterior[side] = true;
            }
        }
    }
    let mut natural = Vec::new();
    for (carrier, c) in e.carriers.iter().enumerate() {
        for (boundary, b) in o.support.boundaries().iter().enumerate() {
            if c.source == b.source {
                natural.push(OriginalPeriodicNaturalCarrier {
                    boundary,
                    carrier,
                    inward_left: !(b.reversed ^ c.reversed),
                });
            }
        }
    }
    if natural.is_empty() {
        if exterior.iter().any(|v| *v) {
            return Err(refused(
                "An exterior support probe lacks an actual natural source carrier",
            ));
        }
    } else {
        let first = &natural[0];
        let boundary = &o.support.boundaries()[first.boundary];
        if natural.iter().any(|c| {
            let b = &o.support.boundaries()[c.boundary];
            b.fixed_axis != boundary.fixed_axis
                || b.end != boundary.end
                || c.inward_left != first.inward_left
        }) {
            return Err(refused(
                "Natural source carriers disagree on the actual support half-space",
            ));
        }
        let inside = usize::from(!first.inward_left);
        let outside = 1 - inside;
        if exterior[inside] || !exterior[outside] {
            return Err(refused(
                "Natural support probes do not have exactly one proved inward side",
            ));
        }
        if data.3.iter().flatten().any(|k| k != "0") {
            return Err(refused(
                "Natural edge normal unexpectedly needs a periodic chart transport",
            ));
        }
        for axis in 0..2 {
            if !wrapped[axis] {
                let q = decode_ratio(&data.1[outside][axis])?;
                if axis == boundary.fixed_axis {
                    if !(if boundary.end == 0 {
                        q < chart[axis][0]
                    } else {
                        q > chart[axis][1]
                    }) {
                        return Err(refused(
                            "Exterior point is not beyond the owned natural domain side",
                        ));
                    }
                } else if q <= chart[axis][0] || q >= chart[axis][1] {
                    return Err(refused(
                        "Natural support probe crosses an unrelated natural boundary",
                    ));
                }
            }
        }
    }
    Ok((data, exterior, natural))
}
impl OwnedPeriodicSupportSideProbes {
    pub fn support(&self) -> &Arc<OwnedPeriodicSupportBoundaries> {
        &self.support
    }
    pub fn owner(&self) -> &Arc<OwnedPeriodicWireJumps> {
        &self.owner
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn queries(&self) -> &[[ExactRatio; 2]; 2] {
        &self.data.1
    }
    pub fn lifted_queries(&self) -> &[[ExactRatio; 2]; 2] {
        &self.data.2
    }
    pub fn exterior(&self) -> &[bool; 2] {
        &self.exterior
    }
    pub fn natural_carriers(&self) -> &[OriginalPeriodicNaturalCarrier] {
        &self.natural
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (d, e, n) = derive_probes(self)?;
        if d != self.data || e != self.exterior || n != self.natural {
            return Err(refused(
                "Natural support probes, source carriers or half-space ownership changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_support_side_probes(
    support: Arc<OwnedPeriodicSupportBoundaries>,
    owner: Arc<OwnedPeriodicWireJumps>,
    edge: usize,
    radius: ExactRatio,
    limits: ParameterOrderLimits,
) -> Result<OwnedPeriodicSupportSideProbes> {
    let point = std::array::from_fn(|_| std::array::from_fn(|_| ["0".into(), "1".into()]));
    let mut o = OwnedPeriodicSupportSideProbes {
        support,
        owner,
        edge,
        radius,
        limits,
        data: (
            ["0".into(), "1".into()],
            point,
            std::array::from_fn(|_| std::array::from_fn(|_| ["0".into(), "1".into()])),
            std::array::from_fn(|_| std::array::from_fn(|_| "0".into())),
            Vec::new(),
            Vec::new(),
        ),
        exterior: [false; 2],
        natural: Vec::new(),
    };
    let (d, e, n) = derive_probes(&o)?;
    o.data = d;
    o.exterior = e;
    o.natural = n;
    Ok(o)
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportMaterialRule {
    support: Arc<OwnedPeriodicSupportBoundaries>,
    material: Arc<OwnedPeriodicMaterialRule>,
    authored: Vec<OriginalPeriodicOperand>,
    program: Vec<OriginalPeriodicBooleanOp>,
}
fn lifted(
    support: &OwnedPeriodicSupportBoundaries,
    authored: &[OriginalPeriodicOperand],
) -> Result<Vec<OriginalPeriodicOperand>> {
    authored
        .iter()
        .map(|p| {
            if p.wire_weights.len() != support.original().wires().len() {
                return Err(refused(
                    "Support material operand omits authored wire columns",
                ));
            }
            let mut p = p.clone();
            p.wire_weights = support
                .roles()
                .iter()
                .map(|r| match *r {
                    OriginalPeriodicSupportWireRole::Authored { wire } => p.wire_weights[wire],
                    OriginalPeriodicSupportWireRole::NaturalSupport => 0,
                })
                .collect();
            Ok(p)
        })
        .collect()
}
impl OwnedPeriodicSupportMaterialRule {
    pub fn support(&self) -> &Arc<OwnedPeriodicSupportBoundaries> {
        &self.support
    }
    pub fn material(&self) -> &Arc<OwnedPeriodicMaterialRule> {
        &self.material
    }
    pub fn authored_operands(&self) -> &[OriginalPeriodicOperand] {
        &self.authored
    }
    pub fn reverify_source(&self) -> Result<()> {
        self.support.reverify_source()?;
        self.material.reverify_source()?;
        if !same_wires(&self.support, self.material.base().owner())
            || self.material.operands() != lifted(&self.support, &self.authored)?
            || self.material.program() != self.program
        {
            return Err(refused(
                "Support rule changes authored operands, auxiliary zero roles, program or actual source owner",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_support_material_rule(
    support: Arc<OwnedPeriodicSupportBoundaries>,
    base: Arc<OwnedPeriodicRelativeWinding>,
    authored: &[OriginalPeriodicOperand],
    program: &[OriginalPeriodicBooleanOp],
) -> Result<OwnedPeriodicSupportMaterialRule> {
    support.reverify_source()?;
    let operands = lifted(&support, authored)?;
    let material = Arc::new(own_periodic_material_rule(base, &operands, program)?);
    let o = OwnedPeriodicSupportMaterialRule {
        support,
        material,
        authored: authored.to_vec(),
        program: program.to_vec(),
    };
    o.reverify_source()?;
    Ok(o)
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportSides {
    probes: Arc<OwnedPeriodicSupportSideProbes>,
    rule: Arc<OwnedPeriodicSupportMaterialRule>,
    samples: [Option<Arc<OwnedPeriodicMaterialSample>>; 2],
    interior: Option<Arc<OwnedPeriodicMaterialSides>>,
    operands: [Option<Vec<bool>>; 2],
    selected: [bool; 2],
    direction: Option<i8>,
}
type SideData = ([Option<Vec<bool>>; 2], [bool; 2], Option<i8>);
fn derive_sides(o: &OwnedPeriodicSupportSides) -> Result<SideData> {
    let _replay = super::periodic_replay::scope();
    o.rule.reverify_source()?;
    o.probes.reverify_source()?;
    if !Arc::ptr_eq(o.rule.support(), o.probes.support())
        || !Arc::ptr_eq(o.rule.material().base().owner(), o.probes.owner())
    {
        return Err(refused(
            "Support side labels use a different original source context",
        ));
    }
    let mut operands: [Option<Vec<bool>>; 2] = [None, None];
    let mut selected = [false; 2];
    for side in 0..2 {
        if o.probes.exterior()[side] {
            if o.samples[side].is_some() {
                return Err(refused(
                    "An exterior support point cannot carry an authored material sample",
                ));
            }
        } else {
            let s = o.samples[side]
                .as_ref()
                .ok_or_else(|| refused("An inward support probe has no actual material sample"))?;
            s.reverify_source()?;
            if !Arc::ptr_eq(s.rule(), o.rule.material())
                || !Arc::ptr_eq(s.query().owner(), o.probes.owner())
            {
                return Err(refused(
                    "Support material sample belongs to a different rule or source owner",
                ));
            }
            let p = s
                .query()
                .path()
                .last()
                .ok_or_else(|| refused("Support material query has no endpoint"))?;
            for a in 0..2 {
                if decode_ratio(&p[a])? != decode_ratio(&o.probes.queries()[side][a])? {
                    return Err(refused(
                        "Support material query misses the proved original inward probe",
                    ));
                }
            }
            operands[side] = Some(s.operand_inside().to_vec());
            selected[side] = s.selected();
        }
    }
    if o.probes.exterior().iter().any(|e| *e) {
        if o.interior.is_some() {
            return Err(refused(
                "A natural exterior was relabelled as an ordinary two-sided material proof",
            ));
        }
    } else {
        let pair = o.interior.as_ref().ok_or_else(|| {
            refused("Interior support edge lacks the original transported material jump proof")
        })?;
        pair.reverify_source()?;
        if pair.probes().edge() != o.probes.edge()
            || !Arc::ptr_eq(pair.probes().owner(), o.probes.owner())
            || pair.probes().queries() != o.probes.queries()
        {
            return Err(refused(
                "Interior support side proof uses unrelated physical probes",
            ));
        }
        for side in 0..2 {
            if !Arc::ptr_eq(&pair.samples()[side], o.samples[side].as_ref().unwrap()) {
                return Err(refused(
                    "Interior support side proof uses unrelated material samples",
                ));
            }
        }
    }
    let direction = match (selected[0], selected[1]) {
        (true, false) => Some(1),
        (false, true) => Some(-1),
        _ => None,
    };
    Ok((operands, selected, direction))
}
impl OwnedPeriodicSupportSides {
    pub fn probes(&self) -> &Arc<OwnedPeriodicSupportSideProbes> {
        &self.probes
    }
    pub fn rule(&self) -> &Arc<OwnedPeriodicSupportMaterialRule> {
        &self.rule
    }
    pub fn samples(&self) -> &[Option<Arc<OwnedPeriodicMaterialSample>>; 2] {
        &self.samples
    }
    /// None is outside the supporting domain, NOT all-false authored operands.
    pub fn operand_labels(&self) -> &[Option<Vec<bool>>; 2] {
        &self.operands
    }
    pub fn selected(&self) -> &[bool; 2] {
        &self.selected
    }
    pub fn boundary_direction(&self) -> Option<i8> {
        self.direction
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (a, b, c) = derive_sides(self)?;
        if a != self.operands || b != self.selected || c != self.direction {
            return Err(refused(
                "Support clipping labels, exterior or selected direction changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_support_sides(
    probes: Arc<OwnedPeriodicSupportSideProbes>,
    rule: Arc<OwnedPeriodicSupportMaterialRule>,
    samples: [Option<Arc<OwnedPeriodicMaterialSample>>; 2],
) -> Result<OwnedPeriodicSupportSides> {
    let interior = if probes.exterior().iter().any(|e| *e) {
        None
    } else {
        let p = Arc::new(own_periodic_physical_side_probes(
            probes.owner().clone(),
            probes.edge(),
            probes.radius.clone(),
            probes.limits,
        )?);
        let a = samples[0]
            .as_ref()
            .ok_or_else(|| refused("Missing first interior support sample"))?
            .clone();
        let b = samples[1]
            .as_ref()
            .ok_or_else(|| refused("Missing second interior support sample"))?
            .clone();
        Some(Arc::new(own_periodic_material_sides(p, [a, b])?))
    };
    let mut o = OwnedPeriodicSupportSides {
        probes,
        rule,
        samples,
        interior,
        operands: [None, None],
        selected: [false; 2],
        direction: None,
    };
    let (a, b, c) = derive_sides(&o)?;
    o.operands = a;
    o.selected = b;
    o.direction = c;
    Ok(o)
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportSelection {
    rule: Arc<OwnedPeriodicSupportMaterialRule>,
    sides: Vec<Arc<OwnedPeriodicSupportSides>>,
    selected: Vec<OriginalPeriodicSelectedEdge>,
}
fn derive_selection(
    o: &OwnedPeriodicSupportSelection,
) -> Result<Vec<OriginalPeriodicSelectedEdge>> {
    let _replay = super::periodic_replay::scope();
    o.rule.reverify_source()?;
    let graph = o.rule.material().base().owner().owner().owner();
    if o.sides.len() != graph.edges().len() || o.sides.len() > 1000000 {
        return Err(refused(
            "Support boundary selection omits physical edge side proofs",
        ));
    }
    let mut selected = Vec::new();
    let mut flow = vec![0i64; graph.vertices().len()];
    for (edge, s) in o.sides.iter().enumerate() {
        s.reverify_source()?;
        if s.probes().edge() != edge || !Arc::ptr_eq(s.rule(), &o.rule) {
            return Err(refused(
                "Support boundary selection repeats or misbinds an original edge",
            ));
        }
        if let Some(direction) = s.boundary_direction() {
            let ends = graph.edges()[edge].vertices;
            flow[ends[0]] += i64::from(direction);
            flow[ends[1]] -= i64::from(direction);
            selected.push(OriginalPeriodicSelectedEdge { edge, direction });
        }
    }
    if flow.iter().any(|n| *n != 0) {
        return Err(refused(
            "Support-clipped selected boundary is not conserved at actual vertices",
        ));
    }
    Ok(selected)
}
impl OwnedPeriodicSupportSelection {
    pub fn rule(&self) -> &Arc<OwnedPeriodicSupportMaterialRule> {
        &self.rule
    }
    pub fn sides(&self) -> &[Arc<OwnedPeriodicSupportSides>] {
        &self.sides
    }
    pub fn selected(&self) -> &[OriginalPeriodicSelectedEdge] {
        &self.selected
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive_selection(self)? != self.selected {
            return Err(refused(
                "Complete support-clipped original boundary selection changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_support_selection(
    rule: Arc<OwnedPeriodicSupportMaterialRule>,
    sides: &[Arc<OwnedPeriodicSupportSides>],
) -> Result<OwnedPeriodicSupportSelection> {
    let mut o = OwnedPeriodicSupportSelection {
        rule,
        sides: sides.to_vec(),
        selected: Vec::new(),
    };
    o.selected = derive_selection(&o)?;
    Ok(o)
}
#[cfg(test)]
pub(in crate::profile_certificate::span_intersections::translated) mod tests {
    use super::*;
    pub(in crate::profile_certificate::span_intersections::translated) fn full_plane(
        complement: bool,
    ) -> Arc<OwnedPeriodicSupportSelection> {
        plane_selection(0, complement)
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn with_hole()
    -> Arc<OwnedPeriodicSupportSelection> {
        plane_selection(1, true)
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn tangent_hole()
    -> Arc<OwnedPeriodicSupportSelection> {
        plane_selection(2, true)
    }
    fn plane_selection(shape: usize, complement: bool) -> Arc<OwnedPeriodicSupportSelection> {
        let surface = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let curves = match shape {
            0 => vec![],
            1 => vec![
                Curve::from_polyline(vec![
                    vec![0.25, 0.25],
                    vec![0.75, 0.25],
                    vec![0.75, 0.75],
                    vec![0.25, 0.75],
                    vec![0.25, 0.25],
                ])
                .unwrap(),
            ],
            _ => vec![Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
                control_points: vec![
                    vec![0.75, 0.25],
                    vec![0.75, 0.5],
                    vec![0.5, 0.5],
                    vec![0.25, 0.5],
                    vec![0.25, 0.25],
                    vec![0.25, 0.],
                    vec![0.5, 0.],
                    vec![0.75, 0.],
                    vec![0.75, 0.25],
                ],
                weights: vec![
                    1.,
                    std::f64::consts::FRAC_1_SQRT_2,
                    1.,
                    std::f64::consts::FRAC_1_SQRT_2,
                    1.,
                    std::f64::consts::FRAC_1_SQRT_2,
                    1.,
                    std::f64::consts::FRAC_1_SQRT_2,
                    1.,
                ],
                periodic: false,
            }],
        };
        let uses = if shape == 0 {
            vec![]
        } else {
            vec![vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]]
        };
        let input = super::super::periodic_support_boundaries::tests::original(
            &surface,
            [false, false],
            &curves,
            &uses,
        );
        let support = Arc::new(
            own_periodic_support_boundaries(
                input,
                1e-12,
                ParameterOrderLimits::default(),
                64,
                100000,
            )
            .unwrap(),
        );
        let jumps =
            super::super::periodic_support_boundaries::tests::graph(support.augmented().clone());
        let a = [["1".into(), "8".into()], ["1".into(), "8".into()]];
        let base = Arc::new(
            own_periodic_relative_winding(
                jumps.clone(),
                &[a.clone(), a.clone()],
                ParameterOrderLimits::default(),
                1000,
            )
            .unwrap(),
        );
        let program = if complement {
            vec![
                OriginalPeriodicBooleanOp::Operand(0),
                OriginalPeriodicBooleanOp::Complement(0),
            ]
        } else {
            vec![OriginalPeriodicBooleanOp::Operand(0)]
        };
        let rule = Arc::new(
            own_periodic_support_material_rule(
                support.clone(),
                base,
                &[OriginalPeriodicOperand {
                    wire_weights: if shape == 0 { vec![] } else { vec![1] },
                    base_winding: "0".into(),
                    fill: OriginalPeriodicFillRule::NonZero,
                }],
                &program,
            )
            .unwrap(),
        );
        let mut sides = Vec::new();
        for edge in 0..jumps.owner().owner().edges().len() {
            let probes = Arc::new(
                own_periodic_support_side_probes(
                    support.clone(),
                    jumps.clone(),
                    edge,
                    ["1".into(), "1024".into()],
                    ParameterOrderLimits::default(),
                )
                .unwrap(),
            );
            assert_eq!(
                probes.exterior().iter().filter(|e| **e).count(),
                usize::from(!probes.natural_carriers().is_empty())
            );
            let samples = std::array::from_fn(|side| {
                if probes.exterior()[side] {
                    None
                } else {
                    Some(Arc::new(
                        own_periodic_material_sample(
                            rule.material().clone(),
                            Arc::new(
                                own_periodic_relative_winding(
                                    jumps.clone(),
                                    &[a.clone(), probes.queries()[side].clone()],
                                    ParameterOrderLimits::default(),
                                    1000,
                                )
                                .unwrap(),
                            ),
                        )
                        .unwrap(),
                    ))
                }
            });
            let s = Arc::new(own_periodic_support_sides(probes, rule.clone(), samples).unwrap());
            for side in 0..2 {
                assert_eq!(
                    s.operand_labels()[side].is_none(),
                    s.probes().exterior()[side]
                );
                if s.probes().exterior()[side] {
                    assert!(!s.selected()[side]);
                } else if shape == 0 {
                    assert_eq!(s.selected()[side], complement);
                }
            }
            sides.push(s);
        }
        Arc::new(own_periodic_support_selection(rule, &sides).unwrap())
    }
    #[test]
    fn authored_hole_keeps_ordinary_interior_jump_proofs_and_selects_both_real_boundaries() {
        let o = with_hole();
        assert_eq!(o.selected().len(), 8);
        let inner = o
            .sides()
            .iter()
            .filter(|s| s.probes().natural_carriers().is_empty())
            .collect::<Vec<_>>();
        assert_eq!(inner.len(), 4);
        for s in inner {
            assert!(s.interior.is_some());
            assert_eq!(s.probes().exterior(), &[false, false]);
            assert_eq!(s.boundary_direction(), Some(-1));
        }
        o.reverify_source().unwrap();
    }
    #[test]
    fn complement_is_clipped_to_actual_support_and_never_creates_exterior_material_samples() {
        let o = full_plane(true);
        assert_eq!(o.selected().len(), 4);
        o.reverify_source().unwrap();
        let first = &o.sides()[0];
        let side = first.probes().exterior().iter().position(|e| *e).unwrap();
        assert!(
            own_periodic_physical_side_probes(
                first.probes().owner().clone(),
                0,
                ["1".into(), "64".into()],
                ParameterOrderLimits::default()
            )
            .is_err()
        );
        let mut bad = (**first.probes()).clone();
        bad.exterior[side] = false;
        assert!(bad.reverify_source().is_err());
        let mut bad = (**first).clone();
        bad.operands[side] = Some(vec![false]);
        assert!(bad.reverify_source().is_err());
        let mut samples = first.samples().clone();
        samples[side] = samples[1 - side].clone();
        assert!(
            own_periodic_support_sides(first.probes().clone(), first.rule().clone(), samples)
                .is_err()
        );
        let mut bad = (*o).clone();
        bad.selected.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = (**o.rule()).clone();
        bad.material = Arc::new(
            own_periodic_material_rule(
                bad.material.base().clone(),
                &[OriginalPeriodicOperand {
                    wire_weights: vec![1],
                    base_winding: "0".into(),
                    fill: OriginalPeriodicFillRule::NonZero,
                }],
                &bad.program,
            )
            .unwrap(),
        );
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn empty_authored_fill_does_not_select_auxiliary_support_edges() {
        let o = full_plane(false);
        assert!(o.selected().is_empty());
        o.reverify_source().unwrap();
    }
}
