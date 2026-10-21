use super::*;
fn mesh(cells: Vec<Poly>) -> SourceMesh {
    let mut w = Work::default();
    let all = cells.iter().flat_map(Poly::vertices).collect::<Vec<_>>();
    let planes = cells
        .iter()
        .flat_map(|c| c.faces.iter().map(|f| normalized(&plane(f), false)))
        .collect::<BTreeSet<_>>();
    let arranged = partition(
        vec![bounding_box(&all).unwrap()],
        &planes.into_iter().collect::<Vec<_>>(),
        &mut w,
    )
    .unwrap();
    let material = arranged
        .into_iter()
        .filter(|c| {
            let p = centroid(&c.vertices());
            cells.iter().any(|s| s.contains(&p))
        })
        .collect::<Vec<_>>();
    let (p, t) = boundary(&material, &mut w).unwrap();
    SourceMesh {
        positions: p.iter().flatten().map(|x| x.to_f64().unwrap()).collect(),
        indices: t.iter().flatten().copied().collect(),
    }
}
pub(super) fn cube(lo: [i64; 3], hi: [i64; 3]) -> SourceMesh {
    mesh(vec![
        bounding_box(&[lo.map(integer), hi.map(integer)]).unwrap(),
    ])
}
fn assert_sum(a: &SourceMesh, b: &SourceMesh, volume: f64) -> Verified {
    let r = certify(a, b).unwrap();
    assert!(
        (r.volume_mm3 - volume).abs() < 1e-9,
        "{} vs {volume}",
        r.volume_mm3
    );
    assert_eq!(
        verify(a, b, &r.certificate).unwrap().certificate,
        r.certificate
    );
    r
}
#[test]
fn convex_boxes_and_source_binding() {
    let a = cube([0, 0, 0], [2, 3, 4]);
    let b = cube([-1, -1, -1], [1, 1, 1]);
    let r = assert_sum(&a, &b, 120.);
    assert_eq!(r.maximum_coordinate_rounding_mm, 0.);
    let mut bad = r.certificate.clone();
    bad.vertices[0][0][0] = "17".into();
    assert!(verify(&a, &b, &bad).is_err());
    let mut changed = a.clone();
    changed.positions[0] += 0.5;
    assert!(verify(&changed, &b, &r.certificate).is_err());
    bad = r.certificate.clone();
    bad.source_cells[0] += 1;
    assert!(verify(&a, &b, &bad).is_err());
}
#[test]
fn nonconvex_three_dimensional_material() {
    let a = mesh(vec![
        bounding_box(&[[0, 0, 0].map(integer), [4, 1, 2].map(integer)]).unwrap(),
        bounding_box(&[[0, 1, 0].map(integer), [1, 4, 2].map(integer)]).unwrap(),
    ]);
    let b = cube([-1, -1, -1], [1, 1, 1]);
    let r = assert_sum(&a, &b, 108.);
    assert!(r.certificate.source_cells[0] > 1)
}
#[test]
fn cavity_is_preserved_by_global_sum() {
    let mut a = cube([0, 0, 0], [10, 10, 10]);
    let mut hole = cube([3, 3, 3], [7, 7, 7]);
    for t in hole.indices.chunks_exact_mut(3) {
        t.swap(1, 2)
    }
    let offset = (a.positions.len() / 3) as u32;
    a.positions.extend(hole.positions);
    a.indices.extend(hole.indices.iter().map(|i| i + offset));
    assert_sum(&a, &cube([-1, -1, -1], [1, 1, 1]), 1720.);
}
#[test]
fn disconnected_components_and_empty_positive_material() {
    let mut a = cube([0, 0, 0], [1, 1, 1]);
    let b = cube([5, 0, 0], [6, 1, 1]);
    let offset = (a.positions.len() / 3) as u32;
    a.positions.extend(b.positions);
    a.indices.extend(b.indices.iter().map(|i| i + offset));
    assert_sum(&a, &cube([0, 0, 0], [1, 1, 1]), 16.);
    let mut reversed = cube([0, 0, 0], [1, 1, 1]);
    for t in reversed.indices.chunks_exact_mut(3) {
        t.swap(1, 2)
    }
    let r = assert_sum(&reversed, &cube([0, 0, 0], [1, 1, 1]), 0.);
    assert!(r.indices.is_empty());
}
#[test]
fn non_axis_aligned_hull() {
    let a = cube([0, 0, 0], [2, 2, 2]);
    let mut b = cube([0, 0, 0], [1, 1, 1]);
    for p in b.positions.chunks_exact_mut(3) {
        p[0] += p[1];
    }
    let r = certify(&a, &b).unwrap();
    assert!((r.volume_mm3 - 33.).abs() < 1e-9);
    verify(&a, &b, &r.certificate).unwrap();
}
#[test]
fn malformed_open_orientation_and_nonfinite_refuse() {
    let b = cube([0, 0, 0], [1, 1, 1]);
    let mut a = b.clone();
    a.indices.truncate(a.indices.len() - 3);
    assert!(certify(&a, &b).is_err());
    let mut a = b.clone();
    a.indices.swap(1, 2);
    assert!(certify(&a, &b).is_err());
    let mut a = b.clone();
    a.positions[0] = f64::NAN;
    assert!(certify(&a, &b).is_err());
}

#[test]
fn rational_display_displacement_is_outward_bounded() {
    let a = SourceMesh {
        positions: vec![0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.],
        indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
    };
    let b = cube([0, 0, 0], [1, 1, 1]);
    let r = assert_sum(&a, &b, 17. / 3.);
    assert_eq!(r.exact_volume_mm3, ["17", "3"]);
    assert!(r.maximum_coordinate_rounding_mm > 0.);
    let bound = Q::from_float(r.maximum_coordinate_rounding_mm).unwrap();
    for (p, x) in r.certificate.vertices.iter().flatten().zip(&r.positions) {
        let q = Q::new(p[0].parse().unwrap(), p[1].parse().unwrap());
        assert!((q - Q::from_float(*x).unwrap()).abs() <= bound);
    }
}
