//! Whole-boundary Jacobian evidence. All authored faces, including filled caps,
//! share one budget. This is piecewise regularity: crease edges may remain C0.
use crate::{Error, Model, Result};

#[derive(Clone, Debug)]
pub struct Report {
    pub all_faces_regular: bool,
    /// Ordinary nonzero-Jacobian charts or a complete authored pole blow-up.
    /// Pole quotient regularity is stratified, not a claim of G1 at the pole.
    pub all_faces_regular_modulo_poles: bool,
    pub pole_quotients: Vec<Option<PoleReport>>,
    pub cells: usize,
    pub faces: Vec<Option<nurbs_core::surface_regularity::Report>>,
    pub unresolved_faces: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct PoleReport {
    pub edge: usize,
    pub vertex: usize,
    pub collapsed_end: usize,
    pub proof: nurbs_core::surface_quotient_injectivity::PolarReport,
}

pub fn inspect(model: &Model, max_cells: usize) -> Result<Report> {
    model.validate()?;
    if max_cells > 100000 {
        return Err(Error::new(
            "BREP_REGULARITY_BUDGET",
            "Regularity budget exceeds 100000",
        ));
    }
    // Validation precedes budget handling so malformed geometry cannot masquerade
    // as an ordinary unresolved certificate.
    for face in &model.faces {
        face.surface.validate()?;
    }
    let mut out = Report {
        all_faces_regular: false,
        all_faces_regular_modulo_poles: false,
        pole_quotients: Vec::with_capacity(model.faces.len()),
        cells: 0,
        faces: Vec::with_capacity(model.faces.len()),
        unresolved_faces: Vec::new(),
    };
    for (index, face) in model.faces.iter().enumerate() {
        let remaining = max_cells - out.cells;
        if let Some((end,edge,vertex)) = crate::face_injectivity::declared_pole(model,index) {
            let proof = if remaining >= 64 {
                Some(nurbs_core::surface_quotient_injectivity::certify_polar_source_frame(
                    &face.surface,end,8,remaining)?)
            } else { None };
            out.cells += proof.as_ref().map_or(0,|p|p.cells);
            if !proof.as_ref().is_some_and(|p|p.proven) {out.unresolved_faces.push(index);}
            out.pole_quotients.push(proof.map(|proof|PoleReport {edge,vertex,collapsed_end:end,proof}));
            out.faces.push(None);
            continue;
        }
        out.pole_quotients.push(None);
        let report = if remaining == 0 {
            None
        } else {
            match nurbs_core::surface_regularity::inspect(&face.surface, remaining) {
                Ok(report) => {
                    out.cells += report.cells;
                    Some(report)
                }
                Err(_) => {
                    // An exhausted initial grid or numerical enclosure is not
                    // evidence. Reserve the remainder; never restart per face.
                    out.cells = max_cells;
                    None
                }
            }
        };
        if !report.as_ref().is_some_and(|r| r.spanwise_regular) {
            out.unresolved_faces.push(index);
        }
        out.faces.push(report);
    }
    out.all_faces_regular_modulo_poles = !model.faces.is_empty() && out.unresolved_faces.is_empty();
    out.all_faces_regular = out.all_faces_regular_modulo_poles
        && out.pole_quotients.iter().all(Option::is_none);
    Ok(out)
}

pub struct JointReport {
    /// Closed embedded piecewise regular boundary, including exact C0 seams.
    /// Source-frame transport, G1/G2, nesting and material orientation are separate.
    pub proven: bool,
    pub embedding: super::Report,
    pub regularity: Report,
}

pub fn inspect_joint(
    model: &Model,
    tolerance_uv: f64,
    limits: super::Limits,
    max_linear_cells: usize,
    max_regularity_cells: usize,
) -> Result<JointReport> {
    let regularity = inspect(model, max_regularity_cells)?;
    let embedding = super::inspect_with_linear(model, tolerance_uv, limits, max_linear_cells)?;
    let proven = embedding.proven && regularity.all_faces_regular_modulo_poles;
    Ok(JointReport {
        proven,
        embedding,
        regularity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> super::super::Limits {
        super::super::Limits {
            exact_work: 1000000,
            trim_pairs: 1000,
            trim_cells: 10000,
            trim_domain_cells: 100000,
            spans: 100,
            contacts: crate::face_contacts::Limits {
                pairs: 100,
                cells: 10000,
                domain_cells: 100000,
                cells_per_pair: 16,
                domain_cells_per_pair: 1000,
            },
        }
    }
    #[test]
    fn all_faces_and_exact_closure_require_both_independent_proofs() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let proof = inspect_joint(&model, 1e-8, limits(), 1000, 1000).unwrap();
        assert!(proof.proven);
        assert_eq!(proof.regularity.faces.len(), 6);
        let short =
            inspect_joint(&model, 1e-8, limits(), 1000, proof.regularity.cells - 1).unwrap();
        assert!(short.embedding.proven);
        assert!(!short.proven);
        assert!(!short.regularity.unresolved_faces.is_empty());
        let mut low = limits();
        low.contacts.pairs = 1;
        let short = inspect_joint(&model, 1e-8, low, 1000, 1000).unwrap();
        assert!(short.regularity.all_faces_regular);
        assert!(!short.proven);
    }
    #[test]
    fn zero_work_and_invalid_geometry_are_distinct() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let zero = inspect(&model, 0).unwrap();
        assert!(!zero.all_faces_regular);
        assert_eq!(zero.cells, 0);
        assert_eq!(zero.unresolved_faces, (0..6).collect::<Vec<_>>());
        model.faces[5].surface.weights[0][0] = -1.;
        assert!(inspect(&model, 0).is_err());
    }
    #[test]
    fn singular_cap_cannot_hide_behind_regular_walls() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let s = &mut model.faces[5].surface;
        // Preserve all four boundary curves and fold only the cap interior.
        // Topological incidence and sampled edge agreement still pass.
        let a = s.control_points[0][0].clone();
        let b = s.control_points[1][0].clone();
        let c = s.control_points[0][1].clone();
        s.control_points = (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        let u = if (1..=2).contains(&j) && i == 1 {
                            2.
                        } else if (1..=2).contains(&j) && i == 2 {
                            -1.
                        } else {
                            i as f64 / 3.
                        };
                        let v = j as f64 / 3.;
                        (0..3)
                            .map(|k| a[k] + u * (b[k] - a[k]) + v * (c[k] - a[k]))
                            .collect()
                    })
                    .collect()
            })
            .collect();
        s.degree_u = 3;
        s.degree_v = 3;
        s.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
        s.knots_v = s.knots_u.clone();
        s.weights = vec![vec![1.; 4]; 4];
        let proof = inspect(&model, 100).unwrap();
        assert!(!proof.all_faces_regular);
        assert_eq!(proof.unresolved_faces, vec![5]);
        assert!(
            proof.faces[..5]
                .iter()
                .all(|r| r.as_ref().unwrap().spanwise_regular)
        );
    }
}
