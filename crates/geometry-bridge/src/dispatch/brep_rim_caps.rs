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
        "discover_expanded_rims" => {
            let r = cap::discover_and_attach_section_caps_with_expanded_exact_support(
                &source,
                shell,
                cap::SectionPlane { normal, offset },
                work,
                cells,
            )?;
            (r.attachment, r.rings, r.discovery_work, None)
        }
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
            if op == "attach_expanded_rims" {
                (
                    cap::attach_section_caps_with_expanded_exact_support(
                        &source, shell, &refs, normal, offset, work, cells,
                    )?,
                    rings,
                    0,
                    None,
                )
            } else if op == "attach_section_rims" {
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

pub(super) fn close_planes(v: &Value, mode: &str) -> Result<Value> {
    let source: brep_core::Model = field(v, "source")?;
    let shell = field(v, "shell")?;
    let work = field(v, "max_work")?;
    let cells = field(v, "max_verification_cells")?;
    let (attachment, errors) = if mode == "bounded" {
        let planes = bounded_planes(v)?;
        let r = cap::close_shell_with_bounded_section_caps(&source, shell, &planes, work, cells)?;
        (
            r.attachment,
            r.support_error_upper
                .into_iter()
                .map(|q| q.to_string())
                .collect(),
        )
    } else {
        let planes = exact_planes(v)?;
        let r = if mode == "expanded" {
            cap::close_shell_with_expanded_exact_section_caps(&source, shell, &planes, work, cells)?
        } else {
            cap::close_shell_with_section_caps(&source, shell, &planes, work, cells)?
        };
        let errors = vec![String::from("0"); r.cap_faces.len()];
        (r, errors)
    };
    Ok(
        json!({"model":attachment.model,"capFaces":attachment.cap_faces,"closed":attachment.model.shells[shell].closed,"supportErrorUpper":errors,"sourceEdges":source.edges.len(),"resultEdges":attachment.model.edges.len(),"workUsed":attachment.exact_work,"verificationCells":attachment.verification_cells}),
    )
}

fn exact_planes(v: &Value) -> Result<Vec<cap::SectionPlane>> {
    let rows: Vec<[f64; 4]> = field(v, "planes")?;
    let planes = rows
        .into_iter()
        .map(|[x, y, z, offset]| cap::SectionPlane {
            normal: [x, y, z],
            offset,
        })
        .collect::<Vec<_>>();
    Ok(planes)
}

fn bounded_planes(v: &Value) -> Result<Vec<cap::BoundedSectionPlane>> {
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
    Ok(planes)
}

pub(super) fn solid(v: &Value, mode: &str) -> Result<Value> {
    let source: brep_core::Model = field(v, "source")?;
    let options = cap::SectionSolidOptions {
        max_exact_work: field(v, "max_work")?,
        max_verification_cells: field(v, "max_verification_cells")?,
        admission: super::closed_ss::closed_solid_policy(field(v, "tolerance_uv")?),
        volume_tolerance_mm3: field(v, "volume_tolerance_mm3")?,
        max_volume_cells: field(v, "max_volume_cells")?,
    };
    let mut reversed = false;
    let (section, errors) = if mode == "outward_planes" || mode == "outward_inferred" {
        let r = if mode == "outward_planes" {
            cap::build_outward_section_solid(&source, &exact_planes(v)?, options)?
        } else {
            cap::build_outward_inferred_section_solid(&source, options)?
        };
        reversed = r.orientation_reversed;
        let errors = vec![String::from("0"); r.section.cap_faces.len()];
        (r.section, errors)
    } else if mode == "expanded_planes" {
        let r = cap::build_expanded_section_solid(&source, &exact_planes(v)?, options)?;
        let errors = vec![String::from("0"); r.cap_faces.len()];
        (r, errors)
    } else if mode == "expanded" {
        let r = cap::build_expanded_inferred_section_solid(&source, options)?;
        let bounds = vec![String::from("0"); r.cap_faces.len()];
        (r, bounds)
    } else if mode == "inferred" {
        let r =
            cap::build_inferred_section_solid(&source, field(v, "support_tolerance_mm")?, options)?;
        (
            r.section,
            r.support_error_upper
                .into_iter()
                .map(|q| q.to_string())
                .collect(),
        )
    } else if mode == "bounded" {
        let r = cap::build_bounded_section_solid(&source, &bounded_planes(v)?, options)?;
        (
            r.section,
            r.support_error_upper
                .into_iter()
                .map(|q| q.to_string())
                .collect(),
        )
    } else {
        let r = cap::build_section_solid(&source, &exact_planes(v)?, options)?;
        let errors = vec![String::from("0"); r.cap_faces.len()];
        (r, errors)
    };
    let model = section.solid.model();
    let volume = section.volume.volume_mm3;
    Ok(
        json!({"model":model,"orientationReversed":reversed,"capFaces":section.cap_faces,"bodyCount":model.bodies.len(),"boundaryProven":section.solid.geometry().proven,"volumeMm3":[volume.lower,volume.upper],"supportErrorUpper":errors,"volumeCells":section.volume.cells}),
    )
}
