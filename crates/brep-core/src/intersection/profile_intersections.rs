//! Intersection events between distinct retained XY profile segments.
//! Coverage is pair-scoped; this is not a self-intersection absence certificate.
use crate::{Result, invalid};
use cad_predicates::ToleranceContext;
use nurbs_core::{curve::Curve, intersection};
use value_codec::{Value, json};
pub fn inspect(
    loops: &[Vec<Curve>],
    tolerance_mm: f64,
    max_pairs: usize,
    max_boxes: usize,
) -> Result<Value> {
    let count = loops.iter().map(Vec::len).sum::<usize>();
    if loops.is_empty()
        || loops.len() > 64
        || count > 254
        || count < 2
        || loops.iter().any(Vec::is_empty)
        || loops
            .iter()
            .flatten()
            .map(|c| c.control_points.len())
            .sum::<usize>()
            > 8192
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || (1..=10000).contains(&max_pairs) == false
        || (1..=1000000).contains(&max_boxes) == false
    {
        return Err(invalid(
            "Profile intersection inputs or work limits are invalid",
        ));
    }
    let tolerance = ToleranceContext::from_brep_tolerance_mm(tolerance_mm)
        .map_err(|_| invalid("Invalid profile intersection tolerance"))?;
    let mut sources = Vec::new();
    for (loop_index, wire) in loops.iter().enumerate() {
        for (curve_index, c) in wire.iter().enumerate() {
            c.validate()?;
            if c.control_points
                .iter()
                .any(|p| p.len() != 2 || p.iter().any(|x| x.abs() > 1e6))
            {
                return Err(invalid("Profile intersections require bounded XY controls"));
            }
            let mut lifted = c.clone();
            for p in &mut lifted.control_points {
                p.push(0.);
            }
            sources.push((loop_index, curve_index, lifted));
        }
    }
    let total_pairs = count * (count - 1) / 2;
    let mut pairs = Vec::new();
    let (mut boxes, mut resolved) = (0, 0);
    'search: for a in 0..count {
        for b in a + 1..count {
            if pairs.len() == max_pairs || boxes == max_boxes {
                break 'search;
            }
            let report = intersection::intersect_curve_curve_bounded(
                &sources[a].2,
                &sources[b].2,
                Some(
                    ToleranceContext::new(tolerance.specification().clone())
                        .map_err(|_| invalid("Invalid profile intersection tolerance"))?,
                ),
                (max_boxes - boxes).min(8192),
            )?;
            let used = report["coverage"]["boxesVisited"]
                .as_u64()
                .ok_or_else(|| invalid("Invalid intersection work report"))?
                as usize;
            boxes += used;
            if report["coverage"]["complete"].as_bool() == Some(true) {
                resolved += 1;
            }
            pairs.push(json!({"first":{"loop":sources[a].0,"curve":sources[a].1},"second":{"loop":sources[b].0,"curve":sources[b].1},"report":report}));
        }
    }
    Ok(
        json!({"scope":"distinct-profile-segment-pairs","complete":pairs.len()==total_pairs&&resolved==total_pairs,
  "totalPairs":total_pairs,"visitedPairs":pairs.len(),"resolvedPairs":resolved,"unvisitedPairs":total_pairs-pairs.len(),
  "boxesVisited":boxes,"maxPairs":max_pairs,"maxBoxes":max_boxes,"toleranceMm":tolerance_mm,"pairs":pairs}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
        Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
    }
    #[test]
    fn crossing_original_rational_curve_reports_source_indices() {
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.], vec![2., 0.]],
            weights: vec![1., 0.8, 1.],
            periodic: false,
        };
        let input = vec![vec![c], vec![line([1., -1.], [1., 2.])]];
        let before = value_codec::to_string(&input).unwrap();
        let r = inspect(&input, 1e-7, 100, 8192).unwrap();
        assert_eq!(r["visitedPairs"].as_u64(), Some(1));
        assert_eq!(r["complete"].as_bool(), Some(true));
        assert_eq!(r["pairs"][0]["first"], json!({"loop":0,"curve":0}));
        assert_eq!(r["pairs"][0]["second"], json!({"loop":1,"curve":0}));
        let points = r["pairs"][0]["report"]["components"].as_array().unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0]["kind"].as_str(), Some("point"));
        assert!((points[0]["point"][1].as_f64().unwrap() - 4. / 9.).abs() < 1e-7);
        assert_eq!(value_codec::to_string(&input).unwrap(), before);
    }
    #[test]
    fn pair_and_box_limits_are_explicit_and_never_overrun() {
        let input = vec![vec![
            line([0., 0.], [2., 2.]),
            line([0., 2.], [2., 0.]),
            line([3., 0.], [3., 2.]),
        ]];
        let r = inspect(&input, 1e-7, 1, 8192).unwrap();
        assert_eq!(r["totalPairs"].as_u64(), Some(3));
        assert_eq!(r["unvisitedPairs"].as_u64(), Some(2));
        assert_eq!(r["complete"].as_bool(), Some(false));
        let r = inspect(&input, 1e-7, 100, 1).unwrap();
        assert!(r["boxesVisited"].as_u64().unwrap() <= 1);
        assert_eq!(r["complete"].as_bool(), Some(false));
    }
}
