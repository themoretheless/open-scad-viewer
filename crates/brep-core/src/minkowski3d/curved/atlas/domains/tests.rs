use super::*;
fn q(n: i64, d: i64) -> Q {
    Q::new(n.into(), d.into())
}
fn circle() -> DomainPolynomial {
    algebra::encode(&vec![
        vec![q(7, 64), q(-1, 2), Q::one()],
        vec![q(-1, 2)],
        vec![Q::one()],
    ])
}
#[test]
fn closed_curves_crossings_and_coincident_cuts_assemble_complete_parameter_disks() {
    let examples = vec![
        vec![circle()],
        vec![
            algebra::encode(&vec![vec![q(-1, 2), Q::one()]]),
            algebra::encode(&vec![vec![q(-1, 2)], vec![Q::one()]]),
        ],
        vec![Vec::new(), algebra::encode(&vec![vec![Q::one()]])],
    ];
    for cuts in examples {
        let p = partition(&cuts).unwrap();
        assert_eq!(p.vertices.len() + p.faces.len(), p.edges.len() + 1);
        assert!(!p.faces.is_empty());
        if cuts == vec![circle()] {
            assert!(p.faces.iter().any(|f| f.signs == vec![-1]));
            assert!(p.faces.iter().any(|f| f.signs == vec![1]));
            // The region enclosed by a curved loop has a legitimate two-arc
            // boundary (a lens), not an invented straight triangle.
            assert!(p.faces.iter().any(|f| f.boundary.len() == 2));
        }
    }
}
#[test]
fn full_cube_atlas_binds_maps_local_cells_and_all_parent_transitions() {
    let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let cuts = vec![vec![circle()]];
    let c = certify_algebraic_domain_atlas(&m, 0, &cuts).unwrap();
    assert_eq!(c.maps.len(), 12);
    assert_eq!(c.partitions.len(), 12);
    assert_eq!(c.seams.len(), 18);
    assert!(c.seams.iter().all(|s| s.pointwise));
    verify_algebraic_domain_atlas(&m, 0, &cuts, &c).unwrap();
    let mut bad = c.clone();
    bad.seams.pop();
    assert!(verify_algebraic_domain_atlas(&m, 0, &cuts, &bad).is_err());
    let mut bad = c.clone();
    bad.partitions[0].faces.pop();
    assert!(verify_algebraic_domain_atlas(&m, 0, &cuts, &bad).is_err());
    let mut bad = c.clone();
    bad.partitions[0].faces[0].boundary[0] *= -1;
    assert!(verify_algebraic_domain_atlas(&m, 0, &cuts, &bad).is_err());
    let mut bad = c.clone();
    bad.maps[0][0][0][0] = roots::ratio(&q(17, 16));
    assert!(verify_algebraic_domain_atlas(&m, 0, &cuts, &bad).is_err());
    let changed = vec![vec![algebra::encode(&vec![vec![q(-1, 2), Q::one()]])]];
    assert!(verify_algebraic_domain_atlas(&m, 0, &changed, &c).is_err());
}
#[test]
fn a_closed_nonplanar_cone_keeps_its_source_maps_under_implicit_refinement() {
    let m = crate::frustum(1., 0., 2.).unwrap();
    let cuts = vec![vec![circle()]];
    let c = certify_algebraic_domain_atlas(&m, 0, &cuts).unwrap();
    verify_algebraic_domain_atlas(&m, 0, &cuts, &c).unwrap();
    let mut nonplanar = false;
    for map in &c.maps {
        let h = map
            .iter()
            .map(|r| {
                r.iter()
                    .map(|p| std::array::from_fn(|i| roots::decode(&p[i]).unwrap()))
                    .collect()
            })
            .collect::<Net>();
        let p = bernstein::corners(&h);
        let n = cross(&sub(&p[1], &p[0]), &sub(&p[2], &p[0]));
        if h.iter()
            .flatten()
            .any(|h| !dot(&n, &std::array::from_fn(|i| &h[i] - &p[0][i] * &h[3])).is_zero())
        {
            nonplanar = true;
        }
    }
    assert!(nonplanar, "implicit atlas must retain the curved 3D maps");
    assert_eq!(c.maps.len(), c.base.triangles.len());
}
#[test]
fn noncanonical_polynomials_and_unbounded_degree_refuse() {
    assert!(partition(&vec![vec![vec![roots::ratio(&Q::zero())]]]).is_err());
    let mut large = vec![Vec::new(); 10];
    large[9] = vec![roots::ratio(&Q::one())];
    assert!(partition(&vec![large]).is_err());
    let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    assert!(certify_algebraic_domain_atlas(&m, 0, &[]).is_err());
}
