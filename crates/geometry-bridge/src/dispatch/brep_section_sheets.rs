use crate::*;
use brep_core::planar_cap as cap;
use nurbs_core::curve::Curve;
pub(super) fn build(v: &Value, op: &str) -> Result<Value> {
    let rings: Vec<Vec<Curve>> = field(v, "rings")?;
    let rings = rings.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let normal = field(v, "plane_normal")?;
    let offset = field(v, "plane_offset_mm")?;
    let options = cap::SectionSheetOptions {
        tolerance: field(v, "tolerance_mm")?,
        max_exact_work: field(v, "max_work")?,
        max_verification_cells: field(v, "max_verification_cells")?,
    };
    let (sheet, errors) = if op == "bounded_section_brep_sheets" {
        let r = cap::build_section_sheets_with_support_tolerance(
            &rings,
            normal,
            offset,
            options,
            field(v, "support_tolerance_mm")?,
        )?;
        (
            r.sheet,
            r.support_error_upper
                .into_iter()
                .map(|q| q.to_string())
                .collect::<Vec<_>>(),
        )
    } else {
        let sheet = if op == "expanded_section_sheets" {
            cap::build_section_sheets_with_expanded_exact_support(&rings, normal, offset, options)?
        } else {
            cap::build_section_sheets(&rings, normal, offset, options)?
        };
        let errors = vec![String::from("0"); sheet.model.faces.len()];
        (sheet, errors)
    };
    let support_domains = sheet
        .model
        .faces
        .iter()
        .map(|face| {
            let s = &face.surface;
            [
                [
                    s.knots_u[s.degree_u],
                    s.knots_u[s.knots_u.len() - 1 - s.degree_u],
                ],
                [
                    s.knots_v[s.degree_v],
                    s.knots_v[s.knots_v.len() - 1 - s.degree_v],
                ],
            ]
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"model":sheet.model,"supportDomains":support_domains,"faceRings":sheet.face_rings,"edgeCount":sheet.model.edges.len(),"shellsClosed":sheet.model.shells.iter().map(|s|s.closed).collect::<Vec<_>>(),"supportErrorUpper":errors,"workUsed":sheet.exact_work,"verificationCells":sheet.verification_cells}),
    )
}
