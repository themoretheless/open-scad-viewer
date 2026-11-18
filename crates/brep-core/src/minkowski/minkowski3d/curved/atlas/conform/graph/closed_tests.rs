use super::*;
#[test]
fn closed_nonplanar_atlas_preserves_stars_and_seams_after_an_algebraic_clock() {
    let points = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
    let faces = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
    let bary = [[1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
    let half = Q::new(1.into(), 2.into());
    let mid = [half.clone(), Q::zero(), half.clone()];
    let clock = (0..=2)
        .map(|i| {
            (0..=2 - i)
                .map(|j| {
                    [
                        if i == 2 { Q::one() } else { Q::zero() },
                        Q::new(j.into(), 2.into())
                            + if i == 1 && j == 1 {
                                half.clone()
                            } else {
                                Q::zero()
                            },
                        Q::zero(),
                        Q::one(),
                    ]
                })
                .collect()
        })
        .collect::<Net>();
    let mut vertices = BTreeMap::new();
    let mut patches = Vec::new();
    let mut work = Work::default();
    for (index, face) in faces.into_iter().enumerate() {
        // An ambient polynomial shear z -> z+y²/16 deforms the COMPLETE
        // tetrahedron. Every boundary curve is shared exactly by both faces.
        let linear = vec![
            vec![points[face[0]][1].clone(), points[face[2]][1].clone()],
            vec![points[face[1]][1].clone()],
        ];
        let height = bernstein::product(&linear, &linear, &mut work).unwrap();
        let h = (0..=2)
            .map(|i| {
                (0..=2 - i)
                    .map(|j| {
                        let counts = [2 - i - j, i, j];
                        let mut h = std::array::from_fn(|axis| {
                            if axis == 3 {
                                Q::one()
                            } else {
                                (0..3)
                                    .map(|k| {
                                        integer(counts[k] as i64) * &points[face[k]][axis]
                                            / integer(2)
                                    })
                                    .sum()
                            }
                        });
                        h[2] += &height[i][j] / integer(16);
                        h
                    })
                    .collect()
            })
            .collect::<Net>();
        if index == 0 {
            let h = bernstein::compose_triangle(&h, &clock, &mut work).unwrap();
            append_patch(h, false, &mut vertices, &mut patches, &mut work).unwrap();
        } else if index == 2 {
            for tri in [
                [bary[0].clone(), bary[1].clone(), mid.clone()],
                [mid.clone(), bary[1].clone(), bary[2].clone()],
            ] {
                let child = bernstein::restrict(&h, tri, &mut work).unwrap();
                append_patch(child, false, &mut vertices, &mut patches, &mut work).unwrap();
            }
        } else {
            append_patch(h, false, &mut vertices, &mut patches, &mut work).unwrap();
        }
    }
    let mut positions = vec![[Q::zero(), Q::zero(), Q::zero()]; vertices.len()];
    for (p, i) in vertices {
        positions[i as usize] = p;
    }
    let edge = bernstein::edges(&patches[0].h)[0].clone();
    let target = [Q::zero(), half, Q::new(1.into(), 64.into())];
    assert!(nonlinear_cancel::parameter(&edge, &target, &mut work)
        .unwrap()
        .is_none());
    assert!(algebraic::contains_point(&edge, &target, &mut work).unwrap());
    synchronize(&positions, &mut patches, &mut work).unwrap();
    assert_eq!(patches.len(), 6);
    assert!(open(&patches).is_empty());
    let mut edges = BTreeMap::<(u32, u32), Vec<Vec<sb::H>>>::new();
    let mut stars = vec![Vec::new(); positions.len()];
    for (i, p) in patches.iter().enumerate() {
        for v in p.ids {
            stars[v as usize].push(i);
        }
        for (k, mut h) in bernstein::edges(&p.h).into_iter().enumerate() {
            let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
            if a > b {
                h.reverse();
            }
            edges.entry((a.min(b), a.max(b))).or_default().push(h);
        }
    }
    assert_eq!(positions.len() + patches.len(), edges.len() + 2);
    for pair in edges.values() {
        assert_eq!(pair.len(), 2);
        assert!(sb::same(&pair[0], &pair[1]));
    }
    for (v, incident) in stars.iter().enumerate() {
        star(v as u32, incident, &patches, &positions, &mut work).unwrap();
    }
    for (i, a) in patches.iter().enumerate() {
        for b in &patches[i + 1..] {
            if a.ids.iter().all(|v| !b.ids.contains(v)) {
                assert!(disjoint(&a.motion, &b.motion, 0, &mut work).unwrap());
            }
        }
    }
}
