//! Propagate exact spatial edge vertices to independently partitioned charts.
//! Restriction changes neither the source image nor its boundary curves.
use super::*;
mod graph;
mod planar;
fn open(patches: &[Patch]) -> Vec<(usize, usize, Vec<sb::H>)> {
    let mut edges = BTreeMap::<(u32, u32), Vec<(usize, usize, Vec<sb::H>)>>::new();
    for (i, p) in patches.iter().enumerate() {
        for (k, h) in bernstein::edges(&p.h).into_iter().enumerate() {
            let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((i, k, h));
        }
    }
    edges
        .into_values()
        .filter(|e| e.len() == 1)
        .flatten()
        .collect()
}
pub(super) fn synchronize(
    positions: &[P],
    patches: &mut Vec<Patch>,
    work: &mut Work,
) -> Result<()> {
    let mut vertices = positions
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, p)| (p, i as u32))
        .collect::<BTreeMap<_, _>>();
    let bary = [[1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
    for _ in 0..1024 {
        let edges = open(patches);
        if edges.is_empty() {
            return Ok(());
        }
        work.charge(patches.len() * 3)?;
        let candidates = edges
            .iter()
            .flat_map(|(i, k, _)| [patches[*i].ids[*k], patches[*i].ids[(*k + 1) % 3]])
            .collect::<BTreeSet<_>>();
        let mut split = None;
        let mut reparameterize = None;
        'search: for (i, k, h) in edges {
            let hull = h
                .iter()
                .filter(|p| p[3] > Q::zero())
                .map(sb::point)
                .collect::<Vec<_>>();
            let bounds = std::array::from_fn::<_, 3, _>(|axis| {
                [
                    hull.iter().map(|p| p[axis].clone()).min().unwrap(),
                    hull.iter().map(|p| p[axis].clone()).max().unwrap(),
                ]
            });
            for id in &candidates {
                work.charge(1)?;
                if patches[i].ids.contains(id) {
                    continue;
                }
                let point = &positions[*id as usize];
                if (0..3).any(|a| point[a] < bounds[a][0] || point[a] > bounds[a][1]) {
                    continue;
                }
                if let Some(t) = nonlinear_cancel::parameter(&h, point, work)? {
                    split = Some((i, k, t));
                    break 'search;
                }
                let a = sb::point(&h[0]);
                let b = sb::point(h.last().unwrap());
                let direction = sub(&b, &a);
                let offset = sub(point, &a);
                let distance = dot(&offset, &direction);
                if zero(&cross(&direction, &offset))
                    && distance > Q::zero()
                    && distance < dot(&direction, &direction)
                {
                    if let Some(net) = planar::canonical(&patches[i], work)? {
                        reparameterize = Some((i, net));
                        break 'search;
                    }
                }
                if algebraic::contains_point(&h, point, work)? {
                    if let Some(net) = graph::canonical(&patches[i], work)? {
                        reparameterize = Some((i, net));
                        break 'search;
                    }
                }
            }
        }
        if let Some((i, net)) = reparameterize {
            let mut canonical = Vec::new();
            append_patch(net, false, &mut vertices, &mut canonical, work)?;
            if vertices.len() != positions.len() {
                return Err(fail("source image normalization changed source vertices"));
            }
            patches[i] = canonical.remove(0);
            continue;
        }
        let Some((i, k, t)) = split else {
            return Err(fail(
                "atlas partition has unmatched edges without a proved rational split parameter",
            ));
        };
        let mid =
            std::array::from_fn(|a| (Q::one() - &t) * &bary[k][a] + &t * &bary[(k + 1) % 3][a]);
        let old = &patches[i].h;
        let left = bernstein::restrict(
            old,
            [bary[k].clone(), mid.clone(), bary[(k + 2) % 3].clone()],
            work,
        )?;
        let right = bernstein::restrict(
            old,
            [mid, bary[(k + 1) % 3].clone(), bary[(k + 2) % 3].clone()],
            work,
        )?;
        let mut children = Vec::new();
        append_patch(left, false, &mut vertices, &mut children, work)?;
        append_patch(right, false, &mut vertices, &mut children, work)?;
        if vertices.len() != positions.len() {
            return Err(fail(
                "seam restriction introduced an unbound spatial vertex",
            ));
        }
        patches[i] = children.remove(0);
        patches.push(children.remove(0));
        if patches.len() > 1024 {
            return Err(fail("conforming atlas partition budget exceeded"));
        }
    }
    Err(fail("conforming atlas partition iteration budget exceeded"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_curved_tetrahedron_becomes_conforming_without_new_geometry() {
        let points = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
        let faces = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
        let mut vertices = BTreeMap::new();
        let mut patches = Vec::new();
        let mut work = Work::default();
        let bary = [[1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
        let half = Q::new(1.into(), 2.into());
        let mid = [half.clone(), half, Q::zero()];
        for (index, face) in faces.into_iter().enumerate() {
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
                            if face
                                .iter()
                                .position(|v| *v == 0)
                                .is_some_and(|k| counts[k] == 1)
                                && face
                                    .iter()
                                    .position(|v| *v == 1)
                                    .is_some_and(|k| counts[k] == 1)
                            {
                                h[2] += Q::new(1.into(), 8.into());
                            }
                            h
                        })
                        .collect()
                })
                .collect::<Net>();
            if index == 1 {
                for tri in [
                    [bary[0].clone(), mid.clone(), bary[2].clone()],
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
        assert_eq!(patches.len(), 5);
        synchronize(&positions, &mut patches, &mut work).unwrap();
        assert_eq!(patches.len(), 6);
        assert!(open(&patches).is_empty());
        let mut edges = BTreeMap::<(u32, u32), Vec<Vec<sb::H>>>::new();
        for p in &patches {
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
    }
    #[test]
    fn a_split_curved_edge_propagates_to_its_neighbor() {
        let mut vertices = BTreeMap::new();
        let mut patches = Vec::new();
        let mut work = Work::default();
        let point = |x: i64, y: i64| [integer(x), integer(y), Q::zero(), Q::one()];
        // Two opposite triangles share the quadratic edge (0,0)->(1,0).
        let a = vec![
            vec![
                point(0, 0),
                [Q::zero(), Q::new(1.into(), 2.into()), Q::zero(), Q::one()],
                point(0, 1),
            ],
            vec![
                [
                    Q::new(1.into(), 2.into()),
                    Q::new(1.into(), 8.into()),
                    Q::zero(),
                    Q::one(),
                ],
                [
                    Q::new(1.into(), 2.into()),
                    Q::new(1.into(), 2.into()),
                    Q::zero(),
                    Q::one(),
                ],
            ],
            vec![point(1, 0)],
        ];
        let b = a
            .iter()
            .map(|r| {
                r.iter()
                    .map(|p| [p[0].clone(), -p[1].clone(), p[2].clone(), p[3].clone()])
                    .collect()
            })
            .collect::<Net>();
        // Use identical curved edge; reflect the other controls only.
        let mut b = b;
        b[1][0] = a[1][0].clone();
        let half = Q::new(1.into(), 2.into());
        let bary = [
            [Q::one(), Q::zero(), Q::zero()],
            [Q::zero(), Q::one(), Q::zero()],
            [Q::zero(), Q::zero(), Q::one()],
        ];
        let mid = [half.clone(), half, Q::zero()];
        let left = bernstein::restrict(
            &a,
            [bary[0].clone(), mid.clone(), bary[2].clone()],
            &mut work,
        )
        .unwrap();
        let right =
            bernstein::restrict(&a, [mid, bary[1].clone(), bary[2].clone()], &mut work).unwrap();
        append_patch(left, false, &mut vertices, &mut patches, &mut work).unwrap();
        append_patch(right, false, &mut vertices, &mut patches, &mut work).unwrap();
        append_patch(b, true, &mut vertices, &mut patches, &mut work).unwrap();
        let mut positions = vec![[Q::zero(), Q::zero(), Q::zero()]; vertices.len()];
        for (p, i) in vertices {
            positions[i as usize] = p;
        }
        // This open pair must still refuse overall, but its curved seam must be
        // propagated before the remaining genuinely open boundary is reported.
        assert!(synchronize(&positions, &mut patches, &mut work).is_err());
        assert_eq!(patches.len(), 4);
        assert_eq!(open(&patches).len(), 4);
    }
}

pub(super) fn planar_chart(h: &Net, work: &mut Work) -> Result<Option<Net>> {
    let p = Patch {
        ids: [0, 1, 2],
        h: h.clone(),
        derivatives: bernstein::derivatives(h, work)?,
        motion: bernstein::homotopy(h, work)?,
    };
    planar::canonical(&p, work)
}
