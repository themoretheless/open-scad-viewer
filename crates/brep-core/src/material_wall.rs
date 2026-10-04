//! Bounds the minimum aligned material chord between two unions of original faces.
//! Complete pair coverage supplies the lower bound; a certified material chord
//! supplies the upper bound. A surface-clearance witness alone is insufficient.
use crate::{Error, Model, Result, material_chord, material_segment, shell_distance};

pub struct Limits {
    pub material: material_segment::Limits,
    pub distance_cells: usize,
    pub distance_domain_cells: usize,
    pub normal_spans: usize,
}
pub struct Report {
    pub interval_mm: Option<[f64; 2]>,
    pub converged: bool,
    pub reason: &'static str,
    pub clearance: shell_distance::ShellDistance,
    pub candidate: material_chord::NormalReport,
}
/// The scope is the supplied face unions, not the entire body. The admissible
/// class comprises straight interior material chords meeting the stated normal
/// angle tolerance at both endpoints. All selected face pairs contribute to the
/// lower bound, even when distance subdivision exhausts its work budget.
pub fn inspect(
    model: &Model,
    groups: [&[usize]; 2],
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_sine_squared: f64,
    limits: Limits,
) -> Result<Report> {
    if groups[0].iter().any(|face| groups[1].contains(face)) {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Opposing wall face groups must be disjoint",
        ));
    }
    let clearance = shell_distance::distance_between_face_sets(
        model,
        groups[0],
        model,
        groups[1],
        tolerance_mm,
        tolerance_uv,
        limits.distance_cells,
        limits.distance_domain_cells,
    )?;
    let candidate = material_chord::inspect_with_normals(
        model,
        origin,
        direction,
        tolerance_uv,
        limits.material,
        max_sine_squared,
        limits.normal_spans,
    )?;
    let reason = if !candidate.chord.proven {
        "material-chord-unproven"
    } else if candidate.aligned != Some(true) {
        if candidate.aligned == Some(false) {
            "candidate-oblique"
        } else {
            "candidate-normal-unresolved"
        }
    } else {
        let contacts = &candidate.chord.boundary.contacts;
        let faces = [contacts[0].face, contacts[1].face];
        if !(groups[0].contains(&faces[0]) && groups[1].contains(&faces[1])
            || groups[0].contains(&faces[1]) && groups[1].contains(&faces[0]))
        {
            "candidate-outside-groups"
        } else {
            "material-thickness-bounds"
        }
    };
    let interval_mm = if reason == "material-thickness-bounds" {
        let lower = clearance.lower_bound_mm;
        let upper = candidate
            .chord
            .length_interval_mm
            .expect("proven chord has a length interval")[1];
        if !lower.is_finite() || !upper.is_finite() || lower < 0. || lower > upper {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "Material thickness bounds are inconsistent",
            ));
        }
        Some([lower, upper])
    } else {
        None
    };
    let converged = interval_mm.is_some_and(|d| (d[1] - d[0]).next_up() <= tolerance_mm);
    Ok(Report {
        interval_mm,
        converged,
        reason,
        clearance,
        candidate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            material: crate::material_segment::tests::limits(),
            distance_cells: 10000,
            distance_domain_cells: 1000000,
            normal_spans: 100,
        }
    }
    fn radial(model: &Model, groups: [&[usize]; 2], budget: Limits) -> Report {
        inspect(
            model,
            groups,
            [15., 20., 3.],
            [-14.4, -19.2, 0.],
            1e-5,
            1e-7,
            1e-6,
            budget,
        )
        .unwrap()
    }
    #[test]
    fn placed_annular_wall_qualifies_original_face_groups() {
        let source =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let model = crate::transform::affine(
            &source,
            [
                [0., -1., 0., 123.],
                [0., 0., -1., -45.],
                [1., 0., 0., 67.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let aa = [2, 7, 12, 16, 20, 24];
        let bb = [3, 8, 13, 17, 21, 25];
        let budget = limits();
        let r = inspect(
            &model,
            [&aa, &bb],
            [103., -48., 82.],
            [19.2, 0., -14.4],
            1e-5,
            1e-7,
            1e-6,
            budget,
        )
        .unwrap();
        assert!(r.candidate.chord.validity.boundary.proven);
        let interval = r.interval_mm.unwrap();
        assert!(r.converged && interval[0] <= 15. && interval[1] >= 15.);
        assert!(interval[1] - interval[0] <= 1e-5);
        assert!(r.clearance.converged && r.clearance.lower_bound_mm > 14.99);
        let mut exhausted = limits();
        exhausted.material.volume.boundary.contacts.pairs = 1;
        exhausted.material.volume.boundary.contacts.cells = 1;
        exhausted.material.volume.boundary.contacts.cells_per_pair = 1;
        let limited = inspect(&model, [&aa,&bb], [103.,-48.,82.], [19.2,0.,-14.4],
            1e-5,1e-7,1e-6,exhausted).unwrap();
        assert!(!limited.converged && limited.interval_mm.is_none());
        assert_eq!(limited.candidate.chord.reason,"volume-unproven");
    }
    #[test]
    fn full_wall_groups_have_bounds_from_material_not_surface_gap() {
        let m =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let before = format!("{m:?}");
        let a = [2, 7, 12, 16, 20, 24];
        let b = [3, 8, 13, 17, 21, 25];
        let r = radial(&m, [&a, &b], limits());
        let d = r.interval_mm.unwrap();
        assert!(r.converged && d[0] <= 15. && d[1] >= 15. && d[1] - d[0] <= 1e-5);
        assert_eq!(r.clearance.pairs, 36);
        let expanded = [2, 7, 12, 16, 20, 24, 0, 5, 10];
        let mut exhausted = limits();
        exhausted.distance_cells = 1;
        let broad = radial(&m, [&expanded, &b], exhausted);
        let bracket = broad.interval_mm.unwrap();
        assert_eq!(broad.clearance.pairs, 54);
        assert!(bracket[0] <= 13.75 && bracket[1] >= 15. && !broad.converged);
        let reversed = radial(&m, [&b, &a], limits());
        assert_eq!(reversed.interval_mm, r.interval_mm);
        assert_eq!(format!("{m:?}"), before);
        let oblique = inspect(
            &m,
            [&a, &b],
            [25., 2., 3.],
            [-24., 0., 0.],
            1e-5,
            1e-7,
            1e-6,
            limits(),
        )
        .unwrap();
        assert_eq!(oblique.reason, "candidate-oblique");
        assert!(oblique.clearance.converged && oblique.interval_mm.is_none());
        let hole = inspect(
            &m,
            [&a, &b],
            [25., 2., 3.],
            [-50., 0., 0.],
            1e-5,
            1e-7,
            1e-6,
            limits(),
        )
        .unwrap();
        assert_eq!(hole.reason, "material-chord-unproven");
        assert!(hole.interval_mm.is_none() && !hole.converged);
    }
    #[test]
    fn scope_limits_and_wrong_endpoints_cannot_admit_thickness() {
        let m = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let face = |axis: usize, value: f64| {
            m.faces
                .iter()
                .position(|f| {
                    f.surface
                        .control_points
                        .iter()
                        .flatten()
                        .all(|p| p[axis] == value)
                })
                .unwrap()
        };
        let a = [face(0, 0.)];
        let b = [face(0, 10.)];
        let other = [face(1, 10.)];
        let measure = |groups, limits| {
            inspect(
                &m,
                groups,
                [-2., 5., 5.],
                [14., 0., 0.],
                1e-5,
                1e-7,
                1e-6,
                limits,
            )
        };
        let r = measure([&a[..], &b[..]], limits()).unwrap();
        assert!(
            r.converged && r.interval_mm.unwrap()[0] <= 10. && r.interval_mm.unwrap()[1] >= 10.
        );
        let r = measure([&a[..], &other[..]], limits()).unwrap();
        assert_eq!(r.reason, "candidate-outside-groups");
        assert!(r.interval_mm.is_none());
        let mut budget = limits();
        budget.normal_spans = 1;
        let r = measure([&a[..], &b[..]], budget).unwrap();
        assert_eq!(r.reason, "candidate-normal-unresolved");
        assert!(r.interval_mm.is_none());
        let mut reversed = m.clone();
        for face in &mut reversed.shells[0].faces {
            face.reversed = !face.reversed;
        }
        let invalid = inspect(
            &reversed,
            [&a, &b],
            [-2., 5., 5.],
            [14., 0., 0.],
            1e-5,
            1e-7,
            1e-6,
            limits(),
        )
        .unwrap();
        assert_eq!(invalid.reason, "material-chord-unproven");
        assert!(invalid.interval_mm.is_none() && !invalid.converged);
        for groups in [
            [&a[..], &a[..]],
            [&[][..], &b[..]],
            [&[a[0], a[0]][..], &b[..]],
        ] {
            assert!(measure(groups, limits()).is_err())
        }
    }
}
