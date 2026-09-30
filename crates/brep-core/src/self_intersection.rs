//! Combined sufficient proof of absence of self-intersections of boundary charts.
//! This is not a certificate of shell orientation, nesting or filled-volume validity.
use crate::{Model, Result, face_contacts, face_injectivity};

#[derive(Clone, Debug)]
pub struct Report {
    /// Every face is injective and every distinct pair is disjoint or meets only
    /// at a certified authored shared boundary. False means unproven, not invalid.
    pub absence_proven: bool,
    pub faces: face_injectivity::Report,
    pub pairs: face_contacts::Report,
}

pub fn inspect(
    model: &Model,
    tolerance_uv: f64,
    max_spans: usize,
    limits: face_contacts::Limits,
) -> Result<Report> {
    let faces = face_injectivity::inspect(model, max_spans)?;
    let pairs = face_contacts::inspect(model, tolerance_uv, limits)?;
    let absence_proven = faces.all_faces_injective && pairs.all_pairs_classified;
    Ok(Report { absence_proven, faces, pairs })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> face_contacts::Limits {
        face_contacts::Limits { pairs: 100, cells: 10000, domain_cells: 100000,
            cells_per_pair: 16, domain_cells_per_pair: 1000 }
    }
    #[test]
    fn cube_requires_both_face_and_pair_proofs() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let report = inspect(&model, 1e-8, 6, limits()).unwrap();
        assert!(report.absence_proven);
        assert_eq!(report.faces.faces.len(), 6);
        assert_eq!(report.pairs.pairs.len(), 15);
        let report = inspect(&model, 1e-8, 1, limits()).unwrap();
        assert!(!report.absence_proven);
        assert!(report.pairs.all_pairs_classified);
        assert_eq!(report.faces.faces.iter().filter(|f| f.result.is_none()).count(), 5);
        let report = inspect(&model, 1e-8, 6, face_contacts::Limits { pairs: 1, ..limits() }).unwrap();
        assert!(!report.absence_proven);
        assert!(report.faces.all_faces_injective);
        assert!(report.pairs.next_pair.is_some());
    }
    #[test]
    fn collapsed_chart_never_passes_combined_proof() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for row in &mut model.faces[0].surface.control_points {
            for point in row { *point = vec![0.; 3]; }
        }
        let report = inspect(&model, 1e-8, 6, limits()).unwrap();
        assert!(!report.absence_proven);
        assert!(!report.faces.faces[0].result.as_ref().unwrap().proven);
    }
    #[test]
    fn injective_faces_with_an_interior_contact_do_not_pass() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        // Each chart is a regular plane, but the first two cross internally.
        for (face, f) in model.faces.iter_mut().enumerate() {
            f.surface.control_points = (0..2).map(|u| (0..2).map(|v| {
                if face == 1 { vec![u as f64, 0.5, v as f64 - 0.5] }
                else { vec![u as f64, v as f64, face as f64 * 10.] }
            }).collect()).collect();
        }
        let report = inspect(&model, 1e-8, 6, limits()).unwrap();
        assert!(report.faces.all_faces_injective);
        assert!(!report.absence_proven);
        assert!(report.pairs.next_pair.is_none());
        assert_eq!(report.pairs.pairs.len(), 15);
        assert_eq!(report.pairs.pairs[0].reason, "interior-contact");
        assert!(report.pairs.pairs[0].result.as_ref().unwrap().contact.is_some());
        assert!(report.pairs.pairs[1..].iter().all(|p| p.reason == "pair-disjoint"));
    }
}
