use super::*;
fn uv() -> Net {
    (0..=2)
        .map(|i| {
            (0..=2 - i)
                .map(|j| {
                    [
                        if i == 2 { Q::one() } else { Q::zero() },
                        Q::new(j.into(), 2.into())
                            + if i == 1 && j == 1 {
                                Q::new(1.into(), 2.into())
                            } else {
                                Q::zero()
                            },
                        Q::zero(),
                        Q::one(),
                    ]
                })
                .collect()
        })
        .collect()
}
fn graph(rational: bool) -> Net {
    (0..=2)
        .map(|i| {
            (0..=2 - i)
                .map(|j| {
                    let x = Q::new(i.into(), 2.into());
                    let y = Q::new(j.into(), 2.into());
                    if rational {
                        [
                            x + if i == 1 && j == 1 {
                                Q::new(1.into(), 2.into())
                            } else {
                                Q::zero()
                            },
                            y.clone() + if j == 2 { Q::one() } else { Q::zero() },
                            if i == 2 { Q::one() } else { Q::zero() },
                            Q::one() + y,
                        ]
                    } else {
                        [x, y, if i == 2 { Q::one() } else { Q::zero() }, Q::one()]
                    }
                })
                .collect()
        })
        .collect()
}
fn patch(h: Net) -> Patch {
    let mut work = Work::default();
    Patch {
        ids: [0, 1, 2],
        derivatives: bernstein::derivatives(&h, &mut work).unwrap(),
        motion: bernstein::homotopy(&h, &mut work).unwrap(),
        h,
    }
}
fn value(h: &Net, u: Q, v: Q) -> P {
    let b = [Q::one() - &u - &v, u, v];
    let q = bernstein::restrict(h, [b.clone(), b.clone(), b], &mut Work::default()).unwrap();
    sb::point(&q[0][0])
}
#[test]
fn polynomial_and_rational_nonplanar_graphs_eliminate_algebraic_clocks_exactly() {
    for rational in [false, true] {
        let h = bernstein::compose_triangle(&graph(rational), &uv(), &mut Work::default()).unwrap();
        let p = patch(h);
        assert!(planar::canonical(&p, &mut Work::default())
            .unwrap()
            .is_none());
        let canonical = canonical(&p, &mut Work::default()).unwrap().unwrap();
        assert_eq!(bernstein::corners(&canonical), bernstein::corners(&p.h));
        let point = value(
            &canonical,
            Q::new(1.into(), 3.into()),
            Q::new(1.into(), 4.into()),
        );
        assert_eq!(
            point,
            [
                Q::new(1.into(), 3.into()),
                Q::new(1.into(), 4.into()),
                if rational {
                    Q::new(4.into(), 45.into())
                } else {
                    Q::new(1.into(), 9.into())
                }
            ]
        );
        let edge = bernstein::edges(&p.h)[0].clone();
        let target = [
            Q::new(1.into(), 2.into()),
            Q::zero(),
            Q::new(1.into(), 4.into()),
        ];
        assert!(
            nonlinear_cancel::parameter(&edge, &target, &mut Work::default())
                .unwrap()
                .is_none()
        );
        assert!(algebraic::contains_point(&edge, &target, &mut Work::default()).unwrap());
        assert!(nonlinear_cancel::parameter(
            &bernstein::edges(&canonical)[0],
            &target,
            &mut Work::default()
        )
        .unwrap()
        .is_some());
    }
}
#[test]
fn a_curved_nonplanar_neighbor_receives_the_known_vertex_without_rounding() {
    let mut vertices = BTreeMap::new();
    let mut patches = Vec::new();
    let mut work = Work::default();
    let g = graph(false);
    let bary = [[1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
    let half = Q::new(1.into(), 2.into());
    let mid = [half.clone(), half, Q::zero()];
    for tri in [
        [bary[0].clone(), mid.clone(), bary[2].clone()],
        [mid, bary[1].clone(), bary[2].clone()],
    ] {
        let h = bernstein::restrict(&g, tri, &mut work).unwrap();
        append_patch(h, false, &mut vertices, &mut patches, &mut work).unwrap();
    }
    let h = bernstein::compose_triangle(&g, &uv(), &mut work).unwrap();
    let h = h
        .into_iter()
        .map(|r| {
            r.into_iter()
                .map(|mut p| {
                    p[1] = -p[1].clone();
                    p
                })
                .collect()
        })
        .collect();
    append_patch(h, true, &mut vertices, &mut patches, &mut work).unwrap();
    let mut positions = vec![[Q::zero(), Q::zero(), Q::zero()]; vertices.len()];
    for (p, i) in vertices {
        positions[i as usize] = p;
    }
    // The surface is deliberately open: synchronize must propagate the curved
    // seam, then continue to refuse the four remaining exterior boundaries.
    assert!(synchronize(&positions, &mut patches, &mut work).is_err());
    assert_eq!(patches.len(), 4);
    assert_eq!(open(&patches).len(), 4);
}
#[test]
fn a_source_perturbation_is_not_accepted_as_the_old_graph() {
    let mut h = bernstein::compose_triangle(&graph(false), &uv(), &mut Work::default()).unwrap();
    h[1][1][2] += Q::new(1.into(), 1024.into());
    assert!(!canonical(&patch(h), &mut Work::default()).is_ok_and(|v| v.is_some()));
}
