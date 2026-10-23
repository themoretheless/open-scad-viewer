//! Contact witnesses restricted to the authored trimmed face interiors.
//! A section result never establishes completeness for the pair or the solid.
use crate::{Error, Model, Result, face_domain::FaceDomain};
use nurbs_core::{
    surface_contact::{self, Witness},
    trim_domain::Location,
};

#[derive(Clone, Debug)]
pub enum Status {
    Excluded,
    Unresolved,
    InteriorContact(Witness),
}
#[derive(Clone, Debug)]
pub struct Report {
    pub faces: [usize; 2],
    pub status: Status,
    pub domain_cells: usize,
    pub reason: &'static str,
}

pub fn inspect_section(
    model: &Model,
    faces: [usize; 2],
    fixed_axis: usize,
    fixed: f64,
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
    tolerance_uv: f64,
    max_domain_cells: usize,
) -> Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= model.faces.len())
        || !(1..=1_000_000).contains(&max_domain_cells)
    {
        return Err(Error::new(
            "BREP_CONTACT_INPUT",
            "Choose two distinct existing faces and a UV work budget in 1..1000000",
        ));
    }
    let domains = [
        FaceDomain::new(model, faces[0], tolerance_uv)?,
        FaceDomain::new(model, faces[1], tolerance_uv)?,
    ];
    let mut report = Report {
        faces,
        status: Status::Unresolved,
        domain_cells: 0,
        reason: "section-unresolved",
    };
    let witness = match surface_contact::certify(
        &model.faces[faces[0]].surface,
        &model.faces[faces[1]].surface,
        fixed_axis,
        fixed,
        first_other,
        second,
    )? {
        surface_contact::Verdict::Excluded => {
            report.status = Status::Excluded;
            report.reason = "section-excluded";
            return Ok(report);
        }
        surface_contact::Verdict::Unresolved => return Ok(report),
        surface_contact::Verdict::Witness(w) => w,
    };
    // FaceDomain classifies one authored UV chart. Periodic lifted loops need
    // all equivalent chart branches before an outside result is meaningful.
    if faces
        .iter()
        .any(|&i| model.faces[i].surface.periodic_u || model.faces[i].surface.periodic_v)
    {
        return Ok(Report {
            faces,
            status: Status::Unresolved,
            domain_cells: 0,
            reason: "periodic-trim-not-supported",
        });
    }
    // Uniqueness in the entire section box makes exclusion of its sole root
    // sufficient. Never infer exclusion from a midpoint or a sampled polyline.
    let mut inside = true;
    for (domain, uv) in domains.iter().zip([witness.first_uv, witness.second_uv]) {
        if report.domain_cells == max_domain_cells {
            report.reason = "uv-work-limit";
            return Ok(report);
        }
        let result = domain.classify(uv, (max_domain_cells - report.domain_cells).min(100_000))?;
        report.domain_cells += result.cells;
        if result.location == Location::Outside {
            report.status = Status::Excluded;
            report.reason = "unique-root-outside-trim";
            return Ok(report);
        }
        inside &= result.location == Location::Inside;
    }
    if inside {
        report.status = Status::InteriorContact(witness);
        report.reason = "both-trim-interiors";
    } else {
        report.reason = "trim-unresolved";
    }
    Ok(report)
}

/// Search a complete pair of authored trimmed faces. The returned unresolved
/// boxes preserve the coverage limitation even when a contact is found.
pub fn search_pair(
    model: &Model,
    faces: [usize; 2],
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<nurbs_core::surface_contact_search::Report> {
    model.validate_boundary_diagnostic_inputs()?;
    if faces[0] == faces[1] || faces.iter().any(|&f| f >= model.faces.len()) {
        return Err(Error::new(
            "BREP_CONTACT_INPUT",
            "Choose two distinct existing faces",
        ));
    }
    let a = FaceDomain::new(model, faces[0], tolerance_uv)?;
    let b = FaceDomain::new(model, faces[1], tolerance_uv)?;
    nurbs_core::surface_contact_search::search_trimmed(
        &model.faces[faces[0]].surface,
        &model.faces[faces[1]].surface,
        [&a.region, &b.region],
        max_cells,
        max_domain_cells,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn crossing() -> Model {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for (i, face) in m.faces[..2].iter_mut().enumerate() {
            face.surface.control_points = if i == 0 {
                vec![
                    vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                    vec![vec![1., 0., 0.], vec![1., 1., 0.]],
                ]
            } else {
                vec![
                    vec![vec![0., 0.5, -0.5], vec![0., 0.5, 0.5]],
                    vec![vec![1., 0.5, -0.5], vec![1., 0.5, 0.5]],
                ]
            };
        }
        m
    }
    fn run(m: &Model, budget: usize) -> Report {
        inspect_section(m, [0, 1], 0, 0.5, [0., 1.], [[0., 1.]; 2], 1e-8, budget).unwrap()
    }
    #[test]
    fn both_interiors_and_budget_preserve_uncertainty() {
        let m = crossing();
        assert!(matches!(run(&m, 1000).status, Status::InteriorContact(_)));
        let limited = run(&m, 1);
        assert!(matches!(limited.status, Status::Unresolved));
        assert!(limited.domain_cells <= 1);
    }
    #[test]
    fn either_trim_can_exclude_the_unique_surface_contact() {
        for face in 0..2 {
            let mut m = crossing();
            let outer = m.faces[face].outer;
            for c in &mut m.loops[outer].coedges {
                for p in &mut c.pcurve.control_points {
                    p[0] *= 0.25;
                    p[1] *= 0.25;
                }
            }
            let r = run(&m, 1000);
            assert!(matches!(r.status, Status::Excluded));
            assert_eq!(r.reason, "unique-root-outside-trim");
        }
    }
    #[test]
    fn trim_boundary_never_becomes_an_interior_defect() {
        let mut m = crossing();
        let outer = m.faces[0].outer;
        for c in &mut m.loops[outer].coedges {
            for p in &mut c.pcurve.control_points {
                p[0] *= 0.5;
            }
        }
        assert!(matches!(run(&m, 1000).status, Status::Unresolved));
        assert!(matches!(
            run(&crossing(), 1_000_000).status,
            Status::InteriorContact(_)
        ));
    }
    #[test]
    fn hole_excludes_a_contact_of_the_underlying_surfaces() {
        use nurbs_core::curve::Curve;
        let outer = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![10., 0.],
            vec![10., 10.],
            vec![0., 10.],
            vec![0., 0.],
        ])
        .unwrap();
        let hole = Curve::from_polyline(vec![
            vec![3., 3.],
            vec![3., 7.],
            vec![7., 7.],
            vec![7., 3.],
            vec![3., 3.],
        ])
        .unwrap();
        let mut m = crate::prism::extrude(&[vec![outer], vec![hole]], 0., 2.).unwrap();
        let caps: Vec<_> = m
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.holes.is_empty())
            .map(|(i, _)| i)
            .collect();
        assert_eq!(caps.len(), 2);
        let template = crossing();
        for i in 0..2 {
            m.faces[caps[i]].surface = template.faces[i].surface.clone();
        }
        let r = inspect_section(
            &m,
            [caps[0], caps[1]],
            0,
            0.5,
            [0., 1.],
            [[0., 1.]; 2],
            1e-8,
            1000,
        )
        .unwrap();
        assert!(matches!(r.status, Status::Excluded));
        assert_eq!(r.reason, "unique-root-outside-trim");
        // A root in the hole must not hide other contacts of the same pair.
        let all = search_pair(&m, [caps[0], caps[1]], 1e-8, 10000, 1000000).unwrap();
        assert!(all.contact.is_some(), "{} unresolved", all.unresolved.len());
        assert!(!all.absence_proven);
    }
    #[test]
    fn pair_search_filters_surface_extensions_and_preserves_budget_limits() {
        let mut m = crossing();
        let r = search_pair(&m, [0, 1], 1e-8, 1000, 100000).unwrap();
        assert!(r.contact.is_some());
        assert!(!r.absence_proven);
        let limited = search_pair(&m, [0, 1], 1e-8, 1000, 1).unwrap();
        assert!(limited.contact.is_none());
        assert!(!limited.absence_proven);
        assert!(limited.domain_cells <= 1);
        assert!(!limited.unresolved.is_empty());
        let outer = m.faces[0].outer;
        for c in &mut m.loops[outer].coedges {
            for p in &mut c.pcurve.control_points {
                p[1] *= 0.25;
            }
        }
        let r = search_pair(&m, [0, 1], 1e-8, 10000, 1000000).unwrap();
        assert!(r.contact.is_none());
        assert!(
            r.absence_proven,
            "{} cells, {} domain cells, {} unresolved",
            r.cells,
            r.domain_cells,
            r.unresolved.len()
        );
    }
}
