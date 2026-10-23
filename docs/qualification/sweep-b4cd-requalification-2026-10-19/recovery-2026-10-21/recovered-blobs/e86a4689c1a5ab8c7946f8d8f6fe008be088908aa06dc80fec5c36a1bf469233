//! Assemble an unambiguous cycle of open polylines without moving input points.
use super::{Result, Value, field, input};
use nurbs_core::curve::Curve;
use value_codec::json;
type P = [f64; 2];
fn contour_defect(points: &[P]) -> Option<Value> {
    let defect = |kind: &str, indices: Vec<usize>| {
        Some(
            json!({"kind":kind,"segments":indices.into_iter().map(|i|json!({"index":i,"a":points[i],"b":points[(i+1)%points.len()]})).collect::<Vec<_>>()}),
        )
    };
    use cad_predicates::{
        AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
        orient2d,
    };
    let Ok(arena) = SourceArena::authored(
        "represented-profile",
        1,
        points
            .iter()
            .flatten()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    ) else {
        return defect("unproven", (0..points.len()).collect());
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let pair = |i: usize| [arena.leaf(2 * i).unwrap(), arena.leaf(2 * i + 1).unwrap()];
    let mut sign = |a, b, c| {
        orient2d(&mut ctx, pair(a), pair(b), pair(c))
            .ok()
            .and_then(|d| match d.outcome {
                Outcome::Sign(s) => Some(s),
                _ => None,
            })
    };
    let n = points.len();
    let mut noncollinear = false;
    for i in 0..n {
        let a = (i + n - 1) % n;
        let c = (i + 1) % n;
        match sign(a, i, c) {
            Some(Sign::Zero) => {
                let k = if points[a][0] != points[i][0] { 0 } else { 1 };
                if (points[a][k] > points[i][k]) == (points[c][k] > points[i][k]) {
                    return defect("overlap", vec![a, i]);
                }
            }
            Some(_) => noncollinear = true,
            None => return defect("unproven", vec![a, i]),
        }
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let k = (j + 1) % n;
            if (0..2).any(|axis| {
                points[i][axis].max(points[c][axis]) < points[j][axis].min(points[k][axis])
                    || points[j][axis].max(points[k][axis]) < points[i][axis].min(points[c][axis])
            }) {
                continue;
            }
            let q = [sign(i, c, j), sign(i, c, k), sign(j, k, i), sign(j, k, c)];
            if q.iter().any(Option::is_none) {
                return defect("unproven", vec![i, j]);
            }
            let straddle = |a: Option<Sign>, b: Option<Sign>| {
                a == Some(Sign::Zero) || b == Some(Sign::Zero) || a != b
            };
            if straddle(q[0], q[1]) && straddle(q[2], q[3]) {
                return defect("intersection", vec![i, j]);
            }
        }
    }
    if noncollinear {
        None
    } else {
        defect("degenerate", (0..n).collect())
    }
}
pub fn prepare(v: Value) -> Result<Value> {
    assemble(v, None)
}
fn assemble(v: Value, curves: Option<Vec<Vec<Curve>>>) -> Result<Value> {
    let chains: Vec<Vec<P>> = field(&v, "chains")?;
    let tolerance: f64 = field(&v, "tolerance")?;
    if chains.is_empty()
        || chains.len() > 128
        || !tolerance.is_finite()
        || !(0. ..=1e6).contains(&tolerance)
    {
        return Err(input("Invalid profile preparation inputs."));
    }
    if chains.iter().any(|c| c.len() < 2)
        || chains.iter().map(Vec::len).sum::<usize>() > 512
        || chains
            .iter()
            .flatten()
            .flatten()
            .any(|x| !x.is_finite() || x.abs() > 1e6)
    {
        return Err(input("Profile preparation requires 2–512 finite points."));
    }
    if chains.iter().any(|c| c.windows(2).any(|w| w[0] == w[1])) {
        return Err(input("Profile has a zero-length edge."));
    }
    let ends: Vec<P> = chains.iter().flat_map(|c| [c[0], c[c.len() - 1]]).collect();
    let mut links = vec![Vec::new(); ends.len()];
    for i in 0..ends.len() {
        for j in i + 1..ends.len() {
            let dx = (ends[i][0] - ends[j][0]).abs().next_up();
            let dy = (ends[i][1] - ends[j][1]).abs().next_up();
            // Exact coincidences need no connector; nonzero distances use an upward
            // guard for floating point subtraction and hypot at the admission boundary.
            let bound = if ends[i] == ends[j] {
                0.
            } else {
                ((dx * dx).next_up() + (dy * dy).next_up())
                    .next_up()
                    .sqrt()
                    .next_up()
            };
            if bound <= tolerance {
                links[i].push(j);
                links[j].push(i);
            }
        }
    }
    let defects:Vec<Value>=links.iter().enumerate().filter(|(_,l)|l.len()!=1).map(|(i,l)|json!({"chain":i/2,"end":if i%2==0{"start"}else{"end"},"point":ends[i],"kind":if l.is_empty(){"gap"}else{"ambiguous"},"candidates":l})).collect();
    if !defects.is_empty() {
        return Ok(
            json!({"accepted":false,"reason":"endpoint-topology","points":[],"defects":defects,"connectors":[]}),
        );
    }
    let mut wire = Vec::new();
    let mut visited = vec![false; chains.len()];
    let mut entry = 0;
    let mut points = Vec::new();
    let mut connectors = Vec::new();
    loop {
        let chain = entry / 2;
        if visited[chain] {
            break;
        }
        visited[chain] = true;
        let ordered: Vec<P> = if entry % 2 == 0 {
            chains[chain].clone()
        } else {
            chains[chain].iter().rev().copied().collect()
        };
        if let Some(ref curves) = curves {
            if entry % 2 == 0 {
                wire.extend(curves[chain].clone());
            } else {
                for c in curves[chain].iter().rev() {
                    wire.push(c.reverse()?);
                }
            }
        }
        for p in ordered {
            if points.last() != Some(&p) {
                points.push(p);
            }
        }
        let exit = entry ^ 1;
        let next = links[exit][0];
        if ends[exit] != ends[next] {
            connectors.push(json!({"a":ends[exit],"b":ends[next]}));
            if curves.is_some() {
                wire.push(Curve::from_polyline(vec![
                    ends[exit].to_vec(),
                    ends[next].to_vec(),
                ])?);
            }
        }
        entry = next;
    }
    if entry != 0 || visited.iter().any(|x| !*x) {
        return Ok(
            json!({"accepted":false,"reason":"disconnected","points":[],"defects":[],"connectors":connectors}),
        );
    }
    if curves.is_some() {
        let loops = match brep_core::planar_trim::orient_even_odd(&[wire], 1e-7) {
            Ok(loops) => loops,
            Err(e) => {
                return Ok(
                    json!({"accepted":false,"reason":"invalid-contour","points":[],"defects":[],"connectors":connectors,"detail":e.to_string()}),
                );
            }
        };
        let profile = super::brep_profile::profile(loops, 1e-7)?;
        return Ok(
            json!({"accepted":true,"reason":"accepted","points":[],"defects":[],"connectors":connectors,"profile":profile}),
        );
    }
    if points.first() == points.last() {
        points.pop();
    }
    if points.len() < 3
        || points.len() > 512
        || points
            .iter()
            .enumerate()
            .any(|(i, p)| *p == points[(i + 1) % points.len()])
    {
        return Ok(
            json!({"accepted":false,"reason":"invalid-contour","points":[],"defects":[],"connectors":connectors}),
        );
    }
    if let Some(defect) = contour_defect(&points) {
        return Ok(
            json!({"accepted":false,"reason":"invalid-contour","points":[],"defects":[],"segmentDefect":defect,"connectors":connectors}),
        );
    }
    Ok(
        json!({"accepted":true,"reason":"accepted","points":points,"defects":[],"connectors":connectors}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run(chains: Value, tolerance: f64) -> Value {
        prepare(json!({"chains":chains,"tolerance":tolerance})).unwrap()
    }
    #[test]
    fn unordered_reversed_cycle() {
        let r = run(
            json!([
                [[1, 1], [1, 0]],
                [[0, 0], [0, 1]],
                [[0, 1], [1, 1]],
                [[1, 0], [0, 0]]
            ]),
            0.,
        );
        assert_eq!(r["accepted"], json!(true));
        assert_eq!(r["points"].as_array().unwrap().len(), 4);
    }
    #[test]
    fn connector_preserves_points() {
        let r = run(json!([[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0.01]]]), 0.02);
        assert_eq!(r["accepted"], json!(true));
        assert_eq!(r["points"].as_array().unwrap().len(), 5);
        assert_eq!(r["connectors"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn gap_and_branch_refuse() {
        assert_eq!(
            run(json!([[[0, 0], [1, 0], [1, 1]]]), 0.)["reason"],
            json!("endpoint-topology")
        );
        assert_eq!(
            run(
                json!([[[0, 0], [1, 0]], [[1, 0], [0, 1]], [[1, 0], [0, 0]]]),
                0.
            )["accepted"],
            json!(false)
        );
    }
    #[test]
    fn rejects_disconnected_cycles() {
        let r = run(
            json!([
                [[0, 0], [1, 0], [0, 1], [0, 0]],
                [[3, 0], [4, 0], [3, 1], [3, 0]]
            ]),
            0.,
        );
        assert_eq!(r["reason"], json!("disconnected"));
    }
    #[test]
    fn rejects_collinear_triangle() {
        assert_eq!(
            run(json!([[[0, 0], [1, 0], [2, 0], [0, 0]]]), 0.)["accepted"],
            json!(false)
        );
    }
    #[test]
    fn rejects_crossing_and_degenerate() {
        assert_eq!(
            run(json!([[[0, 0], [1, 1], [0, 1], [1, 0], [0, 0]]]), 0.)["accepted"],
            json!(false)
        );
        assert_eq!(
            run(json!([[[0, 0], [1, 0], [0, 0]]]), 0.)["accepted"],
            json!(false)
        );
    }
    #[test]
    fn reports_exact_crossing_segments() {
        let r = run(json!([[[0, 0], [2, 2], [0, 2], [2, 0], [0, 0]]]), 0.);
        assert_eq!(r["segmentDefect"]["kind"], json!("intersection"));
        assert_eq!(
            r["segmentDefect"]["segments"][0],
            json!({"index":0,"a":[0.,0.],"b":[2.,2.]})
        );
        assert_eq!(
            r["segmentDefect"]["segments"][1],
            json!({"index":2,"a":[0.,2.],"b":[2.,0.]})
        );
    }
    #[test]
    fn reports_overlapping_adjacent_segments() {
        let r = run(
            json!([[[0, 0], [2, 0], [1, 0], [1, 2], [0, 2], [0, 0]]]),
            0.,
        );
        assert_eq!(r["accepted"], json!(false));
        assert!(r["segmentDefect"]["segments"].as_array().unwrap().len() >= 2);
    }
}

/// Assemble analytic arcs and polylines. Display samples never enter this path.
pub fn prepare_retained(v: Value) -> Result<Value> {
    let sketches: Vec<Value> = field(&v, "sketches")?;
    if sketches.is_empty() || sketches.len() > 128 {
        return Err(input("Select 1–128 open sketches."));
    }
    let mut curves = Vec::new();
    let mut chains = Vec::new();
    for sketch in sketches {
        if field::<bool>(&sketch, "closed")? {
            return Err(input("Profile preparation requires open sketches."));
        }
        let wire = if let Some(arc) = sketch.get("analytic").filter(|a| !a.is_null()) {
            if field::<String>(arc, "kind")? != "arc" {
                return Err(input("Select open arcs or polylines."));
            }
            let center: P = field(arc, "center")?;
            let radius: f64 = field(arc, "radius")?;
            let start: f64 = field(arc, "start")?;
            let sweep: f64 = field(arc, "sweep")?;
            if !center
                .iter()
                .chain([&radius, &start, &sweep])
                .all(|x| x.is_finite())
                || center.iter().any(|x| x.abs() > 1e6)
                || !(0.01..=1e6).contains(&radius)
                || !(0.1..360.).contains(&sweep.abs())
                || start.abs() > 1e6
            {
                return Err(input("Invalid open arc parameters."));
            }
            let count = (sweep.abs() / 90.).ceil() as usize;
            let direction = super::cad_sketch::arc_direction;
            let ends: Vec<P> = (0..=count)
                .map(|i| {
                    let d = direction(if i == count {
                        start + sweep
                    } else {
                        start + sweep * i as f64 / count as f64
                    });
                    [center[0] + radius * d[0], center[1] + radius * d[1]]
                })
                .collect();
            let weight = (sweep.to_radians() / count as f64 / 2.).cos();
            (0..count)
                .map(|i| {
                    let d = direction(start + sweep * (i as f64 + 0.5) / count as f64);
                    let c = Curve {
                        degree: 2,
                        knots: vec![0., 0., 0., 1., 1., 1.],
                        control_points: vec![
                            ends[i].to_vec(),
                            vec![
                                center[0] + radius * d[0] / weight,
                                center[1] + radius * d[1] / weight,
                            ],
                            ends[i + 1].to_vec(),
                        ],
                        weights: vec![1., weight, 1.],
                        periodic: false,
                    };
                    c.validate()?;
                    Ok(c)
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            let points: Vec<P> = field(&sketch, "points")?;
            if points.len() < 2
                || points.len() > 512
                || points
                    .iter()
                    .flatten()
                    .any(|x| !x.is_finite() || x.abs() > 1e6)
                || points.windows(2).any(|w| w[0] == w[1])
            {
                return Err(input("Invalid polyline size."));
            }
            points
                .windows(2)
                .map(|w| Curve::from_polyline(vec![w[0].to_vec(), w[1].to_vec()]))
                .collect::<Result<Vec<_>>>()?
        };
        if wire.is_empty() {
            return Err(input("Empty profile chain."));
        }
        let a = &wire[0].control_points[0];
        let b = wire.last().unwrap().control_points.last().unwrap();
        chains.push(vec![[a[0], a[1]], [b[0], b[1]]]);
        curves.push(wire);
    }
    if curves.iter().map(Vec::len).sum::<usize>() > 256 {
        return Err(input("Profile exceeds 256 spans."));
    }
    assemble(
        json!({"chains":chains,"tolerance":field::<f64>(&v,"tolerance")?}),
        Some(curves),
    )
}

#[cfg(test)]
mod retained_tests {
    use super::*;
    fn arc(start: f64, sweep: f64) -> Value {
        json!({"closed":false,"analytic":{"kind":"arc","center":[0.,0.],"radius":2.,"start":start,"sweep":sweep}})
    }
    fn line(points: Vec<P>) -> Value {
        json!({"closed":false,"points":points})
    }
    #[test]
    fn semicircle_retains_quadratics_and_area() {
        for sweep in [180., -180.] {
            let r = prepare_retained(
                json!({"sketches":[arc(0.,sweep),line(vec![[-2.,0.],[2.,0.]])],"tolerance":0.}),
            )
            .unwrap();
            assert_eq!(r["accepted"], json!(true), "{r:?}");
            let loops: Vec<Vec<Curve>> = field(&r["profile"], "loops").unwrap();
            assert_eq!(loops[0].iter().filter(|c| c.degree == 2).count(), 2);
            assert!(
                (r["profile"]["areaMm2"].as_f64().unwrap() - 2. * std::f64::consts::PI).abs()
                    < 1e-10
            );
        }
    }
    #[test]
    fn unordered_arcs_make_full_circle() {
        let r=prepare_retained(json!({"sketches":[arc(90.,90.),arc(0.,-90.),arc(0.,90.),arc(180.,90.)],"tolerance":0.})).unwrap();
        assert_eq!(r["accepted"], json!(true), "{r:?}");
        assert!(
            (r["profile"]["areaMm2"].as_f64().unwrap() - 4. * std::f64::consts::PI).abs() < 1e-10
        );
    }
    #[test]
    fn noncardinal_ends_retain_shape() {
        for (start, sweep) in [(13., 77.), (-47., 210.), (11., -120.)] {
            let a = |d: f64| {
                let [x, y] = super::super::cad_sketch::arc_direction(d);
                [2. * x, 2. * y]
            };
            let r=prepare_retained(json!({"sketches":[arc(start,sweep),line(vec![a(start+sweep),a(start)])],"tolerance":1e-12})).unwrap();
            assert_eq!(r["accepted"], json!(true), "{r:?}");
            let expected = 2. * (sweep.to_radians() - sweep.to_radians().sin()).abs();
            assert!((r["profile"]["areaMm2"].as_f64().unwrap() - expected).abs() < 1e-9);
        }
    }
    #[test]
    fn tolerance_adds_an_explicit_connector_without_moving_arc() {
        let sketches = json!([arc(0., 180.), line(vec![[-2., -0.01], [2., 0.]])]);
        let refused = prepare_retained(json!({"sketches":sketches,"tolerance":0.009})).unwrap();
        assert_eq!(refused["reason"], json!("endpoint-topology"));
        let r = prepare_retained(json!({"sketches":sketches,"tolerance":0.011})).unwrap();
        assert_eq!(r["accepted"], json!(true), "{r:?}");
        assert_eq!(r["connectors"], json!([{"a":[-2.,0.],"b":[-2.,-0.01]}]));
        assert!(
            (r["profile"]["areaMm2"].as_f64().unwrap() - (2. * std::f64::consts::PI + 0.02)).abs()
                < 1e-10
        );
    }
    #[test]
    fn gaps_branches_and_intersections_refuse() {
        let a = arc(0., 180.);
        let gap = prepare_retained(json!({"sketches":[a],"tolerance":0.})).unwrap();
        assert_eq!(gap["reason"], json!("endpoint-topology"));
        let branch=prepare_retained(json!({"sketches":[a,line(vec![[-2.,0.],[2.,0.]]),line(vec![[-2.,0.],[0.,0.],[2.,0.]])],"tolerance":0.})).unwrap();
        assert_eq!(branch["reason"], json!("endpoint-topology"));
        let crossing = prepare_retained(
            json!({"sketches":[a,line(vec![[-2.,0.],[0.,3.],[2.,0.]])],"tolerance":0.}),
        )
        .unwrap();
        assert_eq!(crossing["reason"], json!("invalid-contour"));
    }
}

#[cfg(test)]
mod sample_tests {
    use super::*;
    #[test]
    fn snapped_display_endpoints_join_without_connectors() {
        for (start, sweep) in [
            (0., 180.),
            (-47., 210.),
            (90., -90.),
            (13.17, 137.23),
            (-8.31, -67.19),
            (0.1, 100.1),
            (0., 27.06),
        ] {
            let arc =
                json!({"kind":"arc","center":[3.,7.],"radius":2.,"start":start,"sweep":sweep});
            let samples: Vec<P> =
                value_codec::from_value(super::super::cad_sketch::sample(arc.clone()).unwrap())
                    .unwrap();
            let r=prepare_retained(json!({"sketches":[{"closed":false,"analytic":arc},{"closed":false,"points":[samples.last().unwrap(),samples.first().unwrap()]}],"tolerance":0.})).unwrap();
            assert_eq!(r["accepted"], json!(true), "{r:?}");
            assert_eq!(r["connectors"], json!([]));
        }
    }
}
