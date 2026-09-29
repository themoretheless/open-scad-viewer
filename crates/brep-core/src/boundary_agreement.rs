//! Full-interval edge/pcurve lift checks, distinct from sampled validation.
use crate::Model;
use nurbs_core::{
    Error, Result,
    curve_surface_agreement::{self, Status},
};

#[derive(Clone, Debug)]
pub struct BoundaryUse {
    pub face: usize,
    pub wire: usize,
    pub coedge: usize,
    pub edge: usize,
    pub status: Status,
    pub witness: Option<f64>,
    pub witness_distance: Option<[f64; 2]>,
}
#[derive(Clone, Debug)]
pub struct Report {
    /// Only edge/lift agreement, not self-intersection or volume validity.
    pub complete: bool,
    pub cells: usize,
    pub uses: Vec<BoundaryUse>,
}

/// A shared budget includes both uses of each edge. Unprocessed uses are
/// explicitly unresolved. The model is immutable. Structural validation remains
/// mandatory, while full-interval checks replace sampled edge/lift admission.
pub fn verify(model: &Model, max_cells: usize) -> Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    if max_cells == 0 || max_cells > 1_000_000 {
        return Err(Error::new(
            "BREP_AGREEMENT_BUDGET",
            "Boundary agreement budget must be in 1..1000000",
        ));
    }
    let mut report = Report {
        complete: true,
        cells: 0,
        uses: Vec::new(),
    };
    for (face_id, face) in model.faces.iter().enumerate() {
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            for (coedge_id, coedge) in model.loops[wire].coedges.iter().enumerate() {
                let mut entry = BoundaryUse {
                    face: face_id,
                    wire,
                    coedge: coedge_id,
                    edge: coedge.edge,
                    status: Status::Unresolved,
                    witness: None,
                    witness_distance: None,
                };
                if report.cells < max_cells {
                    let checked = curve_surface_agreement::verify(
                        &model.edges[coedge.edge].curve,
                        &coedge.pcurve,
                        &face.surface,
                        coedge.reversed,
                        model.tolerance_mm,
                        (max_cells - report.cells).min(100_000),
                    )?;
                    report.cells += checked.cells;
                    entry.status = checked.status;
                    entry.witness = checked.witness;
                    entry.witness_distance = checked.witness_distance;
                }
                report.complete &= entry.status == Status::WithinTolerance;
                report.uses.push(entry);
            }
        }
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhaustion_preserves_all_boundary_uses_without_mutation() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let before = format!("{model:?}");
        let result = verify(&model, 1).unwrap();
        assert!(!result.complete);
        assert_eq!(result.cells, 1);
        assert_eq!(result.uses.len(), 24);
        assert_eq!(result.uses[0].status, Status::WithinTolerance);
        assert!(
            result.uses[1..]
                .iter()
                .all(|u| u.status == Status::Unresolved)
        );
        assert_eq!(format!("{model:?}"), before);
    }
    #[test]
    fn cylinder_and_sphere_boundary_charts_at_model_tolerance() {
        for model in [crate::cylinder(2., 3.).unwrap(), crate::sphere(2.).unwrap()] {
            let report = verify(&model, 4096).unwrap();
            assert!(report.complete);
            assert_eq!(report.uses.len(), 24);
            assert!(
                report
                    .uses
                    .iter()
                    .all(|u| u.status == Status::WithinTolerance)
            );
        }
    }
    #[test]
    fn lifted_periodic_uv_is_diagnosable_without_admitting_regular_operations() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let face = &mut model.faces[0];
        face.surface
            .control_points
            .push(face.surface.control_points[0].clone());
        face.surface.weights.push(face.surface.weights[0].clone());
        face.surface.knots_u = vec![-1., 0., 1., 2., 3.];
        face.surface.periodic_u = true;
        let wire = face.outer;
        for coedge in &mut model.loops[wire].coedges {
            for uv in &mut coedge.pcurve.control_points {
                uv[0] += 2.;
            }
        }
        let before = format!("{model:?}");
        assert!(model.validate().is_err());
        let result = verify(&model, 10_000).unwrap();
        assert!(result.complete);
        assert_eq!(result.uses.len(), 24);
        assert!(model.validate().is_err());
        assert_eq!(format!("{model:?}"), before);
    }
    #[test]
    fn sampled_mismatch_gets_a_local_diagnostic_and_bad_indices_still_refuse() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for row in &mut model.faces[0].surface.control_points {
            for cp in row {
                cp[2] += 0.125;
            }
        }
        assert!(model.validate().is_err());
        let result = verify(&model, 10_000).unwrap();
        assert!(!result.complete);
        let bad: Vec<_> = result
            .uses
            .iter()
            .filter(|u| u.status == Status::Mismatch)
            .collect();
        assert_eq!(bad.len(), 4);
        assert!(
            bad.iter()
                .all(|u| u.face == 0 && u.witness_distance.unwrap()[0] > model.tolerance_mm)
        );
        model.loops[0].coedges[0].edge = usize::MAX;
        assert!(verify(&model, 10_000).is_err());
    }
    #[test]
    fn every_cube_boundary_use_can_be_proven() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let result = verify(&model, 24).unwrap();
        assert!(result.complete);
        assert_eq!(result.uses.len(), 24);
        assert_eq!(result.cells, 24);
        assert!(
            result
                .uses
                .iter()
                .all(|u| u.status == Status::WithinTolerance)
        );
    }
}
