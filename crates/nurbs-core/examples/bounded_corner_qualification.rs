//! Fresh source-cell diagnostics at the user and strip-factory tolerances.
//! This admits neither the central surface nor a complete CAD body.
use nurbs_core::{
    Result, analysis::bezier_curvature_bound as bounds, construct::curves::bounded_corner,
    curve::Curve,
};
fn main() -> Result<()> {
    let quartic = std::env::args().any(|a| a == "--quartic");
    let smooth = quartic || std::env::args().any(|a| a == "--c2");
    let mut checked = 0;
    let mut refused = false;
    for tolerance in [1e-7, 2.5e-8] {
        for radius in [0.1, 0.2, 0.35] {
            let p = bounded_corner::propose(radius, 10., tolerance)?;
            let c = if quartic {
                p.quartic_g2()?
            } else if smooth {
                p.composite_matched_g2()?
            } else {
                p.composite_cubic()?
            };
            if smooth && !quartic {
                let s = nurbs_core::surface::Surface {
                    degree_u: c.degree,
                    degree_v: 1,
                    knots_u: c.knots.clone(),
                    knots_v: vec![0., 0., 1., 1.],
                    control_points: c
                        .control_points
                        .iter()
                        .map(|p| vec![p.clone(), vec![p[0], p[1], 1.]])
                        .collect(),
                    weights: c.weights.iter().map(|w| vec![*w; 2]).collect(),
                    periodic_u: false,
                    periodic_v: false,
                };
                for knot in [c.knots[c.degree + 1], c.knots[2 * c.degree + 1]] {
                    let r = nurbs_core::surface_g2_pcurves::inspect_original_geometric_span_join(
                        &s, knot, 1e-5, 1e-4, 6000,
                    )?;
                    println!(
                        "tolerance={tolerance} radius={radius} join={knot} decision={:?} normal={:?} tensor={:?} regular={} pending={} cells={}",
                        r.decision, r.normal, r.tensor, r.regularity_proven, r.pending, r.cells
                    );
                    refused |= r.decision != nurbs_core::surface_join::Decision::WithinTolerance
                        || !r.regularity_proven
                        || r.pending != 0;
                }
            }
            let mut sources = p.parts.to_vec();
            for start in if quartic {
                vec![0]
            } else {
                vec![0, c.degree, 2 * c.degree]
            } {
                sources.push(Curve {
                    degree: c.degree,
                    knots: [vec![0.; c.degree + 1], vec![1.; c.degree + 1]].concat(),
                    control_points: c.control_points[start..start + c.degree + 1].to_vec(),
                    weights: c.weights[start..start + c.degree + 1].to_vec(),
                    periodic: false,
                });
            }
            for (part, source) in sources.iter().enumerate() {
                let r = bounds::inspect(source, 10., 4096, 1000000)?;
                println!(
                    "tolerance={tolerance} radius={radius} part={part} outcome={:?} work={} cells={}",
                    r.decision.outcome, r.decision.work_used, r.decision.cells
                );
                if r.certificate.is_none() {
                    refused = true;
                    println!(
                        "unproven witness={:?} controls={:?} weights={:?}",
                        r.decision.witness_parameter, source.control_points, source.weights
                    );
                }
                checked += 1;
            }
        }
    }
    if refused {
        return Err(nurbs_core::Error::new(
            "CORNER_PROFILE_UNPROVEN",
            "An original profile cap or G2 join remains unproven",
        ));
    }
    println!("complete original_cells={checked} maximum_per_mm=10; no body admission");
    Ok(())
}
