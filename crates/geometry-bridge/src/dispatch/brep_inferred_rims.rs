use crate::*;
use brep_core::planar_cap as cap;

pub(super) fn infer(v: &Value, op: &str) -> Result<Value> {
    let source: brep_core::Model = field(v, "source")?;
    let shell = field(v, "shell")?;
    let work = field(v, "max_work")?;
    if op == "infer_rim_planes" {
        let report = cap::infer_section_planes(&source, shell, work)?;
        let planes = report.planes.into_iter().map(|p| {
            json!({"normal":p.normal.map(|q|q.to_string()),"offset":p.offset.to_string(),"rings":p.rings})
        }).collect::<Vec<_>>();
        return Ok(json!({"model":source,"planes":planes,"workUsed":report.exact_work}));
    }
    let cells = field(v, "max_verification_cells")?;
    let (attached, bounds) = if op == "close_inferred_rims" {
        let r = cap::close_shell_with_inferred_section_caps(&source, shell, work, cells)?;
        let errors = vec![String::from("0"); r.cap_faces.len()];
        (r, errors)
    } else {
        let r = cap::close_shell_with_bounded_inferred_section_caps(
            &source,
            shell,
            cap::SectionAttachmentOptions {
                max_exact_work: work,
                max_verification_cells: cells,
                support_tolerance_mm: field(v, "support_tolerance_mm")?,
            },
        )?;
        (
            r.attachment,
            r.support_error_upper
                .into_iter()
                .map(|q| q.to_string())
                .collect(),
        )
    };
    Ok(
        json!({"model":attached.model,"closed":attached.model.shells[shell].closed,"capFaces":attached.cap_faces,"supportErrorUpper":bounds,"workUsed":attached.exact_work}),
    )
}

pub(super) fn inspect(v: &Value) -> Result<Value> {
    let source: brep_core::Model = field(v, "source")?;
    let options = cap::SectionAttachmentOptions {
        max_exact_work: field(v, "max_work")?,
        max_verification_cells: field(v, "max_verification_cells")?,
        support_tolerance_mm: field(v, "support_tolerance_mm")?,
    };
    let r = cap::inspect_inferred_section_solid(
        &source,
        options,
        super::closed_ss::closed_solid_policy(field(v, "tolerance_uv")?),
    )?;
    let evidence = r.admission.geometry();
    let orientations = evidence
        .orientations
        .iter()
        .map(|o| json!({"shell":o.shell,"expectedOutward":o.expected_outward,"outward":o.outward}))
        .collect::<Vec<_>>();
    let bounds = r
        .support_error_upper
        .into_iter()
        .map(|q| q.to_string())
        .collect::<Vec<_>>();
    Ok(
        json!({"model":r.admission.candidate(),"qualified":r.admission.solid().is_some(),"capFaces":r.cap_faces,"boundaryProven":evidence.boundary.proven,"joinsExact":evidence.boundary.agreement.all_joins_exact,"nestingConsistent":evidence.nesting.as_ref().and_then(|n|n.roles_consistent),"orientations":orientations,"supportErrorUpper":bounds}),
    )
}
