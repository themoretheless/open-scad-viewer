use crate::*;
use brep_core::planar_cap as cap;
pub(super) fn attach(v: &Value, op: &str) -> Result<Value> {
    let source: brep_core::Model = field(v, "source")?;
    let shell = field(v, "shell")?;
    let normal = field(v, "plane_normal")?;
    let offset = field(v, "plane_offset_mm")?;
    let work = field(v, "max_work")?;
    let cells = field(v, "max_verification_cells")?;
    let bounded = || -> Result<cap::SectionAttachmentOptions> {
        Ok(cap::SectionAttachmentOptions {
            max_exact_work: work,
            max_verification_cells: cells,
            support_tolerance_mm: field(v, "support_tolerance_mm")?,
        })
    };
    let (attached, rings, discovery, errors) = match op {
        "discover_section_rims" => {
            let r =
                cap::discover_and_attach_section_caps(&source, shell, normal, offset, work, cells)?;
            (r.attachment, r.rings, r.discovery_work, None)
        }
        "discover_bounded_rims" => {
            let r = cap::discover_and_attach_section_caps_with_support_tolerance(
                &source,
                shell,
                cap::SectionPlane { normal, offset },
                field(v, "selection_tolerance_mm")?,
                bounded()?,
            )?;
            (
                r.bounded.attachment,
                r.rings,
                r.discovery_work,
                Some(r.bounded.support_error_upper),
            )
        }
        _ => {
            let rings: Vec<Vec<[usize; 2]>> = field(v, "rim_addresses")?;
            let refs = rings.iter().map(Vec::as_slice).collect::<Vec<_>>();
            if op == "attach_section_rims" {
                (
                    cap::attach_section_caps(&source, shell, &refs, normal, offset, work, cells)?,
                    rings,
                    0,
                    None,
                )
            } else {
                let r = cap::attach_section_caps_with_support_tolerance(
                    &source,
                    shell,
                    &refs,
                    normal,
                    offset,
                    bounded()?,
                )?;
                (r.attachment, rings, 0, Some(r.support_error_upper))
            }
        }
    };
    let bounds = errors
        .map(|v| v.into_iter().map(|q| q.to_string()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec![String::from("0"); attached.cap_faces.len()]);
    Ok(
        json!({"model":attached.model,"capFaces":attached.cap_faces,"closed":attached.model.shells[shell].closed,"sourceEdges":source.edges.len(),"resultEdges":attached.model.edges.len(),"rimAddresses":rings,"supportErrorUpper":bounds,"discoveryWork":discovery,"workUsed":discovery+attached.exact_work,"verificationCells":attached.verification_cells}),
    )
}

pub(super) fn close_planes(v: &Value, bounded: bool) -> Result<Value> {
    let source: brep_core::Model = field(v, "source")?;
    let shell = field(v, "shell")?;
    let work = field(v, "max_work")?;
    let cells = field(v, "max_verification_cells")?;
    let (attachment, errors) = if bounded {
        let rows: Vec<[f64; 6]> = field(v, "planes")?;
        let planes = rows
            .into_iter()
            .map(
                |[x, y, z, offset, selection, support]| cap::BoundedSectionPlane {
                    plane: cap::SectionPlane {
                        normal: [x, y, z],
                        offset,
                    },
                    selection_tolerance_mm: selection,
                    support_tolerance_mm: support,
                },
            )
            .collect::<Vec<_>>();
        let r = cap::close_shell_with_bounded_section_caps(&source, shell, &planes, work, cells)?;
        (
            r.attachment,
            r.support_error_upper
                .into_iter()
                .map(|q| q.to_string())
                .collect(),
        )
    } else {
        let rows: Vec<[f64; 4]> = field(v, "planes")?;
        let planes = rows
            .into_iter()
            .map(|[x, y, z, offset]| cap::SectionPlane {
                normal: [x, y, z],
                offset,
            })
            .collect::<Vec<_>>();
        let r = cap::close_shell_with_section_caps(&source, shell, &planes, work, cells)?;
        let errors = vec![String::from("0"); r.cap_faces.len()];
        (r, errors)
    };
    Ok(
        json!({"model":attachment.model,"capFaces":attachment.cap_faces,"closed":attachment.model.shells[shell].closed,"supportErrorUpper":errors,"sourceEdges":source.edges.len(),"resultEdges":attachment.model.edges.len(),"workUsed":attachment.exact_work,"verificationCells":attachment.verification_cells}),
    )
}
