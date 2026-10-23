//! Fill classification of admitted represented graph walks; no ideal-offset certificate.
use crate::{Result, check, chord_arrangement::Arrangement, chord_faces, chord_winding::{self, FillRule}, chord_witness, curve_offset::Segment};
#[derive(Debug)]
pub struct ClassifiedWalk {
    pub vertices: Vec<usize>,
    pub witness: [f64; 2],
    pub winding: i32,
    pub filled: bool,
}
/// Edges must retain the connected source traversal order. Both orientations are
/// classified; filled walks are not yet a merged region boundary.
pub fn classify(graph: &Arrangement, rule: FillRule, max_witness_checks: usize) -> Result<Vec<ClassifiedWalk>> {
    check((1..=1000000).contains(&max_witness_checks) && graph.vertices.len()<=65536
        && graph.edges.len()<=131072 && graph.vertices.iter().all(|v|v.point.iter().all(|x|x.is_finite() && x.abs()<=1e9)),
        "Use bounded finite graph coordinates and a witness budget.")?;
    let source_edges=graph.source_edges.as_deref().unwrap_or(&graph.edges);
    check(!source_edges.is_empty() && source_edges.len() <= 65536 && source_edges.iter().all(|e|e.vertices.iter().all(|v|*v<graph.vertices.len())), "Use a bounded represented source traversal.")?;
    // Partition ordered source traversals at exact closure; disconnected loops
    // keep their orientation and share the reconstructed graph coordinates.
    let mut sources = Vec::new();
    let mut source = Vec::new();
    let mut first = source_edges[0].vertices[0];
    let mut previous = first;
    for e in source_edges {
        if source.is_empty() { first = e.vertices[0]; previous = first; }
        check(e.vertices[0] == previous, "Source loop is disconnected before closure.")?;
        let i = source.len();
        source.push(Segment { points: e.vertices.map(|v| graph.vertices[v].point), domain: [i as f64,(i+1) as f64], error_upper_mm: 0. });
        previous = e.vertices[1];
        if previous == first {
            check(sources.len() < 128, "Use at most 128 source loops.")?;
            sources.push(std::mem::take(&mut source));
        }
    }
    check(source.is_empty(), "Source loop is not closed.")?;
    // Closed source traversals imply zero net incidence at every vertex.
    // Cancel coincident opposite passes before graph admission and point winding.
    let mut net=std::collections::BTreeMap::<[usize;2],i32>::new();
    for e in source_edges {
        let [a,b]=e.vertices;check(a!=b,"Source has a collapsed represented edge.")?;
        let (key,sign)=if a<b {([a,b],1)} else {([b,a],-1)};
        *net.entry(key).or_default()+=sign;
    }
    let geometry:std::collections::BTreeSet<_>=graph.edges.iter().map(|e| {let [a,b]=e.vertices;if a<b {[a,b]} else {[b,a]}}).collect();
    check(geometry.len()==graph.edges.len() && geometry.len()==net.len() && net.keys().all(|k|geometry.contains(k)),
        "Graph geometry does not match represented source occurrences.")?;
    let active=Arrangement {vertices:graph.vertices.clone(),edges:graph.edges.iter().filter(|e| {
        let [a,b]=e.vertices;net[&if a<b {[a,b]} else {[b,a]}]!=0
    }).cloned().collect(),source_edges:None};
    if active.edges.is_empty() { return Ok(Vec::new()); }
    let mut circulation=Vec::new();
    for ([a,b],multiplicity) in net {
        let pair=if multiplicity>0 {[a,b]} else {[b,a]};
        for _ in 0..multiplicity.unsigned_abs() {
            let i=circulation.len();
            circulation.push(Segment {points:pair.map(|v|graph.vertices[v].point),domain:[i as f64,(i+1) as f64],error_upper_mm:0.});
        }
    }
    let walks=chord_faces::walk(&active)?;

    walks.counterclockwise.into_iter().chain(walks.clockwise).map(|vertices| {
        let witness = chord_witness::left(&active, &vertices, max_witness_checks)?;
        let winding = chord_winding::at_segments(&circulation, witness)?;
        Ok(ClassifiedWalk { vertices, witness, winding, filled: chord_winding::filled(winding, rule) })
    }).collect()
}
/// Return oriented boundary cycles with filled material on their left.
/// Internal edges are crossed through the adjoining filled walk, not retained.
pub fn boundaries(graph: &Arrangement, rule: FillRule, max_checks: usize) -> Result<Vec<Vec<usize>>> {
    use std::collections::{BTreeMap, BTreeSet};
    let walks = classify(graph, rule, max_checks)?;
    let mut next = BTreeMap::new();
    let mut fill = BTreeMap::new();
    for walk in walks {
        let n = walk.vertices.len();
        for i in 0..n {
            let edge = [walk.vertices[i], walk.vertices[(i+1)%n]];
            let successor = [edge[1],walk.vertices[(i+2)%n]];
            check(next.insert(edge, successor).is_none(), "Repeated directed graph edge.")?;
            fill.insert(edge, walk.filled);
        }
    }
    let mut remaining = BTreeSet::new();
    for (&edge, &filled) in &fill {
        let reverse = [edge[1],edge[0]];
        check(fill.contains_key(&reverse), "Missing opposite graph walk.")?;
        let opposite = fill[&reverse];
        if filled && !opposite { remaining.insert(edge); }
    }
    let mut cycles = Vec::new();
    while let Some(&start) = remaining.first() {
        let mut edge = start;
        let mut cycle = Vec::new();
        loop {
            check(remaining.remove(&edge), "Region boundary revisited before closure.")?;
            cycle.push(edge[0]);
            let mut successor = next[&edge];
            let mut steps = 0;
            while fill[&[successor[1],successor[0]]] {
                check(fill[&successor] && steps < next.len(), "Cannot resolve merged region boundary.")?;
                successor = next[&[successor[1],successor[0]]];
                steps += 1;
            }
            check(fill[&successor], "Region boundary has an empty left side.")?;
            edge = successor;
            if edge == start { break; }
        }
        check(cycle.len() >= 3, "Region boundary collapsed.")?;
        cycles.push(cycle);
    }
    Ok(cycles)
}
/// Materialize each selected boundary as connected degree-one NURBS chunks.
/// Loop ownership is retained even when the 256-control-point limit splits it.
pub fn boundary_curves(graph: &Arrangement, rule: FillRule, max_checks: usize) -> Result<Vec<Vec<crate::curve::Curve>>> {
    boundaries(graph,rule,max_checks)?.into_iter().map(|cycle| {
        let mut points: Vec<_> = cycle.iter().map(|v|graph.vertices[*v].point.to_vec()).collect();
        points.push(points[0].clone());
        let mut chunks=Vec::new();
        let mut start=0;
        while start+1 < points.len() {
            let end=(start+256).min(points.len());
            let controls=points[start..end].to_vec();
            let n=controls.len();
            let mut knots=vec![0.];
            knots.extend((0..n).map(|i|i as f64));knots.push((n-1) as f64);
            let curve=crate::curve::Curve { degree:1,knots,control_points:controls,weights:vec![1.;n],periodic:false };
            curve.validate()?;chunks.push(curve);start=end-1;
        }
        Ok(chunks)
    }).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(points: &[[f64;2]]) -> Arrangement {
        let source: Vec<_> = points.windows(2).enumerate().map(|(i,p)| Segment { points:[p[0],p[1]], domain:[i as f64,(i+1) as f64], error_upper_mm:0. }).collect();
        crate::chord_arrangement::split(&source,true,1e-6,1000000).unwrap()
    }
    fn combine(mut a: Arrangement, b: Arrangement) -> Arrangement {
        let offset=a.vertices.len();a.vertices.extend(b.vertices);
        let mut source=a.source_edges.take().unwrap_or_else(||a.edges.clone());
        source.extend(b.source_edges.unwrap_or_else(||b.edges.clone()).into_iter().map(|mut e| { e.vertices=e.vertices.map(|v|v+offset);e }));
        a.source_edges=Some(source);
        a.edges.extend(b.edges.into_iter().map(|mut e| { e.vertices=e.vertices.map(|v|v+offset);e }));a
    }
    #[test]
    fn retraced_spikes_and_closed_bridges_do_not_change_filled_regions() {
        let spike=fixture(&[[0.,0.],[4.,0.],[4.,4.],[0.,4.],[0.,0.],[-3.,0.],[0.,0.]]);
        for rule in [FillRule::NonZero,FillRule::EvenOdd] {
            let loops=boundaries(&spike,rule,10000).unwrap();assert_eq!(loops.len(),1);assert_eq!(loops[0].len(),4);
        }
        let empty=fixture(&[[0.,0.],[3.,3.],[0.,0.]]);
        assert!(boundaries(&empty,FillRule::NonZero,10000).unwrap().is_empty());
        let bridge=fixture(&[[0.,0.],[2.,0.],[2.,2.],[0.,2.],[0.,0.],[-4.,0.],[-6.,0.],[-6.,-2.],[-4.,-2.],[-4.,0.],[0.,0.]]);
        assert_eq!(boundaries(&bridge,FillRule::NonZero,10000).unwrap().len(),2);
    }
    #[test]
    fn coincident_traversals_preserve_winding_and_opposite_directions_cancel() {
        let square=vec![[0.,0.],[4.,0.],[4.,4.],[0.,4.],[0.,0.]];
        for reversed in [false,true] {
            let mut points=square.clone();let second=if reversed {square.iter().copied().rev().collect::<Vec<_>>()} else {square.clone()};
            points.extend(second.into_iter().skip(1));
            let graph=fixture(&points);
            assert_eq!(graph.edges.len(),4);assert_eq!(graph.source_edges.as_ref().unwrap().len(),8);
            assert_eq!(boundaries(&graph,FillRule::NonZero,10000).unwrap().len(),usize::from(!reversed));
            assert!(boundaries(&graph,FillRule::EvenOdd,10000).unwrap().is_empty());
        }
    }
    #[test]
    fn partial_shared_boundary_is_removed_between_filled_regions() {
        let graph=fixture(&[[0.,0.],[4.,0.],[4.,4.],[0.,4.],[0.,0.],[0.,-2.],[2.,-2.],[2.,0.],[0.,0.]]);
        assert_eq!(graph.edges.len(),8);assert_eq!(graph.source_edges.as_ref().unwrap().len(),9);
        for rule in [FillRule::NonZero,FillRule::EvenOdd] {
            let loops=boundaries(&graph,rule,10000).unwrap();assert_eq!(loops.len(),1);
            let c=&loops[0];let area=(0..c.len()).map(|i| {let a=graph.vertices[c[i]].point;let b=graph.vertices[c[(i+1)%c.len()]].point;a[0]*b[1]-a[1]*b[0]}).sum::<f64>()*0.5;
            assert_eq!(area,20.);
        }
    }
    #[test]
    fn long_boundary_chunks_share_endpoints_and_close() {
        let mut points:Vec<_>=(0..300).map(|i| {let t=i as f64*std::f64::consts::TAU/300.;[t.cos()*10.,t.sin()*10.]}).collect();
        points.push(points[0]);
        let graph=fixture(&points);
        let loops=boundary_curves(&graph,FillRule::NonZero,1000000).unwrap();
        assert_eq!(loops.len(),1);assert_eq!(loops[0].len(),2);
        assert_eq!(loops[0][0].control_points.len(),256);
        assert_eq!(loops[0][0].control_points.last(),loops[0][1].control_points.first());
        assert_eq!(loops[0][0].control_points.first(),loops[0][1].control_points.last());
    }
    #[cfg(feature="transport")]
    #[test]
    fn trimmed_offset_transport_preserves_loops_and_scope() {
        let curve=crate::curve::Curve {degree:1,control_points:vec![vec![0.,0.],vec![4.,0.],vec![4.,4.],vec![0.,4.],vec![0.,0.]],knots:vec![0.,0.,1.,2.,3.,4.,4.],weights:vec![1.;5],periodic:false};
        let result=crate::transport::dispatch(value_codec::json!({"op":"curve_offset_trimmed_bevel","curve":value_codec::to_value(curve).unwrap(),"distance":-0.5,"toleranceMm":1e-4,"maxCells":1024,"intersectionToleranceMm":1e-6,"fillRule":"nonzero"})).unwrap();
        assert_eq!(result["loops"].as_array().unwrap().len(),1);
        assert_eq!(result["report"]["regionTrimmed"],true);
        assert_eq!(result["report"]["originalOffsetTopologyCertified"],false);
    }
    #[cfg(feature="transport")]
    #[test]
    fn crossed_source_offset_produces_multiple_closed_regions() {
        let curve=crate::curve::Curve {degree:1,control_points:vec![vec![0.,0.],vec![4.,4.],vec![0.,4.],vec![4.,0.],vec![0.,0.]],knots:vec![0.,0.,1.,2.,3.,4.,4.],weights:vec![1.;5],periodic:false};
        let result=crate::transport::dispatch(value_codec::json!({"op":"curve_offset_trimmed_bevel","curve":value_codec::to_value(curve).unwrap(),"distance":0.1,"toleranceMm":1e-4,"maxCells":1024,"intersectionToleranceMm":1e-6,"fillRule":"nonzero"})).unwrap();
        let loops=result["loops"].as_array().unwrap();assert!(loops.len()>=2);
        for wire in loops {let chunks=wire.as_array().unwrap();assert_eq!(chunks[0]["controlPoints"][0],chunks.last().unwrap()["controlPoints"].as_array().unwrap().last().unwrap().clone());}
    }
    #[test]
    fn selected_boundary_curves_retain_exact_closure() {
        let graph=fixture(&[[0.,6.],[4.,-6.],[-6.,2.],[6.,2.],[-4.,-6.],[0.,6.]]);
        for rule in [FillRule::NonZero,FillRule::EvenOdd] {
            let curves=boundary_curves(&graph,rule,100000).unwrap();
            for chunks in curves {
                assert_eq!(chunks[0].control_points[0],*chunks.last().unwrap().control_points.last().unwrap());
                for c in chunks { c.validate().unwrap();assert_eq!(c.degree,1); }
            }
        }
    }
    #[test]
    fn star_merges_internal_edges_only_for_nonzero_fill() {
        let graph=fixture(&[[0.,6.],[4.,-6.],[-6.,2.],[6.,2.],[-4.,-6.],[0.,6.]]);
        let nonzero=boundaries(&graph,FillRule::NonZero,100000).unwrap();
        assert_eq!(nonzero.len(),1);
        assert_eq!(nonzero[0].len(),10);
        let evenodd=boundaries(&graph,FillRule::EvenOdd,100000).unwrap();
        assert_eq!(evenodd.len(),5);
        assert!(evenodd.iter().all(|cycle|cycle.len()==3));
        let walks=classify(&graph,FillRule::NonZero,100000).unwrap();
        assert!(walks.iter().any(|w| w.winding.abs()==2 && w.filled));
    }
    #[test]
    fn nested_boundaries_follow_orientation_and_fill_rule() {
        let outer=[[0.,0.],[10.,0.],[10.,10.],[0.,10.],[0.,0.]];
        let inner=[[2.,2.],[8.,2.],[8.,8.],[2.,8.],[2.,2.]];
        let same=combine(fixture(&outer),fixture(&inner));
        assert_eq!(boundaries(&same,FillRule::NonZero,10000).unwrap().len(),1);
        assert_eq!(boundaries(&same,FillRule::EvenOdd,10000).unwrap().len(),2);
        let hole=combine(fixture(&outer),fixture(&inner.into_iter().rev().collect::<Vec<_>>()));
        for rule in [FillRule::NonZero,FillRule::EvenOdd] {
            let result=boundaries(&hole,rule,10000).unwrap();
            assert_eq!(result.len(),2);
            let areas:Vec<_>=result.iter().map(|c| (0..c.len()).map(|i| {
                let a=hole.vertices[c[i]].point;let b=hole.vertices[c[(i+1)%c.len()]].point;a[0]*b[1]-a[1]*b[0]
            }).sum::<f64>()).collect();
            assert!(areas.iter().any(|a|*a>0.) && areas.iter().any(|a|*a<0.));
        }
    }
    #[test]
    fn extracted_boundaries_close_and_keep_both_lobes() {
        for (points, count, lengths) in [
            (vec![[0.,0.],[4.,0.],[4.,4.],[0.,4.],[0.,0.]],1,vec![4]),
            (vec![[0.,0.],[4.,4.],[0.,4.],[4.,0.],[0.,0.]],2,vec![3,3]),
        ] {
            for p in [points.clone(),points.into_iter().rev().collect()] {
                let graph=fixture(&p);
                let result=boundaries(&graph,FillRule::NonZero,10000).unwrap();
                assert_eq!(result.len(),count);
                let mut sizes:Vec<_>=result.iter().map(Vec::len).collect();sizes.sort();assert_eq!(sizes,lengths);
                for cycle in result { for i in 0..cycle.len() {
                    assert!(graph.edges.iter().any(|e| e.vertices==[cycle[i],cycle[(i+1)%cycle.len()]] || e.vertices==[cycle[(i+1)%cycle.len()],cycle[i]]));
                }}
            }
        }
    }
    #[test]
    fn reversal_preserves_fill_and_exterior_is_empty() {
        let points = [[0.,0.],[4.,0.],[4.,4.],[0.,4.],[0.,0.]];
        for (p,sign) in [(points.to_vec(),1),(points.into_iter().rev().collect(),-1)] {
            let result=classify(&fixture(&p),FillRule::NonZero,10000).unwrap();
            assert_eq!(result.len(),2);
            assert_eq!(result.iter().filter(|w|w.filled).count(),1);
            assert_eq!(result.iter().find(|w|w.filled).unwrap().winding,sign);
            assert_eq!(result.iter().find(|w|!w.filled).unwrap().winding,0);
        }
    }
    #[test]
    fn both_crossed_lobes_are_filled_and_budget_failure_is_explicit() {
        let graph=fixture(&[[0.,0.],[4.,4.],[0.,4.],[4.,0.],[0.,0.]]);
        for rule in [FillRule::NonZero,FillRule::EvenOdd] {
            let result=classify(&graph,rule,10000).unwrap();
            let mut filled: Vec<_>=result.iter().filter(|w|w.filled).map(|w|w.winding).collect();
            filled.sort(); assert_eq!(filled,vec![-1,1]);
            assert_eq!(result.iter().filter(|w|!w.filled).count(),1);
        }
        assert!(classify(&graph,FillRule::NonZero,1).is_err());
    }
}
