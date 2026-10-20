//! Exact Boolean sector continuation of selected ORIGINAL physical edge darts.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicMaterialSector {
    pub vertex: usize,
    /// Open sector from the first outgoing germ to the second, counterclockwise.
    pub ccw_ports: [usize; 2],
    pub operand_inside: Vec<bool>,
    pub material: bool,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicMaterialCycles {
    owner: Arc<OwnedPeriodicBoundarySelection>,
    rotation: Arc<OwnedPeriodicRotation>,
    max_refinements: usize,
    max_darts: usize,
    sectors: Vec<Vec<OriginalPeriodicMaterialSector>>,
    successor: Vec<Option<usize>>,
    walks: Vec<Vec<usize>>,
    dart_walks: Vec<Option<usize>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Label {
    operands: Vec<bool>,
    material: bool,
}
type Data = (
    Vec<Vec<OriginalPeriodicMaterialSector>>,
    Vec<Option<usize>>,
    Vec<Vec<usize>>,
    Vec<Option<usize>>,
);
fn connect(
    rows: &[Vec<usize>],
    labels: &[[Label; 2]],
    selected: &[OriginalPeriodicSelectedEdge],
) -> Result<Data> {
    let n = labels
        .len()
        .checked_mul(2)
        .ok_or_else(|| refused("Selected material dart count overflow"))?;
    let mut positions = vec![None; n];
    let mut active = vec![false; n];
    for e in selected {
        if e.edge >= labels.len() || ![-1, 1].contains(&e.direction) {
            return Err(refused("Invalid selected original edge orientation"));
        }
        let dart = e.edge * 2 + usize::from(e.direction < 0);
        if std::mem::replace(&mut active[dart], true) {
            return Err(refused("Repeated selected original edge dart"));
        }
    }
    let side =
        |dart: usize, left: bool| -> &Label { &labels[dart / 2][(dart % 2) ^ usize::from(!left)] };
    for (vertex, row) in rows.iter().enumerate() {
        for (position, &dart) in row.iter().enumerate() {
            if dart >= n || positions[dart].replace((vertex, position)).is_some() {
                return Err(refused(
                    "Material sectors duplicate or misplace a physical dart",
                ));
            }
        }
    }
    if positions.iter().any(Option::is_none) {
        return Err(refused("Material sectors omit original physical darts"));
    }
    for dart in 0..n {
        if active[dart] != (side(dart, true).material && !side(dart, false).material) {
            return Err(refused(
                "Selected material direction disagrees with complete side labels",
            ));
        }
    }
    let mut sectors = Vec::new();
    let mut previous = vec![None; n];
    for (vertex, row) in rows.iter().enumerate() {
        let mut local = Vec::new();
        let mut last = row.iter().rev().copied().find(|&dart| active[dart]);
        for (position, &dart) in row.iter().enumerate() {
            let next = row[(position + 1) % row.len()];
            let left = side(dart, true);
            let right = side(next, false);
            if left != right {
                return Err(refused(
                    "Adjacent original germs disagree on their shared material sector",
                ));
            }
            local.push(OriginalPeriodicMaterialSector {
                vertex,
                ccw_ports: [dart, next],
                operand_inside: left.operands.clone(),
                material: left.material,
            });
            previous[dart] = last;
            if active[dart] {
                last = Some(dart);
            }
        }
        sectors.push(local);
    }
    let mut successor = vec![None; n];
    let mut incoming = vec![0usize; n];
    for dart in 0..n {
        if active[dart] {
            let twin = dart ^ 1;
            let next = previous[twin].ok_or_else(|| {
                refused("Selected material boundary has no outgoing continuation")
            })?;
            if positions[twin].map(|p| p.0) != positions[next].map(|p| p.0) {
                return Err(refused(
                    "Material continuation leaves the actual arrival vertex",
                ));
            }
            successor[dart] = Some(next);
            incoming[next] += 1;
        }
    }
    for dart in 0..n {
        if incoming[dart] != usize::from(active[dart]) {
            return Err(refused(
                "Selected material continuation is not a complete permutation",
            ));
        }
    }
    let mut dart_walks = vec![None; n];
    let mut walks = Vec::new();
    for start in 0..n {
        if active[start] && dart_walks[start].is_none() {
            let mut walk = Vec::new();
            let mut current = start;
            while dart_walks[current].is_none() {
                dart_walks[current] = Some(walks.len());
                walk.push(current);
                current = successor[current]
                    .ok_or_else(|| refused("Material walk enters an unselected source dart"))?;
            }
            if current != start {
                return Err(refused(
                    "Material boundary walk merges into a different orbit",
                ));
            }
            walks.push(walk);
        }
    }
    Ok((sectors, successor, walks, dart_walks))
}
/// Source-domain support labels: None is exterior, never an all-false vector.
/// This shares only the pure labelled-permutation kernel; callers must replay
/// the distinct support/half-disc owner before invoking it.
pub(in super::super) fn connect_support(
    rows: &[Vec<usize>],
    operands: &[[Option<Vec<bool>>; 2]],
    material: &[[bool; 2]],
    selected: &[OriginalPeriodicSelectedEdge],
) -> Result<Data> {
    if operands.len() != material.len() {
        return Err(refused("Support sector labels omit physical edges"));
    }
    let mut labels = Vec::new();
    for (a, b) in operands.iter().zip(material) {
        for side in 0..2 {
            if a[side].as_ref().is_some_and(Vec::is_empty) || (a[side].is_none() && b[side]) {
                return Err(refused(
                    "Support exterior is not an authored operand value or material",
                ));
            }
        }
        labels.push(std::array::from_fn(|side| Label {
            operands: a[side].clone().unwrap_or_default(),
            material: b[side],
        }));
    }
    connect(rows, &labels, selected)
}
fn natural_interior(
    use_: &OriginalPeriodicVertexChartUse,
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    refinements: usize,
) -> Result<()> {
    let f = decode_polynomial(&use_.parameter.primitive)?;
    let root = [
        decode_ratio(&use_.parameter.interval[0])?,
        decode_ratio(&use_.parameter.interval[1])?,
    ];
    for axis in 0..2 {
        if !wrapped[axis] {
            let num = decode_polynomial(&use_.position[axis][0])?;
            let den = decode_polynomial(&use_.position[axis][1])?;
            let sign_den = arithmetic::event_coordinates::sign_at(&den, &f, &root, refinements)?;
            if sign_den == 0 {
                return Err(refused(
                    "Material vertex has an undefined original chart coordinate",
                ));
            }
            for end in 0..2 {
                let level = Q::from_float(chart[axis][end])
                    .ok_or_else(|| refused("Invalid actual natural chart boundary"))?;
                let mut p = num.clone();
                p.resize(p.len().max(den.len()), Q::zero());
                for (i, d) in den.iter().enumerate() {
                    p[i] -= &level * d;
                }
                while p.last().is_some_and(Q::is_zero) {
                    p.pop();
                }
                let sign =
                    arithmetic::event_coordinates::sign_at(&p, &f, &root, refinements)? * sign_den;
                if sign != if end == 0 { 1 } else { -1 } {
                    return Err(refused(
                        "Material vertex needs an owned natural support boundary sector",
                    ));
                }
            }
        }
    }
    Ok(())
}
fn derive(o: &OwnedPeriodicMaterialCycles) -> Result<Data> {
    let charts = o.owner.rule().base().owner().owner();
    let graph = charts.owner();
    let n = graph
        .edges()
        .len()
        .checked_mul(2)
        .ok_or_else(|| refused("Complete selected dart allowance overflow"))?;
    if !(1..=1000000).contains(&o.max_darts) || n > o.max_darts || o.max_refinements > 512 {
        return Err(refused("Invalid complete material sector/dart allowance"));
    }
    o.owner.reverify_source()?;
    o.rotation.reverify_source()?;
    if !Arc::ptr_eq(graph, o.rotation.owner().owner().owner()) {
        return Err(refused(
            "Material rotation belongs to a different original physical chart owner",
        ));
    }
    let cover = graph.owner().owner().pieces();
    for chart in charts.charts() {
        let use_ = chart
            .uses
            .first()
            .ok_or_else(|| refused("Material vertex has no original chart point"))?;
        natural_interior(use_, cover.chart(), cover.wrapped(), o.max_refinements)?;
    }
    let ports = o.rotation.owner().ports();
    if ports.len() != n {
        return Err(refused(
            "Material rotation omits original physical edge endpoints",
        ));
    }
    for (dart, p) in ports.iter().enumerate() {
        if p.end > 1
            || p.edge.checked_mul(2).and_then(|n| n.checked_add(p.end)) != Some(dart)
            || p.vertex != graph.edges()[p.edge].vertices[p.end]
        {
            return Err(refused(
                "Material germ is not its original physical endpoint dart",
            ));
        }
    }
    let labels = o
        .owner
        .sides()
        .iter()
        .map(|s| {
            s.samples().each_ref().map(|s| Label {
                operands: s.operand_inside().to_vec(),
                material: s.selected(),
            })
        })
        .collect::<Vec<_>>();
    connect(o.rotation.rotations(), &labels, o.owner.selected())
}
impl OwnedPeriodicMaterialCycles {
    pub fn owner(&self) -> &Arc<OwnedPeriodicBoundarySelection> {
        &self.owner
    }
    pub fn rotation(&self) -> &Arc<OwnedPeriodicRotation> {
        &self.rotation
    }
    pub fn sectors(&self) -> &[Vec<OriginalPeriodicMaterialSector>] {
        &self.sectors
    }
    pub fn successor(&self) -> &[Option<usize>] {
        &self.successor
    }
    /// Material-left selected ORIGINAL boundary walks. They may touch/repeat a
    /// physical vertex and are NOT connected faces, holes or closed UV lifts.
    pub fn walks(&self) -> &[Vec<usize>] {
        &self.walks
    }
    pub fn dart_walks(&self) -> &[Option<usize>] {
        &self.dart_walks
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (s, n, w, d) = derive(self)?;
        if s != self.sectors || n != self.successor || w != self.walks || d != self.dart_walks {
            return Err(refused(
                "Original material sectors, selected permutation or boundary walks changed",
            ));
        }
        Ok(())
    }
}
/// Connect exactly the selected original darts through their actual Boolean
/// sectors; unchanged-material internal edges are skipped. Neither natural
/// support-boundary sectors nor connected-region/face ownership are inferred.
pub fn own_periodic_material_cycles(
    owner: Arc<OwnedPeriodicBoundarySelection>,
    rotation: Arc<OwnedPeriodicRotation>,
    max_refinements: usize,
    max_darts: usize,
) -> Result<OwnedPeriodicMaterialCycles> {
    let mut o = OwnedPeriodicMaterialCycles {
        owner,
        rotation,
        max_refinements,
        max_darts,
        sectors: Vec::new(),
        successor: Vec::new(),
        walks: Vec::new(),
        dart_walks: Vec::new(),
    };
    let (s, n, w, d) = derive(&o)?;
    o.sectors = s;
    o.successor = n;
    o.walks = w;
    o.dart_walks = d;
    Ok(o)
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicMaterialLifts {
    owner: Arc<OwnedPeriodicMaterialCycles>,
    max_refinements: usize,
    max_parts: usize,
    walks: Vec<OriginalPeriodicBoundaryWalk>,
    parts_count: usize,
}
fn derive_lifts(
    o: &OwnedPeriodicMaterialLifts,
) -> Result<(Vec<OriginalPeriodicBoundaryWalk>, usize)> {
    if o.max_refinements > 512 || !(1..=1000000).contains(&o.max_parts) {
        return Err(refused("Invalid complete selected source lift allowance"));
    }
    o.owner.reverify_source()?;
    let expected = o
        .owner
        .dart_walks()
        .iter()
        .map(Option::is_some)
        .collect::<Vec<_>>();
    periodic_boundary_lifts::lift_walks(
        o.owner.rotation().owner(),
        o.owner.walks(),
        o.max_refinements,
        o.max_parts,
        &expected,
    )
}
impl OwnedPeriodicMaterialLifts {
    pub fn owner(&self) -> &Arc<OwnedPeriodicMaterialCycles> {
        &self.owner
    }
    pub fn walks(&self) -> &[OriginalPeriodicBoundaryWalk] {
        &self.walks
    }
    pub fn parts_count(&self) -> usize {
        self.parts_count
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (w, n) = derive_lifts(self)?;
        if w != self.walks || n != self.parts_count {
            return Err(refused(
                "Selected original material lifts or native source coverage changed",
            ));
        }
        Ok(())
    }
}
/// Lift ONLY selected material walks; every admitted dart is retained once and
/// every excluded dart stays excluded. Nonzero homology is kept explicit,
/// without claiming a closed UV loop, connected region or physical face.
pub fn own_periodic_material_lifts(
    owner: Arc<OwnedPeriodicMaterialCycles>,
    max_refinements: usize,
    max_parts: usize,
) -> Result<OwnedPeriodicMaterialLifts> {
    let mut o = OwnedPeriodicMaterialLifts {
        owner,
        max_refinements,
        max_parts,
        walks: Vec::new(),
        parts_count: 0,
    };
    let (w, n) = derive_lifts(&o)?;
    o.walks = w;
    o.parts_count = n;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn label(value: bool) -> Label {
        Label {
            operands: vec![value],
            material: value,
        }
    }
    #[test]
    fn touching_lobes_keep_separate_material_walks_and_skip_an_internal_edge() {
        let rows = [vec![0, 4, 5, 1, 2, 3]];
        let labels = [
            [label(true), label(false)],
            [label(true), label(false)],
            [label(true), label(true)],
        ];
        let edges = [
            OriginalPeriodicSelectedEdge {
                edge: 0,
                direction: 1,
            },
            OriginalPeriodicSelectedEdge {
                edge: 1,
                direction: 1,
            },
        ];
        let (sectors, next, walks, owners) = connect(&rows, &labels, &edges).unwrap();
        assert_eq!(walks, [vec![0], vec![2]]);
        assert_eq!(next, [Some(0), None, Some(2), None, None, None]);
        assert_eq!(owners, [Some(0), None, Some(1), None, None, None]);
        assert_eq!(sectors[0].len(), 6);
        let exterior = labels.clone().map(|s| s.map(|l| label(!l.material)));
        let edges = [
            OriginalPeriodicSelectedEdge {
                edge: 0,
                direction: -1,
            },
            OriginalPeriodicSelectedEdge {
                edge: 1,
                direction: -1,
            },
        ];
        assert_eq!(connect(&rows, &exterior, &edges).unwrap().2, [vec![1, 3]]);
        let full = std::array::from_fn::<_, 3, _>(|_| [label(true), label(true)]);
        let full = connect(&rows, &full, &[]).unwrap();
        assert!(full.2.is_empty());
        assert!(full.0.iter().flatten().all(|s| s.material));
        let mut bad = labels.clone();
        bad[2][1].operands = vec![false];
        assert!(
            connect(
                &rows,
                &bad,
                &[
                    OriginalPeriodicSelectedEdge {
                        edge: 0,
                        direction: 1
                    },
                    OriginalPeriodicSelectedEdge {
                        edge: 1,
                        direction: 1
                    }
                ]
            )
            .is_err()
        );
        assert!(connect(&[vec![0, 4, 5, 1, 2]], &labels, &[]).is_err());
        assert!(connect(&[vec![0, 4, 5, 1, 2, 2]], &labels, &[]).is_err());
        assert!(connect(&rows, &labels, &[]).is_err());
    }
    #[test]
    fn natural_vertex_sector_guard_uses_original_selected_fields_and_signed_denominators() {
        let q = |n: i64| [n.to_string(), "1".to_string()];
        let poly = |v: &[i64]| v.iter().copied().map(q).collect::<Vec<_>>();
        let mut p = OriginalPeriodicVertexChartUse {
            source: OriginalPeriodicChartCutRef { piece: 0, cut: 0 },
            period_shift: ["0".into(), "0".into()],
            translation: [q(0), q(0)],
            parent: None,
            parameter: OriginalSourceParameter {
                primitive: poly(&[-2, 0, 1]),
                interval: [q(1), q(2)],
                parameter: poly(&[0, 1]),
            },
            position: [
                [poly(&[0, 1]), poly(&[1])],
                [vec![["-1".into(), "2".into()]], poly(&[-1])],
            ],
        };
        natural_interior(&p, [[0., 2.], [0., 1.]], [false, false], 192).unwrap();
        assert!(natural_interior(&p, [[0., 1.], [0., 1.]], [false, false], 192).is_err());
        p.position[1][0] = poly(&[0]);
        assert!(natural_interior(&p, [[0., 2.], [0., 1.]], [true, false], 192).is_err());
        p.position[1][1] = poly(&[0]);
        assert!(natural_interior(&p, [[0., 2.], [0., 1.]], [true, false], 192).is_err());
    }
    #[test]
    fn actual_essential_periodic_loop_has_one_selected_orbit_and_empty_boolean_keeps_no_walks() {
        let curve = Curve::from_polyline(vec![vec![0., 0.5], vec![4., 0.5]]).unwrap();
        let jumps = Arc::new(super::super::periodic_wire_jumps::tests::own(
            &[curve],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        ));
        let ports = Arc::new(own_periodic_ports(jumps.owner().clone(), 192).unwrap());
        let rotation =
            Arc::new(own_periodic_rotation(ports, ContactPortOrderLimits::default()).unwrap());
        let a = [["1".into(), "1".into()], ["1".into(), "8".into()]];
        let base = Arc::new(
            own_periodic_relative_winding(
                jumps.clone(),
                &[a.clone(), a.clone()],
                ParameterOrderLimits::default(),
                64,
            )
            .unwrap(),
        );
        let operands = [OriginalPeriodicOperand {
            wire_weights: vec![1],
            base_winding: "0".into(),
            fill: OriginalPeriodicFillRule::NonZero,
        }];
        for empty in [false, true] {
            let program = if empty {
                vec![
                    OriginalPeriodicBooleanOp::Operand(0),
                    OriginalPeriodicBooleanOp::Difference(0, 0),
                ]
            } else {
                vec![OriginalPeriodicBooleanOp::Operand(0)]
            };
            let rule =
                Arc::new(own_periodic_material_rule(base.clone(), &operands, &program).unwrap());
            let probes = Arc::new(
                own_periodic_physical_side_probes(
                    jumps.clone(),
                    0,
                    ["1".into(), "64".into()],
                    ParameterOrderLimits::default(),
                )
                .unwrap(),
            );
            let samples = probes.queries().clone().map(|p| {
                Arc::new(
                    own_periodic_material_sample(
                        rule.clone(),
                        Arc::new(
                            own_periodic_relative_winding(
                                jumps.clone(),
                                &[a.clone(), p],
                                ParameterOrderLimits::default(),
                                64,
                            )
                            .unwrap(),
                        ),
                    )
                    .unwrap(),
                )
            });
            let sides = Arc::new(own_periodic_material_sides(probes, samples).unwrap());
            let selection = Arc::new(own_periodic_boundary_selection(rule, &[sides]).unwrap());
            assert!(
                own_periodic_material_cycles(selection.clone(), rotation.clone(), 192, 1).is_err()
            );
            let cycles =
                own_periodic_material_cycles(selection, rotation.clone(), 192, 64).unwrap();
            cycles.reverify_source().unwrap();
            assert_eq!(cycles.walks().len(), usize::from(!empty));
            let lifts = own_periodic_material_lifts(Arc::new(cycles.clone()), 192, 1).unwrap();
            assert_eq!(lifts.parts_count(), usize::from(!empty));
            if !empty {
                assert_eq!(lifts.walks()[0].homology, ["1", "0"]);
                assert!(!lifts.walks()[0].is_closed_uv());
                assert_eq!(
                    lifts.walks()[0].net_translation,
                    [
                        ["4".to_string(), "1".to_string()],
                        ["0".to_string(), "1".to_string()]
                    ]
                );
                let mut bad = lifts.clone();
                bad.parts_count += 1;
                assert!(bad.reverify_source().is_err());
            }

            if !empty {
                assert_eq!(cycles.walks(), &[vec![0]]);
                let mut bad = cycles.clone();
                bad.successor[0] = Some(1);
                assert!(bad.reverify_source().is_err());
                let mut bad = cycles.clone();
                bad.walks.clear();
                assert!(bad.reverify_source().is_err());
                let mut bad = cycles.clone();
                bad.sectors[0][0].material = !bad.sectors[0][0].material;
                assert!(bad.reverify_source().is_err());
            }
        }
    }
}
