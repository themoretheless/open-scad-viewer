//! Whole-parameter contact diagnostics on original model edge definitions.
//! This does not admit a solid, pole G1, wall thickness or shell orientation.
use crate::{Model, Result, invalid};
use nurbs_core::contact_normal_agreement;

#[derive(Clone, Copy)]
pub struct Limits {
    pub tolerance_mm: f64,
    pub max_sine_squared: f64,
    pub agreement_cells: usize,
    pub normal_cells: usize,
    pub normal_spans: usize,
}

pub struct Report {
    pub edge: usize,
    pub faces: [usize; 2],
    pub contact: contact_normal_agreement::Report,
}

/// Follow the original shared edge and both original pcurves over their full
/// parameter domains. Reverse each pcurve according to its actual coedge use.
/// Diagnostic fields cannot substitute for a source-body admission certificate.
pub fn inspect(model: &Model, edge: usize, limits: Limits) -> Result<Report> {
    inspect_interval(model, edge, [0., 1.], limits)
}

/// Explicit normal interval in the original world's edge parameter. Excluded
/// ends stay outside this result; positions are checked over the full edge.
pub fn inspect_interval(
    model: &Model,
    edge: usize,
    interval: [f64; 2],
    limits: Limits,
) -> Result<Report> {
    model.validate()?;
    let original = model
        .edges
        .get(edge)
        .ok_or_else(|| invalid("Tangency edge is out of range"))?;
    if original.degenerate {
        return Err(invalid(
            "Collapsed edge needs a separate pole-limit qualification",
        ));
    }
    let mut uses = Vec::new();
    for (face_index, face) in model.faces.iter().enumerate() {
        for loop_index in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &model.loops[loop_index].coedges {
                if coedge.edge == edge {
                    uses.push((face_index, coedge));
                }
            }
        }
    }
    if uses.len() != 2 || uses[0].0 == uses[1].0 {
        return Err(invalid(
            "Tangency requires exactly two distinct incident faces",
        ));
    }
    let faces = [uses[0].0, uses[1].0];
    let contact = contact_normal_agreement::certify_interval(
        &original.curve,
        [
            &model.faces[faces[0]].surface,
            &model.faces[faces[1]].surface,
        ],
        [&uses[0].1.pcurve, &uses[1].1.pcurve],
        [uses[0].1.reversed, uses[1].1.reversed],
        interval,
        limits.tolerance_mm,
        limits.max_sine_squared,
        limits.agreement_cells,
        limits.normal_cells,
        limits.normal_spans,
    )?;
    Ok(Report {
        edge,
        faces,
        contact,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            tolerance_mm: 1e-7,
            max_sine_squared: 1e-4,
            agreement_cells: 256,
            normal_cells: 10000,
            normal_spans: 20000,
        }
    }
    #[test]
    fn sharp_cube_edge_has_matching_positions_and_proven_angular_break() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let report = inspect(&model, 0, limits()).unwrap();
        assert!(report.contact.positions_proven);
        assert!(!report.contact.tangent_planes_proven);
        assert!(
            report
                .contact
                .cells
                .iter()
                .any(|c| c.normals.as_ref().is_some_and(|n| n.aligned == Some(false)))
        );
        assert!(inspect(&model, model.edges.len(), limits()).is_err());
    }
    #[test]
    fn unequal_radius_seam_has_whole_parameter_agreement_without_mutation() {
        let model = crate::circular_blend::partial_annular_variable_arc(
            20.,
            5.,
            6.,
            [0.5, 1.25],
            [0.3, 0.7, 0.4],
            1e-7,
        )
        .unwrap();
        let before = value_codec::to_string(&model).unwrap();
        let edge = model.loops[model.faces[0].outer]
            .coedges
            .iter()
            .find(|a| {
                model.loops[model.faces[5].outer]
                    .coedges
                    .iter()
                    .any(|b| a.edge == b.edge)
            })
            .unwrap()
            .edge;
        let report = inspect(&model, edge, limits()).unwrap();
        assert!(report.contact.positions_proven);
        assert!(report.contact.tangent_planes_proven);
        let mut intervals = report
            .contact
            .cells
            .iter()
            .map(|c| {
                assert_eq!(c.normals.as_ref().unwrap().aligned, Some(true));
                c.interval
            })
            .collect::<Vec<_>>();
        intervals.sort_by(|a, b| a[0].total_cmp(&b[0]));
        assert_eq!(intervals.first().unwrap()[0], 0.);
        assert_eq!(intervals.last().unwrap()[1], 1.);
        for adjacent in intervals.windows(2) {
            assert_eq!(adjacent[0][1], adjacent[1][0]);
        }
        assert_eq!(value_codec::to_string(&model).unwrap(), before);
    }

    #[test]
    fn pole_is_refused_and_work_exhaustion_cannot_qualify_a_regular_seam() {
        let model = crate::circular_blend::partial_annular_variable_arc(
            20.,
            5.,
            6.,
            [0.5, 1.25],
            [0.3, 0.7, 0.4],
            1e-7,
        )
        .unwrap();
        let pole = model.edges.iter().position(|e| e.degenerate).unwrap();
        assert!(inspect(&model, pole, limits()).is_err());
        let seam = model.loops[model.faces[0].outer]
            .coedges
            .iter()
            .find(|a| {
                model.loops[model.faces[5].outer]
                    .coedges
                    .iter()
                    .any(|b| a.edge == b.edge)
            })
            .unwrap()
            .edge;
        let stopped = inspect(
            &model,
            seam,
            Limits {
                normal_cells: 1,
                normal_spans: 2,
                ..limits()
            },
        )
        .unwrap();
        assert!(!stopped.contact.tangent_planes_proven);
        assert_eq!(stopped.faces, [0, 5]);
        assert!(
            stopped
                .contact
                .cells
                .iter()
                .any(|c| c.normals.as_ref().is_none_or(|n| n.aligned != Some(true)))
        );
    }
}
