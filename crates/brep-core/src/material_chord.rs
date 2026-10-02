//! Material chords between certified transverse roots on original trimmed faces.
//! The authored finite line must start outside and have exactly two separated
//! interior roots, with complete boundary coverage. Normal alignment and global
//! minimum wall thickness are separate requirements, not inferred from a chord.
use crate::{Model, Result, material_segment, ray_parity, volume_validity};
use nurbs_core::surface_distance::{enclosure_distance, rectangle_bounds};

pub struct Report {
    pub proven: bool,
    pub reason: &'static str,
    pub validity: volume_validity::Report,
    pub seed: Option<ray_parity::PointReport>,
    pub boundary: material_segment::SegmentReport,
    pub point_enclosures: Option<[Vec<[f64; 2]>; 2]>,
    pub length_interval_mm: Option<[f64; 2]>,
}
pub fn inspect(
    model: &Model,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    limits: material_segment::Limits,
) -> Result<Report> {
    material_segment::valid_line(
        origin,
        direction,
        tolerance_uv,
        limits.point_cells,
        limits.point_domain_cells,
    )?;
    let boundary = material_segment::inspect_boundary(
        model,
        origin,
        direction,
        tolerance_uv,
        limits.segment_cells,
        limits.segment_domain_cells,
    )?;
    let validity = volume_validity::inspect(model, tolerance_uv, limits.volume)?;
    let mut out = Report {
        proven: false,
        reason: "volume-unproven",
        validity,
        seed: None,
        boundary,
        point_enclosures: None,
        length_interval_mm: None,
    };
    if !out.validity.proven {
        return Ok(out);
    }
    let seed = ray_parity::classify_point(
        model,
        origin,
        &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
        tolerance_uv,
        limits.point_cells,
        limits.point_domain_cells,
    )?;
    let outside = seed.parity;
    out.seed = Some(seed);
    if outside != Some(false) {
        out.reason = if outside == Some(true) {
            "seed-inside"
        } else {
            "seed-unresolved"
        };
        return Ok(out);
    }
    if !out.boundary.unresolved.is_empty() {
        out.reason = "boundary-unresolved";
        return Ok(out);
    }
    if out.boundary.contacts.len() != 2 {
        out.reason = "requires-two-crossings";
        return Ok(out);
    }
    out.boundary
        .contacts
        .sort_by(|a, b| a.parameter[0].total_cmp(&b.parameter[0]));
    let [a, b] = [&out.boundary.contacts[0], &out.boundary.contacts[1]];
    if a.parameter[1] >= b.parameter[0] {
        out.reason = "overlapping-root-intervals";
        return Ok(out);
    }
    let points = [
        rectangle_bounds(&model.faces[a.face].surface, a.uv)?,
        rectangle_bounds(&model.faces[b.face].surface, b.uv)?,
    ];
    let (lo, hi) = enclosure_distance(&points[0], &points[1])?;
    // Strict Krawczyk roots have invertible projected surface derivatives:
    // they are transverse crossings. Starting outside an embedded closed
    // material boundary, the first crossing enters material and the second
    // exits. Complete coverage excludes any hidden cavity in between.
    out.point_enclosures = Some(points);
    out.length_interval_mm = Some([lo, hi]);
    out.proven = true;
    out.reason = "material-chord";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material_segment::tests::limits;
    #[test]
    fn original_boundary_roots_admit_chords_without_rounded_endpoint_assumptions() {
        let cube = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let before = format!("{cube:?}");
        for (origin, direction) in [
            ([-2., 5., 5.], [14., 0., 0.]),
            ([12., 5., 5.], [-14., 0., 0.]),
        ] {
            let r = inspect(&cube, origin, direction, 1e-8, limits()).unwrap();
            assert!(r.proven, "{}", r.reason);
            assert_eq!(r.boundary.contacts.len(), 2);
            let length = r.length_interval_mm.unwrap();
            assert!(length[0] <= 10. && length[1] >= 10. && length[1] - length[0] < 1e-6);
            assert!(r.boundary.unresolved.is_empty());
        }
        assert_eq!(format!("{cube:?}"), before);
        let cavity = crate::operations::boolean(
            &cube,
            &crate::cuboid([4.; 3], [6.; 3]).unwrap(),
            "difference",
        )
        .unwrap();
        let r = inspect(&cavity, [-2., 5., 5.], [14., 0., 0.], 1e-8, limits()).unwrap();
        assert!(!r.proven && r.reason == "requires-two-crossings");
        assert_eq!(r.boundary.contacts.len(), 4);
        assert!(r.length_interval_mm.is_none());
        let r = inspect(&cube, [2., 5., 5.], [10., 0., 0.], 1e-8, limits()).unwrap();
        assert!(!r.proven && r.reason == "seed-inside");
        let r = inspect(&cube, [-2., 5., 5.], [12., 0., 0.], 1e-8, limits()).unwrap();
        assert!(!r.proven && r.reason == "boundary-unresolved");
    }
    #[test]
    fn curved_annular_wall_has_original_face_endpoints_and_excludes_hidden_cavities() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let r = inspect(&model, [25., 2., 3.], [-24., 0., 0.], 1e-7, limits()).unwrap();
        assert!(r.proven, "{}", r.reason);
        let expected = (400_f64 - 4.).sqrt() - (25_f64 - 4.).sqrt();
        let bounds = r.length_interval_mm.unwrap();
        assert!(bounds[0] <= expected && bounds[1] >= expected);
        assert!(bounds[1] - bounds[0] < 1e-5);
        assert_ne!(r.boundary.contacts[0].face, r.boundary.contacts[1].face);
        let r = inspect(&model, [25., 2., 3.], [-50., 0., 0.], 1e-7, limits()).unwrap();
        assert!(!r.proven && r.reason == "requires-two-crossings");
        assert_eq!(r.boundary.contacts.len(), 4);
    }
}
