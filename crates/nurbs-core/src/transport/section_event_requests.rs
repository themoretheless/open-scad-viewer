use super::*;
use section::AlgebraicBoundaryClock as Clock;
pub(super) fn clock(p: Clock) -> Value {
    match p {
        Clock::Rational(q) => json!({"kind":"rational","value":q.to_string()}),
        Clock::Quadratic {
            linear,
            constant,
            upper_root,
        } => {
            json!({"kind":"quadratic","linear":linear.to_string(),"constant":constant.to_string(),"upperRoot":upper_root})
        }
    }
}
pub(super) fn measure(
    v: &Value,
    op: &str,
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    work: u64,
    prior: u64,
) -> Result<Value> {
    use section::PlanarSectionOutcome::*;
    let section = section::partition_quadratic_planar_section(left, right, work)?;
    let mut used = section.work_used;
    let tol: Option<f64> = if op == "enclose_face_events" {
        Some(field(v, "xyz_tolerance")?)
    } else {
        None
    };
    let mut out = match section.outcome {
        Transverse(s) => {
            let mut events = Vec::with_capacity(s.events.len());
            for e in s.events {
                let mut item = json!({"onBoundary":e.on_boundary,"parameterBounds":e.parameter_bounds.map(|x|x.to_string())});
                if let Some(t) = tol {
                    if used == work {
                        return Ok(
                            json!({"status":"refused","reason":"ResourceLimit","workUsed":prior+used}),
                        );
                    }
                    let r = section::enclose_algebraic_planar_boundary_point(
                        &s.line,
                        &e.clock,
                        t,
                        work - used,
                    )?;
                    used += r.work_used;
                    match r.outcome {
                        Ok(b) => {
                            item["enclosure"] = json!({"xyz":b.xyz.map(|a|a.map(|q|q.to_string())),"parameter":b.parameter.map(|q|q.to_string()),"diameterSquared":b.diameter_squared.to_string()});
                        }
                        Err(e) => {
                            return Ok(
                                json!({"status":"refused","reason":format!("{e:?}"),"workUsed":prior+used}),
                            );
                        }
                    }
                }
                item["clock"] = clock(e.clock);
                events.push(item);
            }
            json!({"status":"transverse","line":line(s.line),"events":events,"samples":s.samples.iter().map(ToString::to_string).collect::<Vec<_>>()})
        }
        Parallel => json!({"status":"parallel"}),
        Coincident => json!({"status":"coincident"}),
        Refused(e) => json!({"status":"refused","reason":format!("{e:?}")}),
    };
    out["workUsed"] = json!(prior + used);
    Ok(out)
}
