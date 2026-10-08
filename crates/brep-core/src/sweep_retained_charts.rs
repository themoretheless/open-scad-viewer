//! Whole retained wall chart sets with native shared injectivity work.
//! This does not classify intersections, cap geometry or global shell embedding.
use crate::{Error, MAX_FACES, Model, Result};
use nurbs_core::surface_linear_monotonicity::{self, Report as ChartReport};
pub struct Report {
    pub certified: bool,
    pub cells: usize,
    pub charts: Vec<(usize, ChartReport)>,
    pub unresolved_faces: Vec<usize>,
}
pub fn inspect(model: &Model, cap_faces: &[usize], max_cells: usize) -> Result<Report> {
    inspect_with_projections(model,cap_faces,max_cells,&[])
}
pub fn inspect_with_projections(model: &Model, cap_faces: &[usize], max_cells: usize, proposals: &[Option<[[i8;3];2]>]) -> Result<Report> {
    if max_cells > 100000 || model.faces.len() > MAX_FACES || cap_faces.len() > 16 {
        return Err(Error::new(
            "BREP_RETAINED_CHARTS_INVALID",
            "Invalid retained wall chart selection or budget",
        ));
    }
    let mut caps = vec![false; model.faces.len()];
    for &face in cap_faces {
        if face >= caps.len() || caps[face] {
            return Err(Error::new(
                "BREP_RETAINED_CHARTS_INVALID",
                "Invalid retained wall cap selection",
            ));
        }
        caps[face] = true;
    }
    let mut out = Report {
        certified: false,
        cells: 0,
        charts: Vec::new(),
        unresolved_faces: Vec::new(),
    };
    for (face, value) in model.faces.iter().enumerate() {
        if caps[face] {
            continue;
        }
        let proof =
            surface_linear_monotonicity::inspect_candidate_with_hint(&value.surface, proposals.get(face).copied().flatten(), max_cells - out.cells)?;
        if proof.cells > max_cells - out.cells {
            return Err(Error::new(
                "BREP_RETAINED_CHARTS_WORK",
                "Invalid native chart work accounting",
            ));
        }
        out.cells += proof.cells;
        if !proof.certified {
            out.unresolved_faces.push(face);
        }
        out.charts.push((face, proof));
    }
    out.certified = !out.charts.is_empty() && out.unresolved_faces.is_empty();
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hollow_union_keeps_face_identity_and_complete_exhausted_suffix() {
        let sections = [0., 5., 10.]
            .iter()
            .map(|&z| {
                let outer = nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap();
                let mut inner =
                    nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.2).unwrap();
                inner.control_points.reverse();
                inner.weights.reverse();
                vec![vec![outer], vec![inner]]
            })
            .collect::<Vec<_>>();
        let model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let empty_model = crate::rational_section_loft(&sections[..2]).unwrap();
        let all_faces = (0..empty_model.faces.len()).collect::<Vec<_>>();
        let empty = inspect(&empty_model, &all_faces, 10000).unwrap();
        assert!(!empty.certified && empty.charts.is_empty() && empty.cells == 0);
        let positive = inspect(&model, &caps, 10000).unwrap();
        assert!(positive.certified && positive.cells <= 10000);
        assert_eq!(
            positive.charts.iter().map(|c| c.0).collect::<Vec<_>>(),
            (0..16).collect::<Vec<_>>()
        );
        let zero = inspect(&model, &caps, 0).unwrap();
        assert!(!zero.certified && zero.cells == 0);
        assert_eq!(zero.unresolved_faces, (0..16).collect::<Vec<_>>());
        let short = inspect(&model, &caps, 1).unwrap();
        assert!(!short.certified && short.cells <= 1 && short.charts.len() == 16);
        assert!(inspect(&model, &[caps[0], caps[0]], 10000).is_err());
        assert!(inspect(&model, &[model.faces.len()], 10000).is_err());
        assert!(inspect(&model, &caps, 100001).is_err());
        let mut folded = model.clone();
        let s = &mut folded.faces[0].surface;
        s.control_points[2] = s.control_points[0].clone();
        let r = inspect(&folded, &caps, 10000).unwrap();
        assert!(!r.certified && r.unresolved_faces.contains(&0));
        let mut proposals=vec![None;model.faces.len()];
        for (face,chart) in &positive.charts { proposals[*face]=Some(chart.projection); }
        let hinted=inspect_with_projections(&folded,&caps,10000,&proposals).unwrap();
        assert!(!hinted.certified && hinted.unresolved_faces.contains(&0));
    }
}
