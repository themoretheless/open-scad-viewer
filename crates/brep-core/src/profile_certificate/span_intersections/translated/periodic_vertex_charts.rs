//! Exact local chart transport at every actual-support quotient vertex.
use super::*;
use crate::profile_certificate::algebraic_intersections::{
    decode_polynomial, decode_ratio, polynomial,
};
use num_bigint::BigInt;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicVertexChartUse {
    pub source: OriginalPeriodicChartCutRef,
    pub period_shift: [String; 2],
    pub translation: [ExactRatio; 2],
    /// Parent source use, graph relation index, and reverse traversal flag.
    pub parent: Option<(OriginalPeriodicChartCutRef, usize, bool)>,
    pub parameter: OriginalSourceParameter,
    /// Common-chart point in THIS original use's selected root field.
    pub position: [[ExactPolynomial; 2]; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicVertexChart {
    pub vertex: usize,
    pub canonical: OriginalPeriodicChartCutRef,
    pub uses: Vec<OriginalPeriodicVertexChartUse>,
    pub relations: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicVertexCharts {
    owner: Arc<OwnedPeriodicQuotientGraph>,
    max_refinements: usize,
    max_steps: usize,
    charts: Vec<OriginalPeriodicVertexChart>,
    steps: usize,
}
fn relation_delta(
    o: &OwnedPeriodicVertexCharts,
    r: &OriginalPeriodicChartRelation,
) -> Result<[BigInt; 2]> {
    let aligned = o.owner.owner();
    let arrangement = aligned.owner();
    let (pair, reverse) = match r.origin {
        OriginalPeriodicChartRelationOrigin::Event { pair, .. }
        | OriginalPeriodicChartRelationOrigin::Common { pair, .. } => (pair, false),
        OriginalPeriodicChartRelationOrigin::Alias { alias, .. } => {
            let a = aligned
                .aliases()
                .get(alias)
                .ok_or_else(|| refused("Missing exact interval alias for vertex transport"))?;
            (a.pair, a.side == 1)
        }
    };
    let delta = arrangement
        .pairs()
        .get(pair)
        .ok_or_else(|| refused("Missing original contact pair for vertex transport"))?
        .owner()
        .second_translation_delta();
    let chart = arrangement.pieces().chart();
    let wrapped = arrangement.pieces().wrapped();
    let mut out = [BigInt::from(0), BigInt::from(0)];
    for axis in 0..2 {
        let mut d = decode_ratio(&delta[axis])?;
        if reverse {
            d = -d;
        }
        if !wrapped[axis] {
            if !d.is_zero() {
                return Err(refused("Local vertex transport shifts an unwrapped axis"));
            }
            continue;
        }
        let period =
            Q::from_float(chart[axis][1]).unwrap() - Q::from_float(chart[axis][0]).unwrap();
        if period <= Q::zero() {
            return Err(refused("Invalid actual chart period for vertex transport"));
        }
        let q = d / period;
        if !q.is_integer() {
            return Err(refused(
                "Vertex contact transport is not an exact whole period",
            ));
        }
        out[axis] = q.to_integer();
    }
    Ok(out)
}
type ArcStep = (OriginalPeriodicChartCutRef, [BigInt; 2], usize, bool);
type Assigned = (
    [BigInt; 2],
    Option<(OriginalPeriodicChartCutRef, usize, bool)>,
);
fn derive(o: &OwnedPeriodicVertexCharts) -> Result<(Vec<OriginalPeriodicVertexChart>, usize)> {
    if o.max_refinements > 512 || o.max_steps == 0 || o.max_steps > 1000000 {
        return Err(refused("Invalid complete local vertex chart allowance"));
    }
    o.owner.reverify_source()?;
    let mut vertex_of = BTreeMap::new();
    for (vertex, v) in o.owner.vertices().iter().enumerate() {
        for &r in &v.uses {
            if vertex_of.insert(r, vertex).is_some() {
                return Err(refused(
                    "Original source cut belongs to multiple physical vertices",
                ));
            }
        }
    }
    let mut adjacency: BTreeMap<OriginalPeriodicChartCutRef, Vec<ArcStep>> = BTreeMap::new();
    let mut relations = vec![Vec::new(); o.owner.vertices().len()];
    for (index, r) in o.owner.relations().iter().enumerate() {
        let a = *vertex_of
            .get(&r.cuts[0])
            .ok_or_else(|| refused("Missing original vertex transport source"))?;
        let b = *vertex_of
            .get(&r.cuts[1])
            .ok_or_else(|| refused("Missing original vertex transport counterpart"))?;
        if a != b {
            return Err(refused(
                "Contact relation crosses different physical vertex classes",
            ));
        }
        let d = relation_delta(o, r)?;
        adjacency
            .entry(r.cuts[0])
            .or_default()
            .push((r.cuts[1], d.clone(), index, false));
        adjacency
            .entry(r.cuts[1])
            .or_default()
            .push((r.cuts[0], d.map(|q| -q), index, true));
        relations[a].push(index);
    }
    let arrangement = o.owner.owner().owner();
    let cover = arrangement.pieces();
    let chart = cover.chart();
    let periods = chart.map(|v| Q::from_float(v[1]).unwrap() - Q::from_float(v[0]).unwrap());
    let mut charts = Vec::new();
    let mut steps = 0;
    for (vertex, v) in o.owner.vertices().iter().enumerate() {
        let canonical = *v
            .uses
            .first()
            .ok_or_else(|| refused("Physical vertex has no actual original cut use"))?;
        let mut assigned: BTreeMap<OriginalPeriodicChartCutRef, Assigned> = BTreeMap::new();
        assigned.insert(canonical, ([BigInt::from(0), BigInt::from(0)], None));
        let mut stack = vec![canonical];
        while let Some(source) = stack.pop() {
            let current = assigned[&source].0.clone();
            for (target, delta, index, reverse) in adjacency.get(&source).into_iter().flatten() {
                if steps >= o.max_steps {
                    return Err(refused(
                        "Complete vertex chart transport exhausted its relation allowance",
                    ));
                }
                steps += 1;
                let shift = [&current[0] + &delta[0], &current[1] + &delta[1]];
                if let Some(existing) = assigned.get(target) {
                    if existing.0 != shift {
                        return Err(refused(
                            "Physical vertex transport cycle has a nonzero period defect",
                        ));
                    }
                } else {
                    assigned.insert(*target, (shift, Some((source, *index, *reverse))));
                    stack.push(*target);
                }
            }
        }
        if assigned.len() != v.uses.len() {
            return Err(refused(
                "Physical vertex has unconnected/unproved chart uses",
            ));
        }
        let mut uses = Vec::new();
        for source in &v.uses {
            let (shift, parent) = assigned[source].clone();
            let translation = std::array::from_fn::<_, 2, _>(|axis| {
                Q::from_integer(shift[axis].clone()) * &periods[axis]
            });
            let piece = &cover.pieces()[source.piece];
            let parameter = o.owner.owner().cuts()[source.piece][source.cut]
                .value
                .clone();
            let original = cells(&cover.sources()[piece.source])?;
            let cell = &original[piece.cell];
            let f = decode_polynomial(&parameter.primitive)?;
            let map = decode_polynomial(&parameter.parameter)?;
            let root = [
                decode_ratio(&parameter.interval[0])?,
                decode_ratio(&parameter.interval[1])?,
            ];
            let offset = [
                decode_ratio(&piece.translation[0])? + &translation[0],
                decode_ratio(&piece.translation[1])? + &translation[1],
            ];
            let point = arithmetic::shore_events::translated_point(
                &cell.net,
                &cell.domain,
                &map,
                &f,
                &root,
                &offset,
                o.max_refinements,
            )?;
            uses.push(OriginalPeriodicVertexChartUse {
                source: *source,
                period_shift: shift.map(|q| q.to_string()),
                translation: translation.map(|q| ratio(&q)),
                parent,
                parameter,
                position: point.map(|c| c.map(|p| polynomial(&p))),
            });
        }
        charts.push(OriginalPeriodicVertexChart {
            vertex,
            canonical,
            uses,
            relations: relations[vertex].clone(),
        });
    }
    Ok((charts, steps))
}
impl OwnedPeriodicVertexCharts {
    pub fn owner(&self) -> &Arc<OwnedPeriodicQuotientGraph> {
        &self.owner
    }
    pub fn charts(&self) -> &[OriginalPeriodicVertexChart] {
        &self.charts
    }
    pub fn steps(&self) -> usize {
        self.steps
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (c, w) = derive(self)?;
        if c != self.charts || w != self.steps {
            return Err(refused(
                "Exact local vertex chart transport, source point or work changed",
            ));
        }
        Ok(())
    }
}
/// A common intrinsic chart per vertex, derived from ALL proved contact/alias
/// translations. Every period cycle must close exactly. Independent original
/// root fields remain independent; no floating-point center is synthesized.
pub fn own_periodic_vertex_charts(
    owner: Arc<OwnedPeriodicQuotientGraph>,
    max_refinements: usize,
    max_steps: usize,
) -> Result<OwnedPeriodicVertexCharts> {
    let mut o = OwnedPeriodicVertexCharts {
        owner,
        max_refinements,
        max_steps,
        charts: Vec::new(),
        steps: 0,
    };
    let (charts, steps) = derive(&o)?;
    o.charts = charts;
    o.steps = steps;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    use periodic_graph::quotient_tests::{aligned, sources};
    fn graph(end: f64) -> Arc<OwnedPeriodicQuotientGraph> {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![end, 0.5]]).unwrap();
        let (base, seams) = sources(&[a]);
        Arc::new(
            own_periodic_quotient_graph(aligned(base, seams), ParameterOrderLimits::default())
                .unwrap(),
        )
    }
    #[test]
    fn loop_vertex_uses_one_exact_chart_and_retains_both_original_clocks() {
        let graph = graph(4.);
        assert!(own_periodic_vertex_charts(graph.clone(), 192, 1).is_err());
        let o = own_periodic_vertex_charts(graph, 192, 4096).unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.charts().len(), 1);
        let c = &o.charts()[0];
        assert_eq!(c.uses.len(), 2);
        assert_eq!(c.uses[0].period_shift, ["0", "0"]);
        assert_eq!(c.uses[1].period_shift, ["-1", "0"]);
        assert_eq!(
            c.uses[1].translation[0],
            ["-4".to_string(), "1".to_string()]
        );
        assert!(c.uses.iter().all(|u| u.position[0][0].is_empty()));
        assert_eq!(o.steps(), 2 * o.owner().relations().len());
        assert!(c.uses[0].parent.is_none());
        assert!(c.uses[1].parent.is_some());
        let mut bad = o.clone();
        bad.charts[0].uses[1].translation[0] = ["0".to_string(), "1".to_string()];
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.charts[0].uses[1].parent = None;
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.charts[0].uses[1].position[1][0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.steps += 1;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn double_traversal_keeps_same_natural_parameter_in_distinct_piece_charts() {
        let o = own_periodic_vertex_charts(graph(8.), 192, 10000).unwrap();
        o.reverify_source().unwrap();
        let c = &o.charts()[0];
        assert_eq!(o.owner().vertices().len(), 1);
        assert_eq!(o.owner().edges().len(), 1);
        assert_eq!(o.owner().edges()[0].carriers.len(), 2);
        assert_eq!(c.uses.len(), 4);
        let end = c
            .uses
            .iter()
            .find(|u| u.source.piece == 0 && u.source.cut == 1)
            .unwrap();
        let start = c
            .uses
            .iter()
            .find(|u| u.source.piece == 1 && u.source.cut == 0)
            .unwrap();
        assert_eq!(end.period_shift, ["-1", "0"]);
        assert_eq!(start.period_shift, ["0", "0"]);
        assert!(c.uses.iter().all(|u| u.position[0][0].is_empty()));
        assert!(o.owner().relations().iter().any(|r|matches!(r.origin,OriginalPeriodicChartRelationOrigin::Common{pair,..} if o.owner().owner().owner().pairs()[pair].owner().second_translation_delta()[0]!=["0".to_string(),"1".to_string()])));
    }
    #[test]
    fn nonrepresentable_actual_knot_period_never_becomes_a_float_translation() {
        use num_traits::ToPrimitive;
        let mut rim = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        rim.knots = vec![0.1, 0.1, 0.9, 1.7, 2.5, 3.3, 3.3];
        let surface = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
        let a = Curve::from_polyline(vec![vec![0.1, 0.5], vec![3.3, 0.5]]).unwrap();
        let cover = Arc::new(
            own_original_periodic_pieces(
                &[a],
                [[0.1, 3.3], [0., 1.]],
                [true, false],
                1e-12,
                ParameterOrderLimits::default(),
                32,
            )
            .unwrap(),
        );
        let support = Arc::new(
            bind_original_periodic_pieces_to_surface(
                cover.clone(),
                &surface,
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        let wires = Arc::new(
            own_original_periodic_wires(
                support,
                &[vec![OriginalPeriodicSourceUse {
                    source: 0,
                    reversed: false,
                }]],
            )
            .unwrap(),
        );
        let embedded = Arc::new(
            own_original_periodic_embedding(
                wires,
                4096,
                OriginalInjectiveAtlasLimits::default(),
                OriginalAtlasPairLimits::default(),
            )
            .unwrap(),
        );
        let base = Arc::new(
            own_periodic_arrangement_cuts(cover, 1e-12, ParameterOrderLimits::default(), 1)
                .unwrap(),
        );
        let seams = Arc::new(
            own_periodic_seam_contacts(embedded, 1e-12, ParameterOrderLimits::default(), 2)
                .unwrap(),
        );
        let g = Arc::new(
            own_periodic_quotient_graph(aligned(base, seams), ParameterOrderLimits::default())
                .unwrap(),
        );
        let o = own_periodic_vertex_charts(g, 192, 4096).unwrap();
        o.reverify_source().unwrap();
        let shift = decode_ratio(&o.charts()[0].uses[1].translation[0]).unwrap();
        let expected = Q::from_float(0.1).unwrap() - Q::from_float(3.3).unwrap();
        assert_eq!(shift, expected);
        assert_ne!(Q::from_float(shift.to_f64().unwrap()).unwrap(), shift);
        for u in &o.charts()[0].uses {
            let n = decode_polynomial(&u.position[0][0]).unwrap();
            let d = decode_polynomial(&u.position[0][1]).unwrap();
            assert_eq!(n.len(), 1);
            assert_eq!(d.len(), 1);
            assert_eq!(&n[0] / &d[0], Q::from_float(0.1).unwrap());
        }
    }
}
