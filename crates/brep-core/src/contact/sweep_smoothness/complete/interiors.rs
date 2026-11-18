//! Smoothness at the original knot lines of every face.
use crate::{Model, Result, Surface};
use nurbs_core::analysis::surface_continuity;

mod geometric;

pub struct InteriorReport {
    pub certified: bool,
    pub exact_work: u64,
    pub cells: usize,
    pub unresolved_face: Option<usize>,
    /// Exact geometric joins proved after partitioning the original chart.
    pub geometric_seams: usize,
}

fn axis_basis_order(knots: &[f64], degree: usize, controls: usize, order: usize) -> bool {
    let [lo, hi] = [knots[degree], knots[controls]];
    knots
        .chunk_by(|a, b| a == b)
        .filter(|block| lo < block[0] && block[0] < hi)
        .all(|block| degree >= block.len() + order)
}

fn basis_order(surface: &Surface, order: usize) -> bool {
    !surface.periodic_u
        && !surface.periodic_v
        && axis_basis_order(
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
            order,
        )
        && axis_basis_order(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
            order,
        )
}

pub(super) fn inspect(
    model: &Model,
    order: usize,
    cells: usize,
    exact_work: u64,
) -> Result<InteriorReport> {
    let mut report = InteriorReport {
        certified: false,
        exact_work: 0,
        cells: 0,
        unresolved_face: None,
        geometric_seams: 0,
    };
    for (face, record) in model.faces.iter().enumerate() {
        let surface = &record.surface;
        if basis_order(surface, order) {
            continue;
        }
        let proof = surface_continuity::qualify(
            surface,
            order,
            cells - report.cells,
            exact_work - report.exact_work,
        )?;
        report.exact_work += proof.exact_work;
        report.cells += proof.cells;
        if proof.certificate.is_some() {
            continue;
        }
        let geometric = geometric::inspect(
            surface,
            order,
            cells - report.cells,
            exact_work - report.exact_work,
        )?;
        report.cells += geometric.cells;
        report.exact_work += geometric.work;
        report.geometric_seams += geometric.seams;
        if !geometric.certified {
            report.unresolved_face = Some(face);
            return Ok(report);
        }
    }
    report.certified = !model.faces.is_empty();
    Ok(report)
}

#[cfg(test)]
mod tests {

    #[test]
    fn complete_boundary_inspects_internal_curvature_jumps_separately_from_edges() {
        let mut model = crate::cuboid([0.; 3], [3.; 3]).unwrap();
        let index = model
            .faces
            .iter()
            .position(|face| {
                face.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] == 3.)
            })
            .unwrap();
        let surface = &mut model.faces[index].surface;
        let [u0, u1] = [surface.knots_u[1], surface.knots_u[2]];
        let [v0, v1] = [surface.knots_v[1], surface.knots_v[2]];
        let um = (u0 + u1) * 0.5;
        let origin = surface.control_points[0][0].clone();
        let u: Vec<_> = surface.control_points[1][0]
            .iter()
            .zip(&origin)
            .map(|(x, a)| (x - a) / 3.)
            .collect();
        let v: Vec<_> = surface.control_points[0][1]
            .iter()
            .zip(&origin)
            .map(|(x, a)| (x - a) / 3.)
            .collect();
        surface.degree_u = 3;
        surface.degree_v = 3;
        surface.knots_u = vec![u0, u0, u0, u0, um, um, u1, u1, u1, u1];
        surface.knots_v = vec![v0, v0, v0, v0, v1, v1, v1, v1];
        surface.control_points = [0., 0.5, 1., 2., 2.5, 3.]
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                (0..4)
                    .map(|j| {
                        let mut point = (0..3)
                            .map(|axis| origin[axis] + u[axis] * x + v[axis] * j as f64)
                            .collect::<Vec<_>>();
                        if i == 2 && j == 1 {
                            point[2] += 1. / 32.;
                        }
                        point
                    })
                    .collect()
            })
            .collect();
        surface.weights = vec![vec![1.; 4]; 6];
        let limits = super::super::Limits {
            order: 2,
            exact_work: 16_000_000,
            agreement_work: 1_000_000,
            regularity_cells: 100_000,
        };
        let c2 = super::super::inspect(&model, limits).unwrap();
        assert!(c2.regularity.all_faces_regular);
        assert!(c2.extraction_complete);
        assert_eq!(c2.face_interiors.unresolved_face, Some(index));
        assert!(c2.face_interiors.exact_work > 0);
        assert!(!c2.face_interiors.certified && !c2.g2_certified);
        assert!(c2.exact_work <= limits.exact_work);
        assert!(c2.regularity.cells + c2.face_interiors.cells <= limits.regularity_cells);

        let c1 =
            super::super::inspect(&model, super::super::Limits { order: 1, ..limits }).unwrap();
        assert!(c1.face_interiors.certified);
        assert_eq!(c1.face_interiors.unresolved_face, None);

        model.faces[index].surface.control_points[2][1][2] -= 1. / 32.;
        let smooth = super::super::inspect(&model, limits).unwrap();
        assert!(smooth.face_interiors.certified);
        assert!(smooth.face_interiors.exact_work > 0);
        let short = super::super::inspect(
            &model,
            super::super::Limits {
                exact_work: smooth.exact_work - 1,
                ..limits
            },
        )
        .unwrap();
        assert!(!short.face_interiors.certified);
        assert_eq!(short.face_interiors.unresolved_face, Some(index));
        assert!(short.exact_work < smooth.exact_work);
    }
}
