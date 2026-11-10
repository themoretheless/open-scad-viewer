use super::*;
fn q(a: i64, b: i64) -> Q {
    Q::new(a.into(), b.into())
}
fn surface(p: &Bi, degree: usize) -> Surface {
    fn choose(n: usize, k: usize) -> Q {
        (0..k).fold(Q::one(), |q, i| {
            q * integer((n - i) as i64) / integer((i + 1) as i64)
        })
    }
    let mut control = vec![vec![vec![0.; 3]; degree + 1]; degree + 1];
    for i in 0..=degree {
        for j in 0..=degree {
            let mut z = Q::zero();
            for (b, row) in p.iter().enumerate() {
                for (a, c) in row.iter().enumerate() {
                    if a <= i && b <= j {
                        z +=
                            c * choose(i, a) / choose(degree, a) * choose(j, b) / choose(degree, b);
                    }
                }
            }
            let zf = z.to_f64().unwrap();
            assert_eq!(
                Q::from_float(zf).unwrap(),
                z,
                "fixture controls must be exact"
            );
            control[i][j] = vec![i as f64 / degree as f64, j as f64 / degree as f64, zf];
        }
    }
    let mut knots = vec![0.; degree + 1];
    knots.extend(vec![1.; degree + 1]);
    Surface {
        degree_u: degree,
        degree_v: degree,
        knots_u: knots.clone(),
        knots_v: knots,
        control_points: control,
        weights: vec![vec![1.; degree + 1]; degree + 1],
        periodic_u: false,
        periodic_v: false,
    }
}
fn radius_squared() -> Bi {
    vec![
        vec![q(1, 8), q(-1, 2), Q::one()],
        vec![q(-1, 2)],
        vec![Q::one()],
    ]
}
fn circle() -> (Surface, Surface, DomainPolynomial) {
    // Independently curved lower/upper caps. Their quadratic height difference
    // cannot be the difference of two planes under any common height shear.
    let lower = vec![vec![Q::zero(), Q::zero(), q(1, 8)], vec![], vec![q(1, 4)]];
    let difference = sum(&vec![vec![q(1, 64)]], &scale(&radius_squared(), &-Q::one())).unwrap();
    let upper = sum(&lower, &difference).unwrap();
    (
        surface(&upper, 2),
        surface(&lower, 2),
        vec![vec![roots::ratio(&integer(4))]],
    )
}
#[test]
fn independent_curved_caps_keep_algebraic_seams_and_exact_queries() {
    let (upper, lower, witness) = circle();
    let b = certify_graph_region_brep(&upper, &lower, &witness, None).unwrap();
    assert_eq!(b.certificate.components.len(), 1);
    assert_eq!(b.certificate.faces.len(), 2);
    assert!(!b.certificate.seams.is_empty());
    assert!(b
        .certificate
        .seams
        .iter()
        .all(|e| e.faces == [0, 1] && e.oriented_uses[0] == -e.oriented_uses[1]));
    let u = roots::ratio(&q(1, 4));
    let v = u.clone();
    let a = b.evaluate_rational(0, &u, &v).unwrap();
    let c = b.evaluate_rational(1, &u, &v).unwrap();
    assert_eq!(
        roots::decode(&a[2]).unwrap() - roots::decode(&c[2]).unwrap(),
        q(1, 64)
    );
    let seam = b.evaluate_rational(0, &u, &roots::ratio(&q(1, 8))).unwrap();
    assert_eq!(
        seam,
        b.evaluate_rational(1, &u, &roots::ratio(&q(1, 8))).unwrap()
    );
    assert!(b
        .evaluate_rational(0, &roots::ratio(&q(1, 16)), &roots::ratio(&q(1, 16)))
        .is_err());
    verify_graph_region_brep(&upper, &lower, b.certificate()).unwrap();
    let mut bad = b.certificate.clone();
    bad.seams[0].oriented_uses[1] *= -1;
    assert!(verify_graph_region_brep(&upper, &lower, &bad).is_err());
    let mut bad = b.certificate.clone();
    bad.partition.arrangement.slabs[0].roots.pop();
    assert!(verify_graph_region_brep(&upper, &lower, &bad).is_err());
    let mut changed = upper.clone();
    changed.control_points[1][1][2] += 0.125;
    assert!(verify_graph_region_brep(&changed, &lower, b.certificate()).is_err());
}
#[test]
fn annular_algebraic_region_has_two_boundary_cycles_per_cap() {
    let mut work = Work::default();
    let s = radius_squared();
    let a = q(1, 256);
    let b = q(1, 64);
    let outer = sum(&vec![vec![b.clone()]], &scale(&s, &-Q::one())).unwrap();
    let inner = sum(&s, &vec![vec![-a.clone()]]).unwrap();
    let equation = scale(&product(&outer, &inner, &mut work).unwrap(), &integer(9));
    let lower = vec![vec![Q::zero(), Q::zero(), integer(3)]];
    let upper = sum(&lower, &equation).unwrap();
    let delta = &b - &a;
    let multiplier = scale(
        &sum(&scale(&s, &integer(16)), &vec![vec![-&delta * &delta]]).unwrap(),
        &integer(9),
    );
    let (upper, lower) = (surface(&upper, 4), surface(&lower, 4));
    let factors = [encode(&outer), encode(&scale(&inner, &integer(9)))];
    let result =
        certify_graph_region_brep_factored(&upper, &lower, &encode(&multiplier), &factors, None)
            .unwrap();
    assert_eq!(result.certificate.components.len(), 1);
    assert_eq!(result.certificate.faces.len(), 2);
    assert!(result
        .certificate
        .faces
        .iter()
        .all(|f| f.boundary_cycles.len() == 2));
    assert!(result
        .evaluate_rational(0, &roots::ratio(&q(1, 4)), &roots::ratio(&q(1, 4)))
        .is_err());
    result
        .evaluate_rational(0, &roots::ratio(&q(11, 32)), &roots::ratio(&q(1, 4)))
        .unwrap();
    let selected = result.certificate.selected.clone();
    assert!(selected.len() > 1);
    assert!(certify_graph_region_brep_factored(
        &upper,
        &lower,
        &encode(&multiplier),
        &factors,
        Some(&selected[..1])
    )
    .is_err());
    let mut bad_factors = factors.to_vec();
    bad_factors.pop();
    assert!(certify_graph_region_brep_factored(
        &upper,
        &lower,
        &encode(&multiplier),
        &bad_factors,
        None
    )
    .is_err());
    verify_graph_region_brep(&upper, &lower, result.certificate()).unwrap();
}
#[test]
fn unsupported_open_caps_singular_seams_and_projection_mismatch_refuse() {
    let (mut upper, lower, witness) = circle();
    assert!(certify_graph_region_brep(&upper, &lower, &vec![], None).is_err());
    for row in &mut upper.control_points {
        for p in row {
            p[2] += 1.;
        }
    }
    assert!(certify_graph_region_brep(&upper, &lower, &witness, None).is_err());
    let (mut upper, lower, witness) = circle();
    upper.control_points[1][1][0] += 0.125;
    assert!(certify_graph_region_brep(&upper, &lower, &witness, None).is_err());
    assert!(certify_graph_region_brep(&lower, &lower, &witness, None).is_err());
}
#[test]
fn negative_projection_orientation_and_empty_selection_are_explicit() {
    let (mut upper, mut lower, witness) = circle();
    for s in [&mut upper, &mut lower] {
        for row in &mut s.control_points {
            for p in row {
                p[0] = -p[0];
            }
        }
    }
    let b = certify_graph_region_brep(&upper, &lower, &witness, None).unwrap();
    assert!(b.certificate.faces[0].reversed);
    assert!(!b.certificate.faces[1].reversed);
    let empty = certify_graph_region_brep(&upper, &lower, &witness, Some(&[])).unwrap();
    assert!(empty.certificate.faces.is_empty() && empty.certificate.seams.is_empty());
    verify_graph_region_brep(&upper, &lower, empty.certificate()).unwrap();
}

#[test]
fn independently_transposed_source_parameters_are_aligned_exactly() {
    let (upper, lower, witness) = circle();
    let mut transposed = lower.clone();
    for i in 0..3 {
        for j in 0..3 {
            transposed.control_points[i][j] = lower.control_points[j][i].clone();
        }
    }
    let result = certify_graph_region_brep(&upper, &transposed, &witness, None).unwrap();
    assert!(!result.certificate.faces[1].reversed);
    assert_eq!(
        result.certificate.normalized_parameter_maps[1][0],
        [
            roots::ratio(&Q::zero()),
            roots::ratio(&Q::zero()),
            roots::ratio(&Q::one())
        ]
    );
    assert_eq!(
        result.certificate.normalized_parameter_maps[1][1],
        [
            roots::ratio(&Q::zero()),
            roots::ratio(&Q::one()),
            roots::ratio(&Q::zero())
        ]
    );
    let point = result
        .evaluate_rational(1, &roots::ratio(&q(1, 4)), &roots::ratio(&q(1, 4)))
        .unwrap();
    assert_eq!(roots::decode(&point[2]).unwrap(), q(3, 128));
    verify_graph_region_brep(&upper, &transposed, result.certificate()).unwrap();
}

#[test]
fn quartic_intersection_keeps_irrational_critical_parameters() {
    let mut work = Work::default();
    let x = vec![vec![q(-1, 4), Q::one()]];
    let y = vec![vec![q(-1, 4)], vec![Q::one()]];
    let x2 = product(&x, &x, &mut work).unwrap();
    let y2 = product(&y, &y, &mut work).unwrap();
    let fourth = sum(
        &product(&x2, &x2, &mut work).unwrap(),
        &product(&y2, &y2, &mut work).unwrap(),
    )
    .unwrap();
    let equation = scale(
        &sum(&vec![vec![q(1, 8192)]], &scale(&fourth, &-Q::one())).unwrap(),
        &integer(9),
    );
    let lower = vec![vec![Q::zero(), Q::zero(), integer(3)]];
    let upper = sum(&lower, &equation).unwrap();
    let (upper, lower) = (surface(&upper, 4), surface(&lower, 4));
    let multiplier = encode(&vec![vec![q(9, 128)]]);
    let result =
        certify_graph_region_brep_factored(&upper, &lower, &multiplier, &[encode(&equation)], None)
            .unwrap();
    assert_eq!(result.certificate.components.len(), 1);
    assert!(result
        .certificate
        .partition
        .arrangement
        .cuts
        .iter()
        .any(|cut| cut.parameter[0] != cut.parameter[1]));
    let u = roots::ratio(&q(1, 4));
    let a = result.evaluate_rational(0, &u, &u).unwrap();
    let b = result.evaluate_rational(1, &u, &u).unwrap();
    assert_eq!(
        roots::decode(&a[2]).unwrap() - roots::decode(&b[2]).unwrap(),
        q(9, 8192)
    );
    verify_graph_region_brep(&upper, &lower, result.certificate()).unwrap();
}
