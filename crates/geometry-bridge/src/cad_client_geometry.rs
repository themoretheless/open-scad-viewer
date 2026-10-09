//! Native adapters for client proposals and numerical witness validation.
use super::{Result, Value, field, input};
use math_core::profile_plane::Plane;
use value_codec::json;
fn plane(v: &Value) -> Result<Plane> {
    Ok(Plane {
        origin: field(v, "origin")?,
        u: field(v, "u")?,
        v: field(v, "v")?,
    })
}
fn plane_json(p: Plane) -> Value {
    json!({"origin":p.origin,"u":p.u,"v":p.v})
}
pub fn run(v: Value) -> Result<Value> {
    let operation: String = field(&v, "operation")?;
    match operation.as_str() {
        "distanceWitness" => Ok(json!(math_core::witness_bounds::distance_consistent(
            field(&v, "points")?,
            field(&v, "bounds")?,
            field(&v, "lo")?,
            field(&v, "hi")?,
            field(&v, "strictUpper")?
        ))),
        "materialWitness" => Ok(json!(math_core::witness_bounds::material_consistent(
            field(&v, "bounds")?,
            field(&v, "lo")?,
            field(&v, "hi")?
        ))),
        "projectiveCandidates" => {
            let surfaces: Vec<nurbs_core::surface::Surface> = field(&v, "surfaces")?;
            if surfaces.len() > 10000 {
                return Err(input("Invalid source surface count"));
            }
            Ok(json!(
                surfaces
                    .iter()
                    .map(nurbs_core::surface_injectivity::projective_projection_candidates)
                    .collect::<Result<Vec<_>>>()?
            ))
        }
        "quotientWitness" => {
            let bounds: [f64; 4] = field(&v, "bounds")?;
            let margin: f64 = field(&v, "margin")?;
            Ok(json!(math_core::witness_bounds::quotient_consistent(
                bounds, margin
            )))
        }
        "polarCandidates" => {
            let s: nurbs_core::surface::Surface = field(&v, "surface")?;
            s.validate()?;
            Ok(json!(
                nurbs_core::surface_injectivity::polar_projection_candidates(&s)?
            ))
        }
        "numericRectangle" => Ok(json!(planar_geometry::sketch_shapes::numeric_rectangle(
            field(&v, "origin")?,
            field(&v, "size")?
        ))),
        "numericSlot" => {
            let Some((a, b, radius, angle)) = planar_geometry::sketch_shapes::slot_caps(
                field(&v, "a")?,
                field(&v, "b")?,
                field(&v, "width")?,
            ) else {
                return Ok(Value::Null);
            };
            let cap_b =
                json!({"kind":"arc","center":b,"radius":radius,"start":angle-90.,"sweep":180.});
            let cap_a =
                json!({"kind":"arc","center":a,"radius":radius,"start":angle+90.,"sweep":180.});
            let points_b = planar_geometry::sketch_shapes::sample(
                planar_geometry::sketch_shapes::CurveKind::Arc,
                b,
                radius,
                angle - 90.,
                180.,
            )?;
            let points_a = planar_geometry::sketch_shapes::sample(
                planar_geometry::sketch_shapes::CurveKind::Arc,
                a,
                radius,
                angle + 90.,
                180.,
            )?;
            Ok(
                json!({"capA":cap_a,"capB":cap_b,"rightA":points_a.last().unwrap(),"leftA":points_a[0],"rightB":points_b[0],"leftB":points_b.last().unwrap()}),
            )
        }
        "profileProjection" => {
            let values: Vec<Value> = field(&v, "curves")?;
            if values.is_empty() || values.len() > 256 {
                return Err(input("Invalid profile inputs"));
            }
            let curves = values
                .iter()
                .map(|c| {
                    super::request_codec::field::<nurbs_core::curve::Curve>(
                        &json!({"curve":c}),
                        "curve",
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let controls = curves
                .iter()
                .map(|c| {
                    c.control_points
                        .iter()
                        .map(|p| [p[0], p[1], p.get(2).copied().unwrap_or(0.)])
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let provided: Option<Value> = field(&v, "plane")?;
            let p = if let Some(p) = provided {
                plane(&p)?
            } else {
                math_core::profile_plane::infer(
                    &controls.iter().flatten().copied().collect::<Vec<_>>(),
                )?
            };
            let mut error = 0f64;
            let mut projected = Vec::new();
            for (mut value, points) in values.into_iter().zip(controls) {
                let (points, deviation) = math_core::profile_plane::project(&points, p)?;
                error = error.max(deviation);
                value
                    .as_object_mut()
                    .ok_or_else(|| input("Invalid profile curve"))?
                    .insert("controlPoints".into(), json!(points));
                projected.push(value);
            }
            Ok(json!({"plane":plane_json(p),"curves":projected,"projectionMaxDeviationMm":error}))
        }
        "sourceDisplayBudget" => {
            let edges: usize = field(&v, "edges")?;
            let faces: usize = field(&v, "faces")?;
            if edges > 65536 {
                return Err(input("Source edge display exceeds 65536 edges per body"));
            }
            if faces == 0 || faces > 65536 {
                return Err(input("Invalid source face display budget"));
            }
            let divisions = (1usize..=8)
                .take_while(|d| d * d <= 65536 / faces)
                .last()
                .unwrap_or(0);
            Ok(
                json!({"displaySegments":32usize.min(65536/edges.max(1)),"divisions":divisions,"domainCellsPerFace":10000usize.min(1000000/faces)}),
            )
        }
        "wallCandidates" | "wallPlan" => {
            let model: brep_core::Model = field(&v, "model")?;
            let plan = brep_core::wall_search::plan(
                &model,
                field(&v, "groups")?,
                field(&v, "maxCandidates")?,
            )?;
            if operation == "wallPlan" {
                return Ok(
                    json!({"targets":plan.targets.iter().map(|q|json!({"face":q.face,"uv":q.uv})).collect::<Vec<_>>(),"sources":plan.sources.iter().map(|q|json!({"face":q.face,"uv":q.uv})).collect::<Vec<_>>() }),
                );
            }
            let supplied: Option<Value> = field(&v, "samples")?;
            let samples = |v: &Value, key: &str| -> Result<Vec<brep_core::wall_search::Sample>> {
                field::<Vec<Value>>(v, key)?
                    .iter()
                    .map(|s| {
                        Ok(brep_core::wall_search::Sample {
                            point: field(s, "point")?,
                            normal: field(s, "normal")?,
                        })
                    })
                    .collect()
            };
            let (targets, sources) = if let Some(s) = supplied {
                (samples(&s, "targets")?, samples(&s, "sources")?)
            } else {
                (
                    brep_core::wall_search::evaluate(&model, &plan.targets)?,
                    brep_core::wall_search::evaluate(&model, &plan.sources)?,
                )
            };
            Ok(json!(brep_core::wall_search::choose(&plan,&targets,&sources)?.iter().map(|c|json!({"face":c.face,"uv":c.uv,"origin":c.origin,"direction":c.direction})).collect::<Vec<_>>()))
        }
        _ => Err(input("Unknown client geometry operation")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_shapes_preserve_bounds() {
        assert_eq!(
            run(json!({"operation":"numericRectangle","origin":[999999.,0.],"size":[2.,1.]}))
                .unwrap(),
            Value::Null
        );
        let r =
            run(json!({"operation":"numericSlot","a":[0.,0.],"b":[10.,0.],"width":4.})).unwrap();
        assert_eq!(r["rightA"], json!([0., -2.]));
        assert_eq!(r["leftB"], json!([10., 2.]));
    }
    #[test]
    fn preview_budget_is_bounded() {
        let r = run(json!({"operation":"sourceDisplayBudget","edges":2000,"faces":1024})).unwrap();
        assert_eq!(r["displaySegments"], json!(32));
        assert_eq!(r["divisions"], json!(8));
        assert!(run(json!({"operation":"sourceDisplayBudget","edges":65537,"faces":1})).is_err());
    }
}
