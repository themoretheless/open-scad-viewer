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
