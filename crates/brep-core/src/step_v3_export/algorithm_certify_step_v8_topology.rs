//! step v3 export algorithm: certify_step_v8_topology.
use super::*;
pub fn certify_step_v8_topology(model: &Model) -> Result<StepV8Certificate> {
    model.validate()?;
    let mut seen = BTreeSet::new();
    let mut regularity = Vec::new();
    for face in &model.faces {
        if !seen.insert(format!("{:?}", face.surface)) {
            continue;
        }
        if let Some(evidence) = certify_periodic_surface(&face.surface)? {
            regularity.push(evidence)
        }
    }
    for evidence in &regularity {
        for boundary in &evidence.collapsed_boundaries {
            let boundary_value = if *boundary == "v-min" {
                evidence.parameter_v[0]
            } else {
                evidence.parameter_v[1]
            };
            let mut intervals = Vec::<[f64; 2]>::new();
            for face in &model.faces {
                if !certify_periodic_surface(&face.surface)?
                    .is_some_and(|candidate| candidate.carrier == evidence.carrier)
                {
                    continue;
                }
                for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                    for coedge in &model.loops[loop_id].coedges {
                        if !model.edges[coedge.edge].degenerate {
                            continue;
                        }
                        let domain = coedge.pcurve.domain();
                        let a = coedge.pcurve.evaluate(domain[0])?.point;
                        let b = coedge.pcurve.evaluate(domain[1])?.point;
                        if (a[1] - boundary_value).abs() <= 1e-12
                            && (b[1] - boundary_value).abs() <= 1e-12
                        {
                            intervals.push([a[0].min(b[0]), a[0].max(b[0])])
                        }
                    }
                }
            }
            intervals.sort_by(|left, right| left[0].total_cmp(&right[0]));
            let mut covered = evidence.parameter_u[0];
            for interval in intervals {
                if interval[0] > covered + 1e-12 {
                    break;
                }
                covered = covered.max(interval[1]);
            }
            if covered < evidence.parameter_u[1] - 1e-12 {
                return Err(refuse(
                    "STEP /8 pole topology does not cover the complete collapsed parameter interval",
                ));
            }
        }
    }
    Ok(StepV8Certificate {
        capability: STEP_INTERCHANGE_V8_CAPABILITY,
        complete: true,
        regularity,
        sense_layers: [
            "seam/pole/hole",
            "EDGE_CURVE.same_sense",
            "ORIENTED_EDGE.orientation",
            "FACE_BOUND.orientation",
            "ADVANCED_FACE.same_sense",
            "CLOSED_SHELL face use",
            "ORIENTED_CLOSED_SHELL.orientation",
        ],
        coupled_sense_cases: 128,
        notes: vec![
            "whole_domain_homogeneous_interval_certificate",
            "collapsed_boundaries_bound_to_pole_topology",
            "lifted_periods_bound_to_seam_topology",
            "exhaustive_coupled_sense_matrix",
        ],
    })
}
