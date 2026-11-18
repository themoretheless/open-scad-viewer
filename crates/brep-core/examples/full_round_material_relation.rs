//! Reproducible exact-source diagnostic; an error is unproven, never a pass.
use brep_core::{feature_ops::full_round_curved::build, profile_certificate::material_relation::Limits};
use nurbs_core::curve::Curve;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let weight = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0.7".into())
        .parse::<f64>()?;
    let c = |p: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: p.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., weight, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    let source = brep_core::prism::extrude(
        &[vec![
            line([0., 0.], [4., 0.]),
            c([[4., 0.], [5., 4.], [4., 8.]]),
            line([4., 8.], [0., 8.]),
            c([[0., 8.], [-1., 4.], [0., 0.]]),
        ]],
        0.,
        6.,
    )?;
    let round = build(&source, [3, 2, 1])?;
    let r = match round.inspect_source_profile_relation(1e-12, Limits::default()) {
        Ok(r) => r,
        Err(error) => {
            let profiles = |m: &brep_core::Model| {
                let cap = m
                    .faces
                    .iter()
                    .find(|f| {
                        f.surface
                            .control_points
                            .iter()
                            .flatten()
                            .all(|p| p[2] == 6.)
                    })
                    .unwrap();
                std::iter::once(cap.outer)
                    .chain(cap.holes.iter().copied())
                    .map(|w| {
                        m.loops[w]
                            .coedges
                            .iter()
                            .map(|c| {
                                let mut curve = m.edges[c.edge].curve.clone();
                                for p in &mut curve.control_points {
                                    p.truncate(2);
                                }
                                if c.reversed {
                                    curve.control_points.reverse();
                                    curve.weights.reverse();
                                    curve.knots = curve.knots.iter().rev().map(|k| -k).collect();
                                }
                                curve
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            };
            let old = profiles(&source);
            let new = profiles(round.model());
            let curves = old
                .iter()
                .chain(&new)
                .flatten()
                .cloned()
                .collect::<Vec<_>>();
            'pairs: for a in 0..curves.len() {
                for b in a..curves.len() {
                    if let Err(e) = brep_core::profile_certificate::certify_span_intersections(
                        &curves[a],
                        &curves[b],
                        a == b,
                        1e-12,
                    ) {
                        eprintln!(
                            "unproved_pair=({a},{b}) source_curves={} total={} degrees=({},{}) cause={}",
                            old.iter().map(Vec::len).sum::<usize>(),
                            curves.len(),
                            curves[a].degree,
                            curves[b].degree,
                            e
                        );
                        eprintln!("first_curve={:?}\nsecond_curve={:?}", curves[a], curves[b]);
                        break 'pairs;
                    }
                }
            }
            return Err(Box::new(error));
        }
    };
    println!(
        "complete=true subset={} edges={} added={} removed={}",
        r.relation().result_is_subset(),
        r.relation().edge_count(),
        r.relation().added().len(),
        r.relation().removed().len()
    );
    if let Some(w) = r.relation().added().first() {
        println!("added_witness={w:?}");
    }
    let _admitted = r.require_no_addition()?;
    println!("exact_no_addition=true");
    Ok(())
}
