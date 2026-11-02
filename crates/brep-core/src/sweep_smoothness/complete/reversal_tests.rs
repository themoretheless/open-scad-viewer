use super::*;

fn strip(y: f64) -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![-2., -2., -2., -2., 0., 2., 2., 2., 2.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..5)
            .map(|x| vec![vec![x as f64, y, 0.], vec![x as f64, y + 1., 0.]])
            .collect(),
        weights: vec![vec![1.; 2]; 5],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn reversed_multispan_basis_retains_exact_g2_and_charges_reflection_work() {
    let a = strip(0.);
    let mut b = strip(1.);
    b.control_points.reverse();
    b.weights.reverse();
    let reversed = b.clone();
    let (accepted, work) = reverse_along(&mut b, "vMin", 1000000).unwrap();
    assert!(accepted && work > 0 && work < 1000000);
    assert_eq!(b.knots_u, reversed.knots_u);
    assert_eq!(b.control_points, strip(1.).control_points);
    let proof = continuity::inspect_surface_projective_strip_jets(
        &a,
        &b,
        "vMax",
        "vMin",
        2,
        1.,
        1000000 - work,
    )
    .unwrap();
    assert!(
        proof.certified && proof.exact_identity && proof.regularity_certified,
        "{proof:?}"
    );
    for budget in [0, work - 1] {
        let mut short = reversed.clone();
        let (accepted, used) = reverse_along(&mut short, "vMin", budget).unwrap();
        assert!(!accepted && used <= budget);
        assert_eq!(short, reversed);
    }
    let mut twice = b.clone();
    assert!(reverse_along(&mut twice, "vMin", work).unwrap().0);
    assert_eq!(twice, reversed);
}

#[test]
fn exact_asymmetric_reversal_preserves_bits_and_rounded_symmetry_is_refused() {
    let mut asym = strip(1.);
    asym.knots_u[4] = f64::from_bits(1);
    let before = asym.clone();
    assert!(reverse_along(&mut asym, "vMin", 1000000).unwrap().0);
    assert_eq!(asym.knots_u[4], -f64::from_bits(1));
    assert!(reverse_along(&mut asym, "vMin", 1000000).unwrap().0);
    assert_eq!(asym, before);
    let mut rounded = strip(1.);
    rounded.knots_u = vec![0.1, 0.1, 0.1, 0.1, 0.5, 0.9, 0.9, 0.9, 0.9];
    assert_eq!(
        (rounded.knots_u[0] + rounded.knots_u[8]) / 2.,
        rounded.knots_u[4]
    );
    let before = rounded.clone();
    assert!(!reverse_along(&mut rounded, "vMin", 1000000).unwrap().0);
    assert_eq!(rounded, before);
}

#[test]
fn both_parameter_axes_use_the_same_exact_reflection_and_invalid_periodic_stays_refused() {
    let mut s = strip(0.);
    let before = s.clone();
    s.control_points = (0..2)
        .map(|i| before.control_points.iter().map(|r| r[i].clone()).collect())
        .collect();
    s.weights = vec![vec![1.; 5]; 2];
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let before = s.clone();
    assert!(reverse_along(&mut s, "uMin", 1000000).unwrap().0);
    for (a, b) in s.control_points.iter().zip(&before.control_points) {
        assert_eq!(
            a.iter().collect::<Vec<_>>(),
            b.iter().rev().collect::<Vec<_>>()
        );
    }
    let mut periodic = before.clone();
    periodic.periodic_v = true;
    let before = periodic.clone();
    assert_eq!(
        reverse_along(&mut periodic, "uMin", 1000000).unwrap(),
        (false, 0)
    );
    assert_eq!(periodic, before);
}

#[test]
fn complete_owner_accounts_for_multispan_reversal_without_promoting_sharp_caps() {
    let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    for f in &mut model.faces {
        let a = f.surface.control_points[0].clone();
        let b = f.surface.control_points[1].clone();
        let mid = a
            .iter()
            .zip(&b)
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) * 0.5).collect())
            .collect();
        f.surface.control_points = vec![a, mid, b];
        f.surface.weights = vec![vec![1.; 2]; 3];
        f.surface.knots_u = vec![0., 0., 0.5, 1., 1.];
    }
    // Keep every shared world edge in the same literal multispan basis as
    // its surface trace; correspondence must be independently provable.
    for f in &mut model.faces {
        for row in &mut f.surface.control_points {
            let middle = row[0]
                .iter()
                .zip(&row[1])
                .map(|(a, b)| (a + b) * 0.5)
                .collect();
            row.insert(1, middle);
        }
        f.surface.weights = vec![vec![1.; 3]; 3];
        f.surface.knots_v = vec![0., 0., 0.5, 1., 1.];
    }
    for e in &mut model.edges {
        let middle = e.curve.control_points[0]
            .iter()
            .zip(&e.curve.control_points[1])
            .map(|(a, b)| (a + b) * 0.5)
            .collect();
        e.curve.control_points.insert(1, middle);
        e.curve.weights = vec![1.; 3];
        e.curve.knots = vec![0., 0., 0.5, 1., 1.];
    }
    // Give the last face an opposite U clock, including every owned pcurve.
    // World geometry and coedge orientation remain unchanged.
    let face = model.faces.last_mut().unwrap();
    face.surface.control_points.reverse();
    face.surface.weights.reverse();
    let wires = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .collect::<Vec<_>>();
    for wire in wires {
        model.loops[wire].coedges.reverse();
        for c in &mut model.loops[wire].coedges {
            for p in &mut c.pcurve.control_points {
                p[0] = 1. - p[0];
            }
            c.reversed = !c.reversed;
            c.pcurve = c.pcurve.reverse().unwrap();
        }
    }
    let reversed_face = model.faces.len() - 1;
    for face_use in model.shells.iter_mut().flat_map(|s| &mut s.faces) {
        if face_use.face == reversed_face {
            face_use.reversed = !face_use.reversed;
        }
    }
    let before = model.clone();
    let limits = Limits {
        order: 2,
        exact_work: 2000000,
        agreement_work: 1000000,
        regularity_cells: 100000,
    };
    let report = inspect(&model, limits).unwrap();
    assert!(report.parameter_work > 0);
    assert_eq!(report.edge_ids.len(), model.edges.len());
    assert!(report.unresolved_edges.is_empty());
    assert!(report.extraction_complete);
    assert!(!report.g1_certified && !report.g2_certified);
    assert_eq!(
        report.exact_work,
        report.parameter_work + report.proof.exact_work
    );
    assert!(report.exact_work <= limits.exact_work);
    let short = inspect(
        &model,
        Limits {
            exact_work: report.parameter_work - 1,
            ..limits
        },
    )
    .unwrap();
    assert!(!short.extraction_complete && !short.g1_certified && !short.g2_certified);
    assert!(short.exact_work <= report.parameter_work - 1);
    assert_eq!(model, before);
}

#[test]
fn nonuniform_asymmetric_and_periodic_bases_reverse_without_refitting() {
    let mut a = strip(1.);
    a.knots_u[4] = -0.5;
    let saved = a.clone();
    let (ok, work) = reverse_along(&mut a, "vMin", 1000000).unwrap();
    assert!(ok && work > 0);
    assert_eq!(a.knots_u[4], 0.5);
    let mut short = saved.clone();
    assert!(!reverse_along(&mut short, "vMin", work - 1).unwrap().0);
    assert_eq!(short, saved);
    assert!(reverse_along(&mut a, "vMin", 1000000).unwrap().0);
    assert_eq!(a, saved);
    let xy = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
    let mut s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: (0..9).map(|i| i as f64).collect(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..6)
            .map(|i| {
                (0..2)
                    .map(|z| vec![xy[i % 4][0], xy[i % 4][1], z as f64])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 6],
        periodic_u: true,
        periodic_v: false,
    };
    s.validate().unwrap();
    let saved = s.clone();
    assert!(reverse_along(&mut s, "vMin", 1000000).unwrap().0);
    assert!(s.periodic_u);
    s.validate().unwrap();
    assert!(reverse_along(&mut s, "vMin", 1000000).unwrap().0);
    assert_eq!(s, saved);
}
