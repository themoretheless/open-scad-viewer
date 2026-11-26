use super::*;
use crate::editing::{exact_curve_merge, exact_surface_merge};
use crate::surface::Axis;

pub(super) fn merge(v: &Value, op: &str) -> Result<Value> {
    if matches!(op, "join_affine_c1" | "join_affine_c2" | "join_projective_c2" | "join_stationary_c2" | "join_mixed_degree" | "compact_join_seam" | "compare_parametric_seam" | "compare_geometric_seam") {
        let left = field(v,"left")?;
        let right = field(v,"right")?;
        let budget = field(v,"max_work")?;
        return match op {
            "join_affine_c1" | "join_affine_c2" | "join_projective_c2" | "join_stationary_c2" => {
                let join = match op {
                    "join_affine_c1" => exact_curve_merge::merge_with_exact_c1_clock,
                    "join_affine_c2" => exact_curve_merge::merge_with_exact_c2_clock,
                    "join_projective_c2" => exact_curve_merge::merge_with_exact_projective_c2_clock,
                    _ => exact_curve_merge::merge_with_exact_stationary_c2_clock,
                };
                let r = join(&left,&right,budget)?;
                let curve = r.curve.ok_or_else(|| input(format!("Parameter matching refused: {:?}",r.refusal)))?;
                Ok(json!({"curve":curve,"domain":curve.domain(),"seam":left.domain()[1],"workUsed":r.work_used}))
            }
            "join_mixed_degree" => {
                let r = exact_curve_merge::merge_with_exact_degree_elevation(&left,&right,budget)?;
                let curve = r.curve.ok_or_else(|| input(format!("Mixed-degree merge refused: {:?}", r.refusal)))?;
                Ok(json!({"curve":curve,"degree":curve.degree,"domain":curve.domain(),"workUsed":r.work_used}))
            }
            "compact_join_seam" => {
                let r = exact_curve_merge::merge_and_simplify_exact_seam(&left,&right,budget)?;
                let curve = r.curve.ok_or_else(|| input(format!("Compact seam join refused: {:?}", r.join_refusal)))?;
                Ok(json!({"curve":curve,"knots":curve.knots,"seamMultiplicity":r.seam_multiplicity,
                    "removalRefusal":r.removal_refusal.map(|reason|format!("{reason:?}")),"workUsed":r.work_used}))
            }
            "compare_parametric_seam" => {
                let r = exact_curve_merge::compare_exact_curve_seam_jets(&left,&right,budget)?;
                if let Some(reason)=r.refusal { return Err(input(format!("Seam comparison refused: {reason:?}"))); }
                Ok(json!({"order":r.continuity_order,"residuals":r.residuals.map(|rows|rows.map(|row|
                    row.iter().map(ToString::to_string).collect::<Vec<_>>())),"workUsed":r.work_used}))
            }
            _ => {
                let r = exact_curve_merge::compare_exact_curve_geometric_seam(&left,&right,budget)?;
                if let Some(reason)=r.refusal { return Err(input(format!("Geometric comparison refused: {reason:?}"))); }
                Ok(json!({"order":r.continuity_order,"speedScale":r.speed_scale.map(|v|v.to_string()),
                    "accelerationScale":r.acceleration_scale.map(|v|v.to_string()),"workUsed":r.work_used}))
            }
        };
    }
    if matches!(op, "store_periodic_curve" | "rotate_periodic_knot" | "insert_periodic_seam" | "open_periodic_curve"
        | "trim_periodic_curve" | "trim_periodic_turns" | "partition_periodic_curve") {
        let source = field(v,"source")?;
        let budget = field(v,"max_work")?;
        let (curve,used,count) = if op == "partition_periodic_curve" {
            let cuts: Vec<f64> = field(v,"cuts")?;
            let index: usize = field(v,"part")?;
            let r = exact_curve_merge::partition_exact_periodic_curve(&source,field(v,"parameter")?,&cuts,budget)?;
            let mut pieces = r.pieces.ok_or_else(||input(format!("Periodic partition refused at {:?}: {:?}",r.refused_piece,r.refusal)))?;
            let count = pieces.len();
            if index >= count { return Err(input("Selected interval is outside the partition")); }
            (pieces.swap_remove(index),r.work_used,count)
        } else {
            let r = match op {
                "store_periodic_curve" => exact_curve_merge::store_exact_periodic_c2_curve(&source,budget)?,
                "rotate_periodic_knot" => exact_curve_merge::rotate_periodic_curve_at_exact_knot(&source,field(v,"parameter")?,budget)?,
                "insert_periodic_seam" => exact_curve_merge::insert_and_rotate_periodic_curve_seam(&source,field(v,"parameter")?,budget)?,
                "open_periodic_curve" => exact_curve_merge::open_exact_periodic_curve(&source,budget)?,
                "trim_periodic_curve" => exact_curve_merge::restrict_exact_periodic_curve(&source,field(v,"domain")?,budget)?,
                _ => exact_curve_merge::restrict_exact_periodic_curve_over_turns(&source,field(v,"domain")?,budget)?,
            };
            (r.curve.ok_or_else(||input(format!("Periodic curve edit refused: {:?}",r.refusal)))?,r.work_used,1)
        };
        return Ok(json!({"curve":curve,"domain":curve.domain(),"periodic":curve.periodic,"knots":curve.knots,
            "controlCount":curve.control_points.len(),"pieceCount":count,"workUsed":used}));
    }
    if matches!(op, "join_projective_chain" | "compact_projective_chain" | "close_projective_chain") {
        let curves: Vec<crate::curve::Curve> = field(v, "curves")?;
        if curves.is_empty() || curves.len() > 32 { return Err(input("Expected 1 to 32 curves")); }
        let sources = curves.iter().collect::<Vec<_>>();
        let budget = field(v,"max_work")?;
        let (chain, seams, closure) = match op {
            "compact_projective_chain" => {
                let r = exact_curve_merge::merge_and_compact_exact_projective_c2_chain(&sources,budget)?;
                let seams = r.seams.into_iter().map(|s| json!({"parameter":s.parameter,"removed":s.removed,
                    "remaining":s.remaining_multiplicity,"refusal":s.refusal.map(|reason|format!("{reason:?}"))})).collect::<Vec<_>>();
                (r.chain, Some(seams), None)
            }
            "close_projective_chain" => {
                let r = exact_curve_merge::merge_closed_exact_projective_c2_chain(&sources,budget)?;
                (r.chain, None, r.closure.and_then(|c| c.continuity_order))
            }
            _ => (exact_curve_merge::merge_chain_with_exact_projective_c2_clock(&sources,budget)?, None, None),
        };
        let curve = chain.curve.ok_or_else(|| input(format!("C2 chain refused after {} joins: {:?}",chain.joins_verified,chain.refusal)))?;
        return Ok(json!({"curve":curve,"domain":curve.domain(),"knots":curve.knots,"joinsVerified":chain.joins_verified,
            "seams":seams,"closureOrder":closure,"workUsed":chain.work_used}));
    }
    if matches!(op, "merge_weighted_curves" | "merge_weighted_chain") {
        let curves: Vec<crate::curve::Curve> = field(v, "curves")?;
        let budget = field(v, "max_work")?;
        let report = if op == "merge_weighted_curves" {
            let [left, right] = curves.as_slice() else { return Err(input("Expected two curves")); };
            exact_curve_merge::merge_with_weight_scaling(left, right, budget)?
        } else {
            if curves.is_empty() || curves.len() > 32 { return Err(input("Expected 1 to 32 curves")); }
            let sources = curves.iter().collect::<Vec<_>>();
            exact_curve_merge::merge_chain_with_weight_scaling(&sources, budget)?
        };
        let curve = report.curve.ok_or_else(|| input(format!("Weighted curve merge refused: {:?}", report.refusal)))?;
        return Ok(json!({"curve":curve,"domain":curve.domain(),"weights":curve.weights,"workUsed":report.work_used}));
    }
    if op == "merge_curve_exact" {
        let left: crate::curve::Curve = field(v, "left")?;
        let right = field(v, "right")?;
        left.validate()?;
        let seam = left.domain()[1];
        let report = exact_curve_merge::merge(&left, &right)?;
        let curve = report.curve.ok_or_else(|| input(format!("Exact curve merge refused: {:?}", report.refusal)))?;
        return Ok(json!({"curve":curve,"seam":seam,"domain":curve.domain()}));
    }
    let left: crate::surface::Surface = field(v,"left")?;
    let right = field(v,"right")?;
    let direction: String = field(v,"direction")?;
    let axis = match direction.as_str() {
        "u" => Axis::U, "v" => Axis::V, _ => return Err(input("Unknown surface merge direction"))
    };
    let report = exact_surface_merge::merge(&left,&right,axis)?;
    let surface = report.surface.ok_or_else(|| input(format!("Exact surface merge refused: {:?}", report.refusal)))?;
    let seam = match axis { Axis::U => left.knots_u[left.control_points.len()], Axis::V => left.knots_v[left.control_points[0].len()] };
    Ok(json!({"surface":surface,"seam":seam,"direction":direction}))
}

pub(super) fn compress(v: &Value, op: &str) -> Result<Value> {
    use crate::editing::exact_surface_knot_removal::{
        TensorCompression, compress_tensor_surface, compress_tensor_surface_batch,
    };
    fn summary(patch: &TensorCompression) -> Value {
        json!({"removedKnots":patch.removed_knots,"reducedDegrees":patch.reduced_degrees,
            "completed":patch.completed,"controlNet":[patch.surface.control_points.len(),
            patch.surface.control_points[0].len()],"workUsed":patch.work_used})
    }
    let budget = field(v,"max_work")?;
    if op == "compress_tensor_surface" {
        let r = compress_tensor_surface(&field(v,"source")?,budget)?;
        return Ok(json!({"surface":r.surface,"patches":[summary(&r)],"workUsed":r.work_used}));
    }
    let sources: Vec<crate::surface::Surface> = field(v,"surfaces")?;
    let index: usize = field(v,"patch_index")?;
    if sources.len()>32 || index>=sources.len() { return Err(input("Invalid compression batch or patch index")); }
    let r=compress_tensor_surface_batch(&sources,budget)?;
    let patches=r.patches.ok_or_else(||input("Exact batch compression exhausted its shared work budget"))?;
    Ok(json!({"surface":patches[index].surface,"patches":patches.iter().map(summary).collect::<Vec<_>>(),"selectedPatch":index,"workUsed":r.work_used}))
}

pub(super) fn grid(v: &Value, op: &str) -> Result<Value> {
    let sources: Vec<crate::surface::Surface> = field(v, "surfaces")?;
    if sources.is_empty() || sources.len() > 32 {
        return Err(input("Patch layout requires 1 to 32 source surfaces"));
    }
    let budget = field(v, "max_work")?;
    match op {
        "merge_patch_strip" => {
            let axis = match field::<String>(v, "direction")?.as_str() {
                "u" => Axis::U, "v" => Axis::V, _ => return Err(input("Unknown strip axis")),
            };
            let r = exact_surface_merge::merge_surface_strip(&sources, axis, budget)?;
            let surface = r.surface.ok_or_else(|| input(format!("Exact strip refused: {:?}", r.refusal)))?;
            Ok(json!({"surface":surface,"removedKnots":r.removed_knots,"workUsed":r.work_used}))
        }
        "merge_patch_grid" => {
            let r = exact_surface_merge::merge_surface_grid(&sources, field(v,"columns")?, budget)?;
            let surface = r.surface.ok_or_else(|| input(format!("Exact grid refused: {:?}", r.refusal)))?;
            Ok(json!({"surface":surface,"alignmentInsertions":r.inserted_knots_for_alignment,"workUsed":r.work_used}))
        }
        _ => {
            let r = exact_surface_merge::infer_surface_grid_layout(&sources,budget)?;
            let layout = r.layout.ok_or_else(|| input(format!("Patch layout refused: {:?}",r.refusal)))?;
            Ok(json!({"order":layout.order,"uBreaks":layout.u_breaks,"vBreaks":layout.v_breaks,"workUsed":r.work_used}))
        }
    }
}

pub(super) fn split(v: &Value, op: &str) -> Result<Value> {
    use crate::editing::exact_surface_knot_insertion::{split_tensor_surface, tile_tensor_surface};
    let source = field(v, "source")?;
    let budget = field(v, "max_work")?;
    let (surface, columns, rows, index, used) = if op == "split_tensor_surface" {
        let index: usize = field(v, "part")?;
        if index > 1 { return Err(input("A tensor split has two parts")); }
        let axis = match field::<String>(v, "direction")?.as_str() {
            "u" => Axis::U, "v" => Axis::V, _ => return Err(input("Unknown split axis")),
        };
        let (columns, rows) = match axis { Axis::U => (2,1), Axis::V => (1,2) };
        let r = split_tensor_surface(&source, axis, field(v,"parameter")?, budget)?;
        let parts = r.parts.ok_or_else(|| input(format!("Exact tensor split refused: {:?}",r.refusal)))?;
        let [left, right] = parts;
        (if index==0 {left} else {right}, columns, rows, index, r.work_used)
    } else {
        let cuts_u: Vec<f64> = field(v,"cuts_u")?;
        let cuts_v: Vec<f64> = field(v,"cuts_v")?;
        let index = field(v,"patch_index")?;
        let r = tile_tensor_surface(&source,&cuts_u,&cuts_v,budget)?;
        let mut parts = r.patches.ok_or_else(|| input(format!("Exact tensor tiling refused: {:?}",r.refusal)))?;
        if index >= parts.len() { return Err(input("Selected patch is outside the UV grid")); }
        (parts.swap_remove(index),r.columns,r.rows,index,r.work_used)
    };
    let domain = [[surface.knots_u[surface.degree_u],surface.knots_u[surface.control_points.len()]],
        [surface.knots_v[surface.degree_v],surface.knots_v[surface.control_points[0].len()]]];
    Ok(json!({"surface":surface,"domain":domain,"columns":columns,"rows":rows,"selectedPatch":index,"workUsed":used}))
}
