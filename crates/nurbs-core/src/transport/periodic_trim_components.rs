use super::*;
use crate::editing::exact_periodic_surface::*;

pub(super) const OPS: &[&str] = &[
    "lift_exact_periodic_surface_region_components",
    "build_exact_periodic_surface_trim_components",
    "build_exact_periodic_surface_trim_patches",
    "build_exact_periodic_surface_trim_edges",
];

fn rational(t: num_rational::BigRational) -> Value {
    json!({"numerator":t.numer().to_string(),"denominator":t.denom().to_string()})
}
fn lifted(c: LiftedPeriodicContour) -> Value {
    json!({"contour":c.contour,"translation":c.translation,"vertices":c.vertices.into_iter().map(|v|v.map(rational)).collect::<Vec<_>>()})
}
fn trim(c: PeriodicTrimComponent) -> Value {
    json!({"contours":c.contours,"loops":c.loops})
}
fn patch(p: PeriodicTrimPatch) -> Value {
    json!({"surface":p.surface,"trim":trim(p.trim),"uvTranslation":p.uv_translation.map(rational)})
}
fn result<R: std::fmt::Debug>(
    components: Option<Vec<Value>>,
    work: u64,
    refusal: Option<R>,
) -> Value {
    json!({"components":components,"workUsed":work,"refusal":refusal.map(|r|format!("{r:?}"))})
}
pub(super) fn dispatch(request: &Value, op: &str) -> Result<Value> {
    let source = field(request, "surface")?;
    let paths: Vec<Vec<[f64; 2]>> = field(request, "contours")?;
    let contours = paths.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let work = field(request, "max_work")?;
    Ok(match op {
        "lift_exact_periodic_surface_region_components" => {
            let r = lift_exact_periodic_surface_region_components(&source, &contours, work)?;
            result(r.components.map(|c|c.into_iter().map(|p|json!({"outer":lifted(p.outer),"holes":p.holes.into_iter().map(lifted).collect::<Vec<_>>()})).collect()),r.work_used,r.refusal)
        }
        "build_exact_periodic_surface_trim_components" => {
            let r = build_exact_periodic_surface_trim_components(&source, &contours, work)?;
            result(
                r.components.map(|c| c.into_iter().map(trim).collect()),
                r.work_used,
                r.refusal,
            )
        }
        "build_exact_periodic_surface_trim_patches" => {
            let r = build_exact_periodic_surface_trim_patches(&source, &contours, work)?;
            result(
                r.patches.map(|c| c.into_iter().map(patch).collect()),
                r.work_used,
                r.refusal,
            )
        }
        "build_exact_periodic_surface_trim_edges" => {
            let r = build_exact_periodic_surface_trim_edges(&source, &contours, work)?;
            let mut out = result(
                r.patches.map(|c| {
                    c.into_iter()
                        .map(|p| {
                            let loops = p
                                .loops
                                .into_iter()
                                .map(|l| {
                                    l.into_iter()
                                        .map(|e| json!({"curve":e.curve,"pcurve":e.pcurve}))
                                        .collect::<Vec<_>>()
                                })
                                .collect::<Vec<_>>();
                            json!({"patch":patch(p.patch),"loops":loops})
                        })
                        .collect()
                }),
                r.work_used,
                r.refusal,
            );
            out["refusedCurve"] = json!(r.refused_curve);
            out
        }
        _ => return Err(crate::input("Unknown periodic trim component request")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_translation_and_seam_edge_counts_survive_transport() {
        let profile = crate::curve::Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        };
        let source = crate::surface::extrude(&profile, [0., 0., 2.]).unwrap();
        let request = json!({"surface":source,"max_work":1_000_000,
            "contours":[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],
                        [[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]]});
        let mut reports = Vec::new();
        for op in OPS {
            let r = dispatch(&request, op).unwrap();
            assert!(r["refusal"].is_null(), "{r}");
            assert_eq!(r["components"].as_array().unwrap().len(), 1);
            reports.push(r);
        }
        assert_eq!(
            reports[0]["components"][0]["holes"][0]["translation"],
            json!([1, 0])
        );
        assert_eq!(reports[1]["components"][0]["contours"], json!([0, 1]));
        assert_eq!(
            reports[2]["components"][0],
            reports[3]["components"][0]["patch"]
        );
        let loops = reports[3]["components"][0]["loops"].as_array().unwrap();
        assert_eq!(
            loops
                .iter()
                .map(|v| v.as_array().unwrap().len())
                .collect::<Vec<_>>(),
            vec![6, 4]
        );
    }
}
