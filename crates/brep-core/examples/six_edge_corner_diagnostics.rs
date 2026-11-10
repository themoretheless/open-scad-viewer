use brep_core::{face_curvature as c, region_replacement as r};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).ok_or("output directory")?);
    std::fs::create_dir_all(&dir)?;
    let source = r::six_edge_corner_stock()?;
    let prescribed = std::env::args().nth(2).as_deref() == Some("radii");
    let corner = if prescribed {
        r::propose_six_edge_corner_radii(&source, [2.6, 2.8, 0.9])?
    } else {
        r::propose_six_edge_corner(&source, [0., 0.25, 0.5])?
    };
    let fast = std::env::args().nth(3).as_deref() == Some("coefficient");
    std::fs::write(dir.join("source.json"), value_codec::to_string(&source)?)?;
    std::fs::write(
        dir.join("body.json"),
        value_codec::to_string(
            &value_codec::json!({"model":corner.model,"accepted":false,"smoothEdges":corner.smooth_edges}),
        )?,
    )?;
    eprintln!("phase=exact-columnar-boundary");
    let proof = r::inspect_six_edge_graph(&source, &corner)?;
    eprintln!("phase=generic-closure");
    let closure = brep_core::watertight::inspect_with_exact(&corner.model, 1000000, 1000000)?;
    let mut accepted = closure.closure.status == brep_core::watertight::Closure::WithinTolerance;
    let mut fields = Vec::new();
    for face in 0..if fast { 0 } else { corner.model.faces.len() } {
        eprintln!("phase=curvature face={face}");
        let report = c::inspect(
            &corner.model,
            face,
            10.,
            1e-8,
            c::Limits {
                cells: 50000,
                domain_cells: 1000000,
                shape_cells: 1000000,
            },
            &[],
        )?;
        accepted &= report.decision == c::Decision::Within;
        fields.push(value_codec::json!({"face":face,"decision":format!("{:?}",report.decision),"upper":report.absolute_curvature_upper,"reason":report.reason,"cells":report.cells}));
        std::fs::write(
            dir.join("curvature.json"),
            value_codec::to_string_pretty(&fields)?,
        )?;
    }
    let mut radii = Vec::new();
    let s = corner.physical_scale;
    for xy in [
        [0., 8.],
        [0., -8.],
        [8., 0.],
        [-8., 0.],
        [8., -8.],
        [-8., 8.],
    ] {
        // Actual face footprint classification; no saved radius recipe is read.
        let mut found = None;
        for &face in &corner.top_faces {
            let surf = &corner.model.faces[face].surface;
            if xy[0] < surf.knots_u[surf.degree_u]
                || xy[0] > surf.knots_u[surf.control_points.len()]
                || xy[1] < surf.knots_v[surf.degree_v]
                || xy[1] > surf.knots_v[surf.control_points[0].len()]
            {
                continue;
            }
            let d = brep_core::face_domain::FaceDomain::new(&corner.model, face, 1e-8)?;
            let point = [[xy[0]; 2], [xy[1]; 2]];
            if d.classify(point, 10000)?.location == nurbs_core::trim_domain::Location::Inside {
                found = Some(face);
                break;
            }
        }
        let face = found.ok_or("No unique arm face")?;
        let shape = nurbs_core::surface_shape_operator::at(
            &corner.model.faces[face].surface,
            xy,
            [nurbs_core::curve_differential::Side::Automatic; 2],
        )?;
        let tensor = shape.tensor.ok_or("No arm curvature tensor")?;
        let mut trace = nurbs_core::interval_eval::Interval::point(0.);
        for k in 0..3 {
            trace = trace.add(nurbs_core::interval_eval::Interval::new(
                tensor[k][k][0],
                tensor[k][k][1],
            )?)?;
        }
        let absolute = if trace.hi < 0. {
            nurbs_core::interval_eval::Interval::new(-trace.hi, -trace.lo)?
        } else {
            trace
        };
        let radius = nurbs_core::interval_eval::Interval::point(1.).div(absolute)?;
        let point = corner.model.faces[face]
            .surface
            .evaluate(xy[0], xy[1])?
            .point;
        radii.push(value_codec::json!({"face":face,"uv":xy,"point":point,"principalRadiusIntervalMm":[radius.lo,radius.hi],"scope":"one nonzero principal radius at the middle of an extruded arm; not constant across the G2 profile"}));
    }
    std::fs::write(
        dir.join("radii.json"),
        value_codec::to_string_pretty(&radii)?,
    )?;
    let report = value_codec::json!({"acceptedNative":accepted,"coefficientCurvatureUpperPerMm":proof.curvature_upper_per_mm,"genericCurvatureSkipped":fast,"requestedMidRadii":corner.requested_mid_radii,"nominalRadiusIntervals":corner.mid_radius_intervals,"scope":"canonical six-edge corner; three unequal paired arm profiles", "faces":corner.model.faces.len(),"edges":corner.model.edges.len(),"vertices":corner.model.vertices.len(),"topFaces":corner.top_faces.len(),"smoothSeams":proof.smooth_seams,"exactTwoJetEqualities":proof.exact_jet_equalities,"columnarClassifiedFacePairs":proof.classified_face_pairs,"closure":format!("{:?}",closure.closure.status),"materialInsideSource":proof.material_inside_source,"heightBoundsMm":[proof.minimum_height_mm,proof.maximum_height_mm],"volumeExactMm3":proof.volume_exact,"volumeMm3":proof.volume_mm3,"physicalScale":s,"handles":corner.handles,"curvature":fields,"radii":radii});
    std::fs::write(
        dir.join("report.json"),
        value_codec::to_string_pretty(&report)?,
    )?;
    if !accepted {
        return Err("Six-edge body remains unqualified".into());
    }
    let (step, _, _) = cad_step::export_step_v5(&corner.model)?;
    std::fs::write(dir.join("candidate.step"), step)?;
    println!("{}", value_codec::to_string_pretty(&report)?);
    Ok(())
}
