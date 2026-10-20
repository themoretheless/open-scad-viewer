//! Continuous ORIGINAL-clock boundary walks in the universal UV cover.
use super::*;
use crate::profile_certificate::algebraic_intersections::{
    decode_polynomial, decode_ratio, polynomial,
};
use num_bigint::BigInt;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicBoundaryPart {
    pub dart: usize,
    pub edge: usize,
    pub piece: usize,
    pub source: usize,
    pub cell: usize,
    pub interval: usize,
    pub direction: i8,
    pub cuts: [OriginalPeriodicChartCutRef; 2],
    pub parameters: [OriginalSourceParameter; 2],
    /// Deck shift relative to the actual canonical piece; no f64 conversion.
    pub period_shift: [String; 2],
    /// Total exact offset of the unchanged ORIGINAL source pcurve.
    pub translation: [ExactRatio; 2],
    /// Each endpoint stays in its OWN original selected primitive root field.
    pub endpoints: [[[ExactPolynomial; 2]; 2]; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicBoundaryWalk {
    pub cycle: usize,
    pub start_vertex: usize,
    pub parts: Vec<OriginalPeriodicBoundaryPart>,
    pub homology: [String; 2],
    pub net_translation: [ExactRatio; 2],
}
impl OriginalPeriodicBoundaryWalk {
    /// Closed UV lift does not by itself establish simplicity or a material face.
    pub fn is_closed_uv(&self) -> bool {
        self.homology.iter().all(|v| v == "0")
    }
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicBoundaryLifts {
    owner: Arc<OwnedPeriodicCycles>,
    max_refinements: usize,
    max_parts: usize,
    walks: Vec<OriginalPeriodicBoundaryWalk>,
    parts_count: usize,
}
fn shift(v: &[String; 2]) -> Result<[BigInt; 2]> {
    Ok([
        v[0].parse()
            .map_err(|_| refused("Invalid original vertex deck shift"))?,
        v[1].parse()
            .map_err(|_| refused("Invalid original vertex deck shift"))?,
    ])
}
fn derive(o: &OwnedPeriodicBoundaryLifts) -> Result<(Vec<OriginalPeriodicBoundaryWalk>, usize)> {
    let ports = o.owner.owner().owner();
    let n = ports.ports().len();
    if o.max_refinements > 512 || o.max_parts == 0 || o.max_parts > 1000000 || n > o.max_parts {
        return Err(refused(
            "Complete original periodic boundary lift exceeds allowance",
        ));
    }
    o.owner.reverify_source()?;
    lift_walks(
        ports,
        o.owner.cycles(),
        o.max_refinements,
        o.max_parts,
        &vec![true; n],
    )
}
/// Internal shared ORIGINAL-clock lifting kernel. Callers independently replay
/// their complete typed cycle owner before supplying its exact admitted mask.
pub(in super::super) fn lift_walks(
    ports: &OwnedPeriodicPorts,
    cycles: &[Vec<usize>],
    max_refinements: usize,
    max_parts: usize,
    expected: &[bool],
) -> Result<(Vec<OriginalPeriodicBoundaryWalk>, usize)> {
    let charts = ports.owner();
    let graph = charts.owner();
    let cover = graph.owner().owner().pieces();
    let n = ports.ports().len();
    if expected.len() != n || n > 1000000 {
        return Err(refused("Original selected lift mask omits physical darts"));
    }
    let required = expected.iter().filter(|v| **v).count();
    if max_refinements > 512 || max_parts == 0 || max_parts > 1000000 || required > max_parts {
        return Err(refused(
            "Complete original periodic boundary lift exceeds allowance",
        ));
    }
    let domain = cover.chart();
    let periods = domain.map(|d| Q::from_float(d[1]).unwrap() - Q::from_float(d[0]).unwrap());
    let mut original = Vec::new();
    for c in cover.sources() {
        original.push(cells(c)?);
    }
    let mut seen = vec![false; n];
    let mut walks = Vec::new();
    let mut parts_count = 0;
    for (cycle, walk) in cycles.iter().enumerate() {
        let &first = walk
            .first()
            .ok_or_else(|| refused("Original periodic boundary walk is empty"))?;
        let first_port = ports
            .ports()
            .get(first)
            .ok_or_else(|| refused("Original periodic walk has unknown start dart"))?;
        let start_vertex = first_port.vertex;
        let mut vertex = start_vertex;
        let mut history = [BigInt::from(0), BigInt::from(0)];
        let mut parts = Vec::new();
        for &dart in walk {
            if dart >= n || seen[dart] || !expected[dart] {
                return Err(refused(
                    "Original periodic boundary lift duplicates or invents a dart",
                ));
            }
            seen[dart] = true;
            if parts_count >= max_parts {
                return Err(refused(
                    "Complete original boundary lift exhausted its part allowance",
                ));
            }
            parts_count += 1;
            let p = &ports.ports()[dart];
            let twin = p.edge * 2 + (1 - p.end);
            let q = ports
                .ports()
                .get(twin)
                .ok_or_else(|| refused("Original boundary lift lost opposite edge endpoint"))?;
            if p.vertex != vertex
                || p.edge != q.edge
                || p.piece != q.piece
                || p.source != q.source
                || p.cell != q.cell
                || p.interval != q.interval
                || p.cut == q.cut
                || p.direction != -q.direction
            {
                return Err(refused(
                    "Original boundary lift lost its directed native carrier",
                ));
            }
            let a = &charts.charts()[p.vertex].uses[p.chart_use];
            let b = &charts.charts()[q.vertex].uses[q.chart_use];
            let cuts = [
                OriginalPeriodicChartCutRef {
                    piece: p.piece,
                    cut: p.cut,
                },
                OriginalPeriodicChartCutRef {
                    piece: q.piece,
                    cut: q.cut,
                },
            ];
            if a.source != cuts[0] || b.source != cuts[1] {
                return Err(refused(
                    "Original boundary lift uses unrelated vertex transport",
                ));
            }
            let depart = shift(&a.period_shift)?;
            let arrive = shift(&b.period_shift)?;
            let deck = [&history[0] + &depart[0], &history[1] + &depart[1]];
            let piece = &cover.pieces()[p.piece];
            let offset = std::array::from_fn::<_, 2, _>(|axis| {
                Q::from_integer(deck[axis].clone()) * &periods[axis]
            });
            let translation = [
                decode_ratio(&piece.translation[0])? + &offset[0],
                decode_ratio(&piece.translation[1])? + &offset[1],
            ];
            let parameters = [a.parameter.clone(), b.parameter.clone()];
            let cell = &original[p.source][p.cell];
            let mut endpoints = Vec::new();
            for parameter in &parameters {
                let f = decode_polynomial(&parameter.primitive)?;
                let map = decode_polynomial(&parameter.parameter)?;
                let root = [
                    decode_ratio(&parameter.interval[0])?,
                    decode_ratio(&parameter.interval[1])?,
                ];
                let point = arithmetic::shore_events::translated_point(
                    &cell.net,
                    &cell.domain,
                    &map,
                    &f,
                    &root,
                    &translation,
                    max_refinements,
                )?;
                endpoints.push(point.map(|c| c.map(|p| polynomial(&p))));
            }
            parts.push(OriginalPeriodicBoundaryPart {
                dart,
                edge: p.edge,
                piece: p.piece,
                source: p.source,
                cell: p.cell,
                interval: p.interval,
                direction: p.direction,
                cuts,
                parameters,
                period_shift: deck.map(|q| q.to_string()),
                translation: translation.map(|q| ratio(&q)),
                endpoints: endpoints
                    .try_into()
                    .map_err(|_| refused("Incomplete lifted original endpoints"))?,
            });
            history = [
                &history[0] + &depart[0] - &arrive[0],
                &history[1] + &depart[1] - &arrive[1],
            ];
            vertex = q.vertex;
        }
        if vertex != start_vertex {
            return Err(refused(
                "Original boundary lift does not return to its actual quotient vertex",
            ));
        }
        let wrapped = cover.wrapped();
        for axis in 0..2 {
            if !wrapped[axis] && history[axis] != BigInt::from(0) {
                return Err(refused("Boundary homology changes an unwrapped coordinate"));
            }
        }
        let net_translation = std::array::from_fn::<_, 2, _>(|axis| {
            ratio(&(Q::from_integer(history[axis].clone()) * &periods[axis]))
        });
        walks.push(OriginalPeriodicBoundaryWalk {
            cycle,
            start_vertex,
            parts,
            homology: history.map(|q| q.to_string()),
            net_translation,
        });
    }
    if parts_count != expected.iter().filter(|v| **v).count() || seen != expected {
        return Err(refused(
            "Original periodic boundary lift omits directed source darts",
        ));
    }
    Ok((walks, parts_count))
}
impl OwnedPeriodicBoundaryLifts {
    pub fn owner(&self) -> &Arc<OwnedPeriodicCycles> {
        &self.owner
    }
    pub fn walks(&self) -> &[OriginalPeriodicBoundaryWalk] {
        &self.walks
    }
    pub fn parts_count(&self) -> usize {
        self.parts_count
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (w, n) = derive(self)?;
        if w != self.walks || n != self.parts_count {
            return Err(refused(
                "Original lifted boundary, exact homology or source ownership changed",
            ));
        }
        Ok(())
    }
}
/// Continuous original-clock UV lifts with complete directed edge ownership.
/// A nonzero deck displacement remains explicit, never forced into a closed
/// planar loop. Actual surface tracing stays bound to the original piece.
pub fn own_periodic_boundary_lifts(
    owner: Arc<OwnedPeriodicCycles>,
    max_refinements: usize,
    max_parts: usize,
) -> Result<OwnedPeriodicBoundaryLifts> {
    let mut o = OwnedPeriodicBoundaryLifts {
        owner,
        max_refinements,
        max_parts,
        walks: Vec::new(),
        parts_count: 0,
    };
    let (walks, parts_count) = derive(&o)?;
    o.walks = walks;
    o.parts_count = parts_count;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    use periodic_rotation::tests::ports;
    fn own(curves: &[Curve]) -> OwnedPeriodicBoundaryLifts {
        let rotation = Arc::new(
            own_periodic_rotation(ports(curves), ContactPortOrderLimits::default()).unwrap(),
        );
        let cycles = Arc::new(own_periodic_cycles(rotation, 100000).unwrap());
        own_periodic_boundary_lifts(cycles, 192, 100000).unwrap()
    }
    fn point(part: &OriginalPeriodicBoundaryPart, end: usize) -> [Q; 2] {
        let parameter = &part.parameters[end];
        assert_eq!(parameter.interval[0], parameter.interval[1]);
        let root = decode_ratio(&parameter.interval[0]).unwrap();
        let eval = |p: &ExactPolynomial| {
            decode_polynomial(p)
                .unwrap()
                .iter()
                .rev()
                .fold(Q::zero(), |acc, c| acc * &root + c)
        };
        std::array::from_fn(|axis| {
            eval(&part.endpoints[end][axis][0]) / eval(&part.endpoints[end][axis][1])
        })
    }
    #[test]
    fn noncontractible_loop_keeps_two_opposite_lifts_and_original_nonunit_clock() {
        let a = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 6., 6., 6.],
            control_points: vec![vec![0., 0.5], vec![0., 0.5], vec![4., 0.5]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let o = own(&[a]);
        o.reverify_source().unwrap();
        assert_eq!(o.walks().len(), 2);
        assert_eq!(o.parts_count(), 2);
        assert_eq!(o.walks()[0].homology, ["1", "0"]);
        assert_eq!(o.walks()[1].homology, ["-1", "0"]);
        assert!(o.walks().iter().all(|w| !w.is_closed_uv()));
        assert_eq!(
            o.walks()[0].net_translation[0],
            ["4".to_string(), "1".to_string()]
        );
        assert_eq!(
            o.walks()[1].net_translation[0],
            ["-4".to_string(), "1".to_string()]
        );
        assert_eq!(point(&o.walks()[0].parts[0], 0)[0], Q::zero());
        assert_eq!(
            point(&o.walks()[0].parts[0], 1)[0],
            Q::from_integer(4.into())
        );
        assert_eq!(point(&o.walks()[1].parts[0], 0)[0], Q::zero());
        assert_eq!(
            point(&o.walks()[1].parts[0], 1)[0],
            Q::from_integer((-4).into())
        );
        assert_eq!(
            o.walks()[0].parts[0].parameters[0].parameter,
            vec![["2".to_string(), "1".to_string()]]
        );
        assert_eq!(
            o.walks()[0].parts[0].parameters[1].parameter,
            vec![["6".to_string(), "1".to_string()]]
        );
        assert!(own_periodic_boundary_lifts(o.owner().clone(), 192, 1).is_err());
        let mut bad = o.clone();
        bad.walks[0].homology[0] = "0".to_string();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.walks[0].parts[0].translation[0] = ["999".to_string(), "1".to_string()];
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.walks[0].parts[0].endpoints[1][0][0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.parts_count += 1;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn geometric_boundary_walk_does_not_double_count_coincident_source_traversals() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![8., 0.5]]).unwrap();
        let o = own(&[a]);
        o.reverify_source().unwrap();
        assert_eq!(
            o.owner().owner().owner().owner().owner().edges()[0]
                .carriers
                .len(),
            2
        );
        assert_eq!(o.parts_count(), 2);
        assert_eq!(o.walks()[0].homology, ["1", "0"]);
        assert_eq!(o.walks()[1].homology, ["-1", "0"]);
    }
    #[test]
    fn tangent_seam_walks_get_continuous_closed_lifts_without_fitted_controls() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
            control_points: vec![
                vec![0., 0.5],
                vec![0., 0.75],
                vec![1., 0.75],
                vec![0.5, 0.625],
                vec![0., 0.5],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        let mut b = a.clone();
        for p in &mut b.control_points {
            p[0] = 4. - p[0];
        }
        let o = own(&[a, b]);
        o.reverify_source().unwrap();
        assert_eq!(o.walks().len(), 3);
        assert_eq!(o.parts_count(), 8);
        assert!(
            o.walks()
                .iter()
                .all(OriginalPeriodicBoundaryWalk::is_closed_uv)
        );
        for w in o.walks() {
            for i in 0..w.parts.len() {
                assert_eq!(
                    point(&w.parts[i], 1),
                    point(&w.parts[(i + 1) % w.parts.len()], 0)
                );
                if w.parts[i].source == 1 {
                    assert_eq!(
                        w.parts[i].translation[0],
                        ["-4".to_string(), "1".to_string()]
                    );
                }
            }
        }
    }
}
