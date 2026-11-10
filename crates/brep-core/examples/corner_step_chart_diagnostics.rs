//! Explicit exchange-only affine plane reparameterisation; 3D geometry unchanged.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).ok_or("body directory")?);
    let value: value_codec::Value =
        value_codec::from_str(&std::fs::read_to_string(dir.join("body.json"))?)?;
    let original: brep_core::Model = value_codec::from_value(value["model"].clone())?;
    let mut model = original.clone();
    let mut planes = Vec::new();
    for f in 0..model.faces.len() {
        let s = &mut model.faces[f].surface;
        if s.degree_u != 1
            || s.degree_v != 1
            || s.control_points.len() != 2
            || s.control_points.iter().any(|r| r.len() != 2)
            || !s.weights.iter().flatten().all(|w| *w == 1.)
        {
            continue;
        }
        if !(0..3).all(|k| {
            s.control_points[1][1][k] + s.control_points[0][0][k]
                == s.control_points[1][0][k] + s.control_points[0][1][k]
        }) {
            continue;
        }
        let d = [[s.knots_u[1], s.knots_u[2]], [s.knots_v[1], s.knots_v[2]]];
        if d == [[0., 1.], [0., 1.]] {
            continue;
        }
        assert!(d.iter().all(|x| x[0] < x[1]));
        for k in &mut s.knots_u {
            *k = (*k - d[0][0]) / (d[0][1] - d[0][0]);
        }
        for k in &mut s.knots_v {
            *k = (*k - d[1][0]) / (d[1][1] - d[1][0]);
        }
        let wires = std::iter::once(model.faces[f].outer)
            .chain(model.faces[f].holes.iter().copied())
            .collect::<Vec<_>>();
        for w in wires {
            for c in &mut model.loops[w].coedges {
                assert!(
                    c.trim().is_none(),
                    "Explicit source trims require separate clock transport"
                );
                for p in &mut c.pcurve.control_points {
                    for k in 0..2 {
                        p[k] = (p[k] - d[k][0]) / (d[k][1] - d[k][0]);
                    }
                }
            }
        }
        planes.push(f);
    }
    assert_eq!(model.vertices, original.vertices);
    assert_eq!(model.edges, original.edges);
    for (a, b) in model.faces.iter().zip(&original.faces) {
        assert_eq!(a.surface.control_points, b.surface.control_points);
        assert_eq!(a.surface.weights, b.surface.weights);
    }
    model.validate()?;
    let closure = brep_core::watertight::inspect_with_exact(&model, 1_000_000, 1_000_000)?;
    let trims =
        brep_core::face_domain::audit_trim_regions(&model, 1e-8, 10_000, 100_000, 1_000_000)?;
    let accepted = closure.closure.status == brep_core::watertight::Closure::WithinTolerance
        && trims.all_valid;
    std::fs::write(
        dir.join("exchange-chart-report.json"),
        value_codec::to_string_pretty(
            &value_codec::json!({"admittedForExchange":accepted,"planes":planes,"unchanged3DCurves":true,"unchangedSurfaceControlNets":true,"closure":format!("{:?}",closure.closure.status),"trimsValid":trims.all_valid,"scope":"Exchange copy only: unit affine plane UV clocks, 3D geometry and topology unchanged. Original native exact global receipt is not a claim of exact lifts on this rounded exchange clock."}),
        )?,
    )?;
    assert!(accepted, "Exchange copy failed fresh boundary/trim checks");
    let (text, _, _) = cad_step::export_step_v5(&model)?;
    std::fs::write(dir.join("candidate-unit-planar-uv.step"), text)?;
    Ok(())
}
