//! Natural half-disc/corner sectors and selected support-clipped source walks.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSupportVertex {
    pub vertex: usize,
    /// None means interior (or a wrapped axis); Some(0/1) is a natural side.
    pub natural_bounds: [Option<usize>; 2],
    pub natural_ports: [Vec<usize>; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSupportSector {
    pub vertex: usize,
    pub ccw_ports: [usize; 2],
    /// None is outside support, not an all-false authored value.
    pub operand_inside: Option<Vec<bool>>,
    pub material: bool,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportCycles {
    owner: Arc<OwnedPeriodicSupportSelection>,
    rotation: Arc<OwnedPeriodicRotation>,
    max_refinements: usize,
    max_darts: usize,
    vertices: Vec<OriginalPeriodicSupportVertex>,
    sectors: Vec<Vec<OriginalPeriodicSupportSector>>,
    successor: Vec<Option<usize>>,
    walks: Vec<Vec<usize>>,
    dart_walks: Vec<Option<usize>>,
}
fn bounds(
    use_: &OriginalPeriodicVertexChartUse,
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    refinements: usize,
) -> Result<[Option<usize>; 2]> {
    let f = decode_polynomial(&use_.parameter.primitive)?;
    let root = [
        decode_ratio(&use_.parameter.interval[0])?,
        decode_ratio(&use_.parameter.interval[1])?,
    ];
    let mut result = [None, None];
    for axis in 0..2 {
        if !wrapped[axis] {
            let n = decode_polynomial(&use_.position[axis][0])?;
            let d = decode_polynomial(&use_.position[axis][1])?;
            let denominator = arithmetic::event_coordinates::sign_at(&d, &f, &root, refinements)?;
            if denominator == 0 {
                return Err(refused(
                    "Support vertex has an undefined original coordinate",
                ));
            }
            for end in 0..2 {
                let level = Q::from_float(chart[axis][end])
                    .ok_or_else(|| refused("Invalid actual natural domain bound"))?;
                let mut p = n.clone();
                p.resize(p.len().max(d.len()), Q::zero());
                for (i, d) in d.iter().enumerate() {
                    p[i] -= &level * d;
                }
                while p.last().is_some_and(Q::is_zero) {
                    p.pop();
                }
                let sign = arithmetic::event_coordinates::sign_at(&p, &f, &root, refinements)?
                    * denominator;
                if (end == 0 && sign < 0) || (end == 1 && sign > 0) {
                    return Err(refused(
                        "Support vertex escaped its actual natural source domain",
                    ));
                }
                if sign == 0 {
                    if result[axis].replace(end).is_some() {
                        return Err(refused("Support vertex has a degenerate natural axis"));
                    }
                }
            }
        }
    }
    Ok(result)
}
type Data = (
    Vec<OriginalPeriodicSupportVertex>,
    Vec<Vec<OriginalPeriodicSupportSector>>,
    Vec<Option<usize>>,
    Vec<Vec<usize>>,
    Vec<Option<usize>>,
);
fn derive(o: &OwnedPeriodicSupportCycles) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    let jumps = o.owner.rule().material().base().owner();
    let graph = jumps.owner().owner();
    let n = graph
        .edges()
        .len()
        .checked_mul(2)
        .ok_or_else(|| refused("Support cycle dart count overflow"))?;
    if !(1..=1000000).contains(&o.max_darts) || n > o.max_darts || o.max_refinements > 512 {
        return Err(refused("Invalid complete support sector/dart allowance"));
    }
    super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
    super::periodic_replay::verify_arc(&o.rotation, |rotation| rotation.reverify_source())?;
    let ports = o.rotation.owner().ports();
    let charts = o.rotation.owner().owner();
    let rows = o.rotation.rotations();
    let cover = graph.owner().owner().pieces();
    if !Arc::ptr_eq(graph, charts.owner())
        || ports.len() != n
        || rows.len() != charts.charts().len()
    {
        return Err(refused(
            "Support sectors use a different actual physical graph or incomplete rotation",
        ));
    }
    let support = o.owner.rule().support();
    let mut vertices = Vec::new();
    for (vertex, row) in rows.iter().enumerate() {
        let use_ = charts.charts()[vertex]
            .uses
            .first()
            .ok_or_else(|| refused("Support sector lacks an original vertex point"))?;
        let natural_bounds = bounds(use_, cover.chart(), cover.wrapped(), o.max_refinements)?;
        let mut natural_ports: [Vec<usize>; 2] = std::array::from_fn(|_| Vec::new());
        for &dart in row {
            let p = ports
                .get(dart)
                .ok_or_else(|| refused("Support vertex names a missing source germ"))?;
            if p.vertex != vertex
                || p.end > 1
                || p.edge.checked_mul(2).and_then(|v| v.checked_add(p.end)) != Some(dart)
            {
                return Err(refused(
                    "Support germ is not its original physical endpoint dart",
                ));
            }
            for axis in 0..2 {
                let carriers = o.owner.sides()[p.edge].probes().natural_carriers();
                let matching = carriers
                    .iter()
                    .filter(|c| support.boundaries()[c.boundary].fixed_axis == axis)
                    .collect::<Vec<_>>();
                if !matching.is_empty() {
                    let end = natural_bounds[axis].ok_or_else(|| {
                        refused("Natural port is not on its owned source domain side")
                    })?;
                    if matching
                        .iter()
                        .any(|c| support.boundaries()[c.boundary].end != end)
                    {
                        return Err(refused(
                            "Natural source port is assigned to the opposite support boundary",
                        ));
                    }
                    natural_ports[axis].push(dart);
                }
                if let Some(end) = natural_bounds[axis] {
                    if (end == 0 && p.component_signs[axis] < 0)
                        || (end == 1 && p.component_signs[axis] > 0)
                    {
                        return Err(refused(
                            "An original one-sided germ leaves the closed supporting domain",
                        ));
                    }
                }
            }
        }
        for axis in 0..2 {
            let required = if natural_bounds[axis].is_none() {
                0
            } else if natural_bounds[1 - axis].is_some() {
                1
            } else {
                2
            };
            if natural_ports[axis].len() != required {
                return Err(refused(
                    "Support vertex does not own its complete natural half-disc/corner ports",
                ));
            }
        }
        vertices.push(OriginalPeriodicSupportVertex {
            vertex,
            natural_bounds,
            natural_ports,
        });
    }
    let operands = o
        .owner
        .sides()
        .iter()
        .map(|s| s.operand_labels().clone())
        .collect::<Vec<_>>();
    let material = o
        .owner
        .sides()
        .iter()
        .map(|s| *s.selected())
        .collect::<Vec<_>>();
    let (raw, next, walks, dart_walks) =
        periodic_material_cycles::connect_support(rows, &operands, &material, o.owner.selected())?;
    let sectors = raw
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|s| OriginalPeriodicSupportSector {
                    vertex: s.vertex,
                    ccw_ports: s.ccw_ports,
                    operand_inside: (!s.operand_inside.is_empty()).then_some(s.operand_inside),
                    material: s.material,
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (vertex, row) in sectors.iter().enumerate() {
        let expected = usize::from(vertices[vertex].natural_bounds.iter().any(Option::is_some));
        if row.iter().filter(|s| s.operand_inside.is_none()).count() != expected {
            return Err(refused(
                "Natural vertex outside sectors do not form exactly one physical exterior gap",
            ));
        }
    }
    Ok((vertices, sectors, next, walks, dart_walks))
}
impl OwnedPeriodicSupportCycles {
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportSelection> {
        &self.owner
    }
    pub fn rotation(&self) -> &Arc<OwnedPeriodicRotation> {
        &self.rotation
    }
    pub fn vertices(&self) -> &[OriginalPeriodicSupportVertex] {
        &self.vertices
    }
    pub fn sectors(&self) -> &[Vec<OriginalPeriodicSupportSector>] {
        &self.sectors
    }
    pub fn successor(&self) -> &[Option<usize>] {
        &self.successor
    }
    pub fn walks(&self) -> &[Vec<usize>] {
        &self.walks
    }
    pub fn dart_walks(&self) -> &[Option<usize>] {
        &self.dart_walks
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (v, s, n, w, d) = derive(self)?;
        if v != self.vertices
            || s != self.sectors
            || n != self.successor
            || w != self.walks
            || d != self.dart_walks
        {
            return Err(refused(
                "Natural support sectors, selected continuation or boundary walks changed",
            ));
        }
        Ok(())
    }
}
/// Qualify natural half-disc/quadrant sectors before reusing the selected-dart
/// continuation. Boundary walks remain distinct from connected physical faces.
pub fn own_periodic_support_cycles(
    owner: Arc<OwnedPeriodicSupportSelection>,
    rotation: Arc<OwnedPeriodicRotation>,
    max_refinements: usize,
    max_darts: usize,
) -> Result<OwnedPeriodicSupportCycles> {
    let mut o = OwnedPeriodicSupportCycles {
        owner,
        rotation,
        max_refinements,
        max_darts,
        vertices: Vec::new(),
        sectors: Vec::new(),
        successor: Vec::new(),
        walks: Vec::new(),
        dart_walks: Vec::new(),
    };
    let (v, s, n, w, d) = derive(&o)?;
    o.vertices = v;
    o.sectors = s;
    o.successor = n;
    o.walks = w;
    o.dart_walks = d;
    Ok(o)
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportLifts {
    owner: Arc<OwnedPeriodicSupportCycles>,
    max_refinements: usize,
    max_parts: usize,
    walks: Vec<OriginalPeriodicBoundaryWalk>,
    parts_count: usize,
}
fn derive_lifts(
    o: &OwnedPeriodicSupportLifts,
) -> Result<(Vec<OriginalPeriodicBoundaryWalk>, usize)> {
    let _replay = super::periodic_replay::scope();
    if o.max_refinements > 512 || !(1..=1000000).contains(&o.max_parts) {
        return Err(refused("Invalid complete support-boundary lift allowance"));
    }
    super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
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
impl OwnedPeriodicSupportLifts {
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportCycles> {
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
                "Support boundary original-clock lifts or source coverage changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_support_lifts(
    owner: Arc<OwnedPeriodicSupportCycles>,
    max_refinements: usize,
    max_parts: usize,
) -> Result<OwnedPeriodicSupportLifts> {
    let mut o = OwnedPeriodicSupportLifts {
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
    fn cycles(selection: Arc<OwnedPeriodicSupportSelection>) -> OwnedPeriodicSupportCycles {
        let charts = selection.rule().material().base().owner().owner().clone();
        let ports = Arc::new(own_periodic_ports(charts, 192).unwrap());
        let rotation =
            Arc::new(own_periodic_rotation(ports, ContactPortOrderLimits::default()).unwrap());
        own_periodic_support_cycles(selection, rotation, 192, 128).unwrap()
    }
    #[test]
    fn full_support_corner_sectors_close_one_outer_walk_and_empty_support_has_no_walks() {
        let o = cycles(super::super::periodic_support_material::tests::full_plane(
            true,
        ));
        assert_eq!(o.walks().len(), 1);
        assert_eq!(o.walks()[0].len(), 4);
        assert!(
            o.vertices()
                .iter()
                .all(|v| v.natural_bounds.iter().all(Option::is_some))
        );
        assert!(
            o.sectors()
                .iter()
                .all(|s| s.iter().filter(|s| s.operand_inside.is_none()).count() == 1)
        );
        let lifts = own_periodic_support_lifts(Arc::new(o.clone()), 192, 4).unwrap();
        assert_eq!(lifts.parts_count(), 4);
        assert!(lifts.walks()[0].is_closed_uv());
        let mut bad = o.clone();
        bad.vertices[0].natural_ports[0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.sectors[0]
            .iter_mut()
            .find(|s| s.operand_inside.is_none())
            .unwrap()
            .operand_inside = Some(vec![false]);
        assert!(bad.reverify_source().is_err());
        let empty = cycles(super::super::periodic_support_material::tests::full_plane(
            false,
        ));
        assert!(empty.walks().is_empty());
        assert_eq!(
            own_periodic_support_lifts(Arc::new(empty), 192, 1)
                .unwrap()
                .parts_count(),
            0
        );
    }
    #[test]
    fn authored_hole_and_natural_outer_boundary_keep_distinct_selected_walks() {
        let o = cycles(super::super::periodic_support_material::tests::with_hole());
        assert_eq!(o.walks().len(), 2);
        assert!(o.walks().iter().all(|w| w.len() == 4));
        assert_eq!(
            o.vertices()
                .iter()
                .filter(|v| v.natural_bounds.iter().any(Option::is_some))
                .count(),
            4
        );
        let lifts = own_periodic_support_lifts(Arc::new(o), 192, 8).unwrap();
        assert_eq!(lifts.parts_count(), 8);
        assert!(
            lifts
                .walks()
                .iter()
                .all(OriginalPeriodicBoundaryWalk::is_closed_uv)
        );
    }
    #[test]
    fn rational_tangent_hole_uses_actual_higher_germs_and_one_repeated_vertex_material_walk() {
        let o = cycles(super::super::periodic_support_material::tests::tangent_hole());
        assert!(!o.rotation().tangent_witnesses().is_empty());
        assert_eq!(o.owner().selected().len(), 9);
        assert_eq!(o.walks().len(), 1);
        assert_eq!(o.walks()[0].len(), 9);
        let contact = o
            .vertices()
            .iter()
            .position(|v| {
                v.natural_bounds == [None, Some(0)] && o.rotation().rotations()[v.vertex].len() == 4
            })
            .unwrap();
        assert_eq!(
            o.sectors()[contact]
                .iter()
                .filter(|s| s.operand_inside.is_none())
                .count(),
            1
        );
        assert_eq!(
            o.sectors()[contact].iter().filter(|s| s.material).count(),
            2
        );
        let walk = o.walks()[0]
            .iter()
            .map(|d| o.rotation().owner().ports()[*d].vertex)
            .collect::<Vec<_>>();
        assert_eq!(walk.iter().filter(|v| **v == contact).count(), 2);
        assert_eq!(
            own_periodic_support_lifts(Arc::new(o), 192, 9)
                .unwrap()
                .parts_count(),
            9
        );
    }
    #[test]
    fn actual_cylinder_rims_have_half_disc_sectors_and_opposite_noncontractible_lifts() {
        let rim = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
        let input = super::super::periodic_support_boundaries::tests::original(
            &surface,
            [true, false],
            &[],
            &[],
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
        let a = [["1".into(), "1".into()], ["1".into(), "2".into()]];
        let base = Arc::new(
            own_periodic_relative_winding(
                jumps.clone(),
                &[a.clone(), a.clone()],
                ParameterOrderLimits::default(),
                1000,
            )
            .unwrap(),
        );
        let rule = Arc::new(
            own_periodic_support_material_rule(
                support.clone(),
                base,
                &[OriginalPeriodicOperand {
                    wire_weights: vec![],
                    base_winding: "1".into(),
                    fill: OriginalPeriodicFillRule::NonZero,
                }],
                &[OriginalPeriodicBooleanOp::Operand(0)],
            )
            .unwrap(),
        );
        let mut sides = Vec::new();
        for edge in 0..jumps.owner().owner().edges().len() {
            let p = Arc::new(
                own_periodic_support_side_probes(
                    support.clone(),
                    jumps.clone(),
                    edge,
                    ["1".into(), "1024".into()],
                    ParameterOrderLimits::default(),
                )
                .unwrap(),
            );
            let samples = std::array::from_fn(|s| {
                if p.exterior()[s] {
                    None
                } else {
                    Some(Arc::new(
                        own_periodic_material_sample(
                            rule.material().clone(),
                            Arc::new(
                                own_periodic_relative_winding(
                                    jumps.clone(),
                                    &[a.clone(), p.queries()[s].clone()],
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
            sides.push(Arc::new(
                own_periodic_support_sides(p, rule.clone(), samples).unwrap(),
            ));
        }
        let o = cycles(Arc::new(
            own_periodic_support_selection(rule, &sides).unwrap(),
        ));
        assert_eq!(o.walks().len(), 2);
        assert!(
            o.vertices()
                .iter()
                .all(|v| v.natural_bounds[0].is_none() && v.natural_ports[1].len() == 2)
        );
        let lifts = own_periodic_support_lifts(Arc::new(o), 192, 2).unwrap();
        let mut homology = lifts
            .walks()
            .iter()
            .map(|w| w.homology.clone())
            .collect::<Vec<_>>();
        homology.sort();
        assert_eq!(
            homology,
            vec![
                ["-1".to_string(), "0".to_string()],
                ["1".to_string(), "0".to_string()]
            ]
        );
        assert!(lifts.walks().iter().all(|w| !w.is_closed_uv()));
    }
}
