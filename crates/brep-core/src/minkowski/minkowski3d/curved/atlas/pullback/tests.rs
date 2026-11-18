use super::*;
fn q(n: i64, d: i64) -> ExactRatio {
    roots::ratio(&Q::new(n.into(), d.into()))
}
fn surface(n: usize, m: usize, z: impl Fn(usize, usize) -> f64) -> crate::Surface {
    let knots = |n| [vec![0.; n + 1], vec![1.; n + 1]].concat();
    crate::Surface {
        degree_u: n,
        degree_v: m,
        knots_u: knots(n),
        knots_v: knots(m),
        control_points: (0..=n)
            .map(|i| {
                (0..=m)
                    .map(|j| vec![i as f64 / n as f64, j as f64 / m as f64, z(i, j)])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; m + 1]; n + 1],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn a_closed_curved_level_component_keeps_both_branches_and_source_sides() {
    let a = [1. / 16., -3. / 16., 9. / 16.];
    let s = surface(2, 2, |i, j| a[i] + a[j] - 1. / 64.);
    let cert = certify_algebraic_surface_pullback(&s, 2, &q(0, 1)).unwrap();
    verify_algebraic_surface_pullback(&s, 2, &q(0, 1), &cert).unwrap();
    assert!(cert.branches.slabs.iter().any(|s| s.roots.len() == 2));
    assert!(cert.slab_sides.iter().any(|s| s == &vec![[1, -1], [-1, 1]]));
    let mut bad = cert.clone();
    let slab = bad
        .branches
        .slabs
        .iter_mut()
        .find(|s| s.roots.len() == 2)
        .unwrap();
    slab.roots.pop();
    assert!(verify_algebraic_surface_pullback(&s, 2, &q(0, 1), &bad).is_err());
    let mut bad = cert.clone();
    bad.slab_sides.iter_mut().find(|s| !s.is_empty()).unwrap()[0] = [0, 0];
    assert!(verify_algebraic_surface_pullback(&s, 2, &q(0, 1), &bad).is_err());
    let mut changed = s;
    changed.control_points[1][1][2] += 1. / 1024.;
    assert!(verify_algebraic_surface_pullback(&changed, 2, &q(0, 1), &cert).is_err());
}
#[test]
fn nonlinear_tangent_pullback_retains_even_multiplicity_and_irrational_boundary_cut() {
    // z=6(v-u²)², clearing the Bernstein denominators so the authored
    // binary64 tensor is exactly the requested polynomial.
    let s = surface(4, 2, |i, j| {
        (if i == 4 { 6. } else { 0. }) - ((i * (i.saturating_sub(1))) / 2 * j) as f64
            + (if j == 2 { 6. } else { 0. })
    });
    let cert = certify_algebraic_surface_pullback(&s, 2, &q(0, 1)).unwrap();
    verify_algebraic_surface_pullback(&s, 2, &q(0, 1), &cert).unwrap();
    assert!(cert.slab_sides.iter().any(|s| !s.is_empty()));
    assert!(cert.slab_sides.iter().flatten().all(|s| *s == [1, 1]));
    assert!(cert
        .branches
        .cuts
        .iter()
        .any(|cut| cut.parameter[0] != cut.parameter[1]));
    assert!(cert
        .branches
        .cuts
        .iter()
        .any(|cut| cut.excluded_nodes.iter().any(|x| *x)));
}
#[test]
fn crossing_components_and_two_dimensional_coincidence_are_explicit() {
    let s = surface(2, 2, |i, j| (i as f64 / 2. - 0.5) * (j as f64 / 2. - 0.5));
    let cert = certify_algebraic_surface_pullback(&s, 2, &q(0, 1)).unwrap();
    verify_algebraic_surface_pullback(&s, 2, &q(0, 1), &cert).unwrap();
    assert!(cert
        .branches
        .cuts
        .iter()
        .any(|cut| cut.vertical && !cut.vertical_arcs.is_empty()));
    let flat = surface(1, 1, |_, _| 0.);
    let cert = certify_algebraic_surface_pullback(&flat, 2, &q(0, 1)).unwrap();
    assert_eq!(cert.branches.dimension, 2);
    assert!(cert.branches.slabs.is_empty());
    verify_algebraic_surface_pullback(&flat, 2, &q(0, 1), &cert).unwrap();
}
