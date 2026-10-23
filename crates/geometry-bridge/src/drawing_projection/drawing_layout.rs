//! Drawing layout: preserves the original arithmetic and proof budgets.
use super::*;

pub fn drawing(v: &Value) -> Result<Value> {
    let bodies: Vec<Value> = field(v, "bodies")?;
    let view: String = field(v, "view")?;
    let hidden: String = field(v, "hidden")?;
    if bodies.is_empty()
        || bodies.len() > 200
        || !matches!(view.as_str(), "top" | "front" | "right")
        || !matches!(hidden.as_str(), "show" | "hide" | "dash")
    {
        return Err(input("Invalid drawing projection options"));
    }
    let occlusion = v.get("occlusion").and_then(Value::as_str).unwrap_or("mesh");
    if !matches!(occlusion, "mesh" | "brep") {
        return Err(input("Invalid drawing occlusion mode"));
    }
    let tolerance = if occlusion == "brep" { 1e-5 } else { 0.02 };
    let mut faces = Vec::new();
    let mut triangles = Vec::new();
    let mut curves = Vec::new();
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut approximate = false;
    let dimensions = linked_dimensions(v, &bodies, &view)?;
    let mut references = Vec::new();
    for body in &bodies {
        let mesh: Mesh = field(&body, "mesh")?;
        mesh.validate()?;
        let points: Vec<_> = mesh
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| project(p, &view))
            .collect();
        for p in &points {
            for k in 0..2 {
                bounds[k] = bounds[k].min(p[k]);
                bounds[k + 2] = bounds[k + 2].max(p[k]);
            }
        }
        triangles.extend(
            mesh.indices
                .as_chunks::<3>()
                .0
                .iter()
                .map(|i| i.map(|i| points[i])),
        );
        if triangles.len() > 100000 {
            return Err(input("Drawing exceeds 100000 triangles"));
        }
        if body.get("brep").is_some_and(|b| !b.is_null()) {
            let model: brep_core::Model = field(&body, "brep")?;
            model.validate()?;
            references.push(json!({"bodyId":body["id"],"topologySignature":topology_signature(&model),"vertices":model.vertices.iter().map(|v|project(&v.point,&view)).collect::<Vec<_>>(),"vertexIds":model.1.vertices.clone()}));
            if occlusion == "brep" {
                faces.extend(retained_faces(&model, &view)?);
                if faces.len() > 20000 {
                    return Err(input("Drawing exceeds 20000 retained faces"));
                }
            }
            for edge in &model.edges {
                for segment in edge.curve.decompose()? {
                    let mut c = segment.definition().clone();
                    for p in &mut c.control_points {
                        *p = project(p, &view).to_vec();
                    }
                    if curve_path(&c).is_ok() {
                        curves.push(c);
                    } else {
                        approximate = true;
                        let report = nurbs_core::tessellation::curve_tessellation::tessellate(
                            &c, 0.01, 8192,
                        )?;
                        if !report.within_tolerance {
                            return Err(input("Drawing curve projection tolerance is unresolved"));
                        }
                        for s in report.segments {
                            curves.push(Curve::from_polyline(s.points.to_vec())?);
                        }
                    }
                }
            }
        } else {
            if occlusion == "brep" {
                return Err(input(
                    "B-rep surface occlusion requires retained geometry on every body",
                ));
            }
            approximate = true;
            let topology =
                mesh_topology::planar::topology(mesh.view()).map_err(|e| input(e.message))?;
            for edge in topology.edges {
                curves.push(Curve::from_polyline(vec![
                    points[edge.a].to_vec(),
                    points[edge.b].to_vec(),
                ])?);
            }
        }
    }
    if occlusion == "brep" {
        bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for c in &curves {
            for k in 0..2 {
                let mut parameters = c.domain().to_vec();
                parameters.extend(nurbs_core::curve_extrema::axis_extrema(c, k)?);
                for t in parameters {
                    let value = c.evaluate(t)?.point[k];
                    bounds[k] = bounds[k].min(value);
                    bounds[k + 2] = bounds[k + 2].max(value);
                }
            }
        }
    }
    if curves.len() > 20000 {
        return Err(input("Drawing exceeds 20000 edge spans"));
    }
    let mut paths = Vec::new();
    let mut work = 0;
    for c in curves {
        let source = c.clone();
        let mut pieces = Vec::new();
        if hidden == "show" {
            pieces.push((c, false));
        } else {
            clip(
                c,
                if occlusion == "mesh" { &triangles } else { &[] },
                &faces,
                tolerance,
                &mut work,
                &mut pieces,
                0,
            )?;
        }
        // Subdivision proves visibility; merge consecutive equal states back
        // into source trims instead of exporting thousands of tiny SVG paths.
        let mut ranges: Vec<(f64, f64, bool)> = Vec::new();
        for (piece, hidden) in pieces {
            let [a, b] = piece.domain();
            if let Some(last) = ranges.last_mut()
                && last.2 == hidden
                && last.1 == a
            {
                last.1 = b;
            } else {
                ranges.push((a, b, hidden));
            }
        }
        for (a, b, is_hidden) in ranges {
            let c = if [a, b] == source.domain() {
                source.clone()
            } else {
                source.trim(a, b)?
            };
            if hidden == "hide" && is_hidden {
                continue;
            }
            paths.push(json!({"d":curve_path(&c)?,"hidden":is_hidden}));
            if paths.len() > 20000 {
                return Err(input("Drawing exceeds 20000 path fragments"));
            }
        }
    }
    Ok(
        json!({"paths":paths,"bounds":bounds,"approximate":approximate,"occlusionToleranceMm":tolerance,"occlusionGeometry":occlusion,"dimensions":dimensions,"references":references}),
    )
}
