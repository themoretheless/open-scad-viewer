//! Publish independent native premises without upgrading construction admission.
use super::*;
fn attach(result: &mut Value, name: &str, proof: Value, all_levels: bool) {
    result["report"][name] = proof.clone();
    if let Some(levels) = result["levels"].as_array_mut() {
        if all_levels {
            for level in levels {
                level[name] = proof.clone();
            }
        } else if let Some(last) = levels.last_mut() {
            last[name] = proof;
        }
    }
}
pub(super) fn profile(mut result: Value, request: &Value, options: &Value) -> Result<Value> {
    let mut source = request.clone();
    // Construction-only RMF limits must not leak into independent proof calls.
    if let Some(fields) = source.as_object_mut() {
        for key in [
            "rmf_transport_steps",
            "error_max_cells",
            "error_max_products",
        ] {
            fields.remove(key);
        }
    }
    source["order"] = json!(2);
    source["maxCells"] = json!(10000);
    source["maxExactWork"] = json!(1000000);
    attach(
        &mut result,
        "sourceFrameSmoothness",
        nurbs("surface_progressive_sweep_frame_smoothness", source.clone())?,
        true,
    );
    let authored = source["orientation"] == json!("authored");
    let guided = !source["orientation_guide"].is_null();
    if yes(&result["report"], "closedPath") {
        let operation = if authored {
            Some("surface_progressive_sweep_closed_frame_smoothness")
        } else if guided {
            Some("surface_progressive_sweep_closed_guided_frame_smoothness")
        } else if ["rmf", "fixed_normal", "corrected_frenet"]
            .contains(&source["orientation"].as_str().unwrap_or(""))
        {
            Some("surface_progressive_sweep_closed_path_frame_smoothness")
        } else {
            None
        };
        if let Some(operation) = operation {
            let proof = nurbs(operation, source.clone())?;
            let c1 = !authored && !guided && !yes(&proof, "closedSourceFrameSmoothnessCertified");
            attach(&mut result, "closedSourceFrameSmoothness", proof, true);
            if c1 {
                let mut c1 = source.clone();
                c1["order"] = json!(1);
                attach(
                    &mut result,
                    "closedSourceFrameSmoothnessC1",
                    nurbs(operation, c1)?,
                    true,
                );
            }
        }
    }
    if yes(&result["report"], "accepted")
        && result["patches"].as_array().is_some_and(|p| !p.is_empty())
    {
        let proof = nurbs(
            "sweep_retained_patch_regularity",
            json!({"patches":result["patches"],"maxCells":10000}),
        )?;
        attach(&mut result, "retainedPatchRegularity", proof, false);
        let mut retained = source.clone();
        retained["preview_sections"] = result["report"]["sections"].clone();
        retained["transverseScale"] = json!(1);
        let fallback = nurbs(
            "surface_progressive_sweep_decomposition_smoothness",
            retained,
        )?;
        let mut g2 = fallback["g2"].clone();
        g2["profilePatchRanges"] = fallback["profilePatchRanges"].clone();
        attach(&mut result, "retainedDecompositionSmoothness", g2, false);
        attach(
            &mut result,
            "retainedDecompositionG1Fallback",
            fallback,
            false,
        );
        if !guided
            && ["frenet", "fixed_normal"].contains(&source["orientation"].as_str().unwrap_or(""))
        {
            let op = if source["orientation"] == json!("frenet") {
                "sweep_frenet_frame_regularity"
            } else {
                "sweep_fixed_normal_frame_regularity"
            };
            attach(
                &mut result,
                "sourceFrameRegularity",
                nurbs(op, source.clone())?,
                false,
            );
        }
    }
    if authored {
        let proof = nurbs(
            "sweep_authored_frame_regularity",
            json!({"longitudinal":source["frame_axis"],"transverse":source["frame_normal"],"maxCells":value(options,"frameRegularityMaxCells",json!(10000))}),
        )?;
        attach(&mut result, "authoredFrameRegularity", proof, true);
    }
    Ok(result)
}
