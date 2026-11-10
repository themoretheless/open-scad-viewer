//! Oriented rational cone chains for arbitrary authored UV wires. Outer and
//! hole chains keep their signs. Their image winding is the trimmed material;
//! no convexification or sampled trim classification replaces the source.
use super::*;
pub(super) type Seams = BTreeMap<(P, P), Vec<(sb::Trace, bool)>>;
pub(super) const SOURCE_SEAM_CODE: &str = "MINKOWSKI_3D_SOURCE_SEAM";
pub(super) fn verify_seams(seams: Seams, work: &mut Work) -> Result<()> {
    for mut group in seams.into_values() {
        while let Some((edge, reverse)) = group.pop() {
            let mut matched = None;
            for (i, (other, r)) in group.iter().enumerate() {
                if *r != reverse && sb::same_trace(&edge, other, work)? {
                    matched = Some(i);
                    break;
                }
            }
            let i=matched.ok_or_else(||Error::new(SOURCE_SEAM_CODE,"Minkowski 3D refused: original source boundary seams lack an exact oriented C0 identity"))?;
            group.remove(i);
        }
    }
    Ok(())
}
/// Check immutable source closure before an approximation-depth search. A
/// genuine source gap cannot be repaired by subdividing its display mesh.
pub(super) fn preflight(m: &Model, work: &mut Work) -> Result<()> {
    m.validate()?;
    if m.faces.len() > 64
        || m.faces.iter().any(|f| {
            !(1..=8).contains(&f.surface.degree_u) || !(1..=8).contains(&f.surface.degree_v)
        })
        || m.faces
            .iter()
            .map(|f| f.surface.control_points.iter().map(Vec::len).sum::<usize>())
            .sum::<usize>()
            > 8192
        || m.loops.iter().map(|l| l.coedges.len()).sum::<usize>() > 2048
    {
        return Err(fail("original source boundary budget exceeded"));
    }
    let uses = m
        .bodies
        .iter()
        .flat_map(|b| std::iter::once(b.outer_shell).chain(b.inner_shells.iter().copied()))
        .flat_map(|i| m.shells[i].faces.iter().cloned())
        .collect::<Vec<_>>();
    let mut values = BTreeSet::from([Q::zero(), Q::one(), Q::new(1.into(), 2.into())]);
    stations(m, &uses, &mut values)?;
    if values.len() > 33 {
        return Err(fail("original source boundary station budget exceeded"));
    }
    let values = values.into_iter().collect::<Vec<_>>();
    let mut seams = Seams::new();
    for u in uses {
        let f = &m.faces[u.face];
        let s = &f.surface;
        sb::continuous(s, work)?;
        if rectangle(m, u.face).is_ok() {
            let a = [
                Q::from_float(s.knots_u[s.degree_u]).unwrap(),
                Q::from_float(s.knots_v[s.degree_v]).unwrap(),
            ];
            let b = [
                Q::from_float(s.knots_u[s.control_points.len()]).unwrap(),
                Q::from_float(s.knots_v[s.control_points[0].len()]).unwrap(),
            ];
            let c = [
                [a[0].clone(), a[1].clone()],
                [b[0].clone(), a[1].clone()],
                [b[0].clone(), b[1].clone()],
                [a[0].clone(), b[1].clone()],
            ];
            for i in 0..4 {
                for t in values.windows(2) {
                    work.charge(1)?;
                    let pc = t
                        .iter()
                        .map(|t| {
                            [
                                &c[i][0] + t * (&c[(i + 1) % 4][0] - &c[i][0]),
                                &c[i][1] + t * (&c[(i + 1) % 4][1] - &c[i][1]),
                                Q::zero(),
                                Q::one(),
                            ]
                        })
                        .collect();
                    insert_seam(
                        sb::Trace::Surface {
                            surface: s.clone(),
                            pcurve: pc,
                        },
                        u.reversed,
                        &mut seams,
                        work,
                    )?;
                }
            }
        } else {
            for l in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for coedge in &m.loops[l].coedges {
                    let c = &coedge.pcurve;
                    let d = c.domain();
                    let a = Q::from_float(d[0]).unwrap();
                    let len = Q::from_float(d[1]).unwrap() - &a;
                    for t in values.windows(2) {
                        work.charge(1)?;
                        let pc = sb::curve(c, &(&a + &len * &t[0]), &(&a + &len * &t[1]))?;
                        sb::validate_uv(s, &pc, work)?;
                        insert_seam(
                            sb::Trace::Surface {
                                surface: s.clone(),
                                pcurve: pc,
                            },
                            u.reversed,
                            &mut seams,
                            work,
                        )?;
                    }
                }
            }
        }
    }
    verify_seams(seams, work)
}
pub(super) fn insert_seam(
    mut edge: sb::Trace,
    reversed: bool,
    seams: &mut Seams,
    work: &mut Work,
) -> Result<()> {
    if reversed {
        edge.reverse()
    }
    let [a, b] = edge.endpoints()?;
    if a == b
        && sb::same_trace(
            &edge,
            &sb::Trace::Bezier(vec![[a[0].clone(), a[1].clone(), a[2].clone(), Q::one()]]),
            work,
        )?
    {
        return Ok(());
    }
    let reverse = a > b;
    let key = if reverse { (b, a) } else { (a, b) };
    if reverse {
        edge.reverse()
    }
    seams.entry(key).or_default().push((edge, reverse));
    Ok(())
}
pub(super) fn stations(
    m: &Model,
    uses: &[crate::FaceUse],
    stations: &mut BTreeSet<Q>,
) -> Result<()> {
    for use_ in uses {
        let f = &m.faces[use_.face];
        let s = &f.surface;
        for l in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for coedge in &m.loops[l].coedges {
                let c = &coedge.pcurve;
                c.validate()?;
                let d = c.domain();
                let a = Q::from_float(d[0]).unwrap();
                let len = Q::from_float(d[1]).unwrap() - &a;
                for k in &c.knots {
                    if *k >= d[0] && *k <= d[1] {
                        let t = (Q::from_float(*k).unwrap() - &a) / &len;
                        stations.insert(t.clone());
                        stations.insert(Q::one() - t);
                    }
                }
                // Exact rational crossing stations for rational degree-one
                // pcurves. Nonlinear unresolved knot crossings refuse in lift.
                if c.degree == 1 {
                    for k in c
                        .knots
                        .windows(2)
                        .filter(|k| k[0] < k[1] && k[0] >= d[0] && k[1] <= d[1])
                    {
                        let ka = Q::from_float(k[0]).unwrap();
                        let kb = Q::from_float(k[1]).unwrap();
                        let p = sb::curve(c, &ka, &kb)?;
                        for (axis, knots) in [(0, &s.knots_u), (1, &s.knots_v)] {
                            for knot in knots {
                                let knot = Q::from_float(*knot).unwrap();
                                let x = &p[0][axis] - &knot * &p[0][3];
                                let y = &p[1][axis] - &knot * &p[1][3];
                                if &x * &y < Q::zero() {
                                    let t = (&ka + (&kb - &ka) * (&x / (&x - &y)) - &a) / &len;
                                    stations.insert(t.clone());
                                    stations.insert(Q::one() - t);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
fn planar(points: &[P]) -> bool {
    for i in 1..points.len() {
        for j in i + 1..points.len() {
            let n = cross(&sub(&points[i], &points[0]), &sub(&points[j], &points[0]));
            if !zero(&n) {
                return points
                    .iter()
                    .all(|p| dot(&n, &sub(p, &points[0])).is_zero());
            }
        }
    }
    true
}
pub(super) fn append(
    m: &Model,
    use_: &crate::FaceUse,
    stations: &[Q],
    depth: u8,
    vertices: &mut BTreeMap<P, u32>,
    triangles: &mut Vec<u32>,
    seams: &mut Seams,
    tubes: &mut Vec<Poly>,
    maximum: &mut Q,
    limit: &Q,
    work: &mut Work,
) -> Result<()> {
    let face = &m.faces[use_.face];
    let s = &face.surface;
    let lo = [s.knots_u[s.degree_u], s.knots_v[s.degree_v]];
    let hi = [
        s.knots_u[s.control_points.len()],
        s.knots_v[s.control_points[0].len()],
    ];
    let anchor: [Q; 2] = std::array::from_fn(|k| {
        (Q::from_float(lo[k]).unwrap() + Q::from_float(hi[k]).unwrap()) / integer(2)
    });
    let loops = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|l| {
            m.loops[l]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    // Unlike the topology proof, filled winding enclosures also admit touching
    // and self-crossing trims. Exact source span joins must nevertheless close.
    for wire in &loops {
        let mut first = None;
        let mut last = None;
        for c in wire {
            let d = c.domain();
            let a = sb::curve(
                c,
                &Q::from_float(d[0]).unwrap(),
                &Q::from_float(c.knots[c.degree + 1]).unwrap(),
            )?;
            let start = sb::point(&a[0]);
            let knots = c
                .knots
                .iter()
                .copied()
                .filter(|t| *t < d[1])
                .last()
                .unwrap();
            let b = sb::curve(
                c,
                &Q::from_float(knots).unwrap(),
                &Q::from_float(d[1]).unwrap(),
            )?;
            let end = sb::point(b.last().unwrap());
            if last.as_ref().is_some_and(|p| *p != start) {
                return Err(fail("trim wire is not exactly joined"));
            }
            if first.is_none() {
                first = Some(start)
            }
            last = Some(end);
        }
        if first != last {
            return Err(fail("trim wire is not exactly closed"));
        }
    }
    let affine = sb::affine(s)?;
    let radial = if affine.is_some() { 1 } else { 1usize << depth };
    for c in loops.iter().flatten() {
        let d = c.domain();
        let a = Q::from_float(d[0]).unwrap();
        let len = Q::from_float(d[1]).unwrap() - &a;
        let mut previous = None;
        for t in stations.windows(2) {
            work.charge(1)?;
            let pc = sb::curve(c, &(&a + &len * &t[0]), &(&a + &len * &t[1]))?;
            sb::validate_uv(s, &pc, work)?;
            let start = sb::point(&pc[0]);
            if previous.as_ref().is_some_and(|p| *p != start) {
                return Err(fail("trim pcurve is discontinuous at an internal knot"));
            }
            previous = Some(sb::point(pc.last().unwrap()));
            insert_seam(
                sb::Trace::Surface {
                    surface: s.clone(),
                    pcurve: pc.clone(),
                },
                use_.reversed,
                seams,
                work,
            )?;
            let uv = pc
                .iter()
                .map(|p| [&p[0] / &p[3], &p[1] / &p[3]])
                .collect::<Vec<_>>();
            for r in 0..radial {
                let rs = [
                    Q::new(r.into(), radial.into()),
                    Q::new((r + 1).into(), radial.into()),
                ];
                let fan = |r: &Q, p: &[Q; 2]| {
                    std::array::from_fn::<_, 2, _>(|k| &anchor[k] + r * (&p[k] - &anchor[k]))
                };
                let box_points = rs
                    .iter()
                    .flat_map(|r| uv.iter().map(|p| fan(r, p)))
                    .collect::<Vec<_>>();
                let low = std::array::from_fn(|k| {
                    box_points
                        .iter()
                        .map(|p| p[k].clone())
                        .min()
                        .unwrap()
                        .max(Q::from_float(lo[k]).unwrap())
                });
                let high = std::array::from_fn(|k| {
                    box_points
                        .iter()
                        .map(|p| p[k].clone())
                        .max()
                        .unwrap()
                        .min(Q::from_float(hi[k]).unwrap())
                });
                let composed = if affine.is_none() {
                    sb::fan(s, &pc, &anchor, &rs, work)?
                } else {
                    None
                };
                let source = if let Some(h) = &composed {
                    h.iter().flatten().map(sb::point).collect()
                } else {
                    sb::controls(s, &low, &high)?
                };
                let corners = [
                    fan(&rs[0], &uv[0]),
                    fan(&rs[1], &uv[0]),
                    fan(&rs[1], uv.last().unwrap()),
                    fan(&rs[0], uv.last().unwrap()),
                ]
                .iter()
                .map(|p| sb::at(s, p))
                .collect::<Result<Vec<_>>>()?;
                let error = if let Some(a) = &affine {
                    sb::affine_curve_error(a, &pc).into_iter().max().unwrap()
                } else if let Some(h) = &composed {
                    sb::error(h).into_iter().max().unwrap()
                } else {
                    (0..3)
                        .map(|k| {
                            let lo = source.iter().map(|p| p[k].clone()).min().unwrap();
                            let hi = source.iter().map(|p| p[k].clone()).max().unwrap();
                            (hi - corners.iter().map(|p| p[k].clone()).min().unwrap())
                                .max(corners.iter().map(|p| p[k].clone()).max().unwrap() - lo)
                        })
                        .max()
                        .unwrap()
                };
                *maximum = maximum.clone().max(error);
                if &*maximum * integer(2) > *limit {
                    return Err(fail(
                        "continuous trimmed-source fan exceeds requested deviation",
                    ));
                }
                let ids = corners
                    .iter()
                    .map(|p| {
                        let next = vertices.len() as u32;
                        *vertices.entry(p.clone()).or_insert(next)
                    })
                    .collect::<Vec<_>>();
                for mut tr in [[0, 1, 2], [0, 2, 3]] {
                    if zero(&cross(
                        &sub(&corners[tr[1]], &corners[tr[0]]),
                        &sub(&corners[tr[2]], &corners[tr[0]]),
                    )) {
                        continue;
                    }
                    if use_.reversed {
                        tr.swap(1, 2)
                    }
                    triangles.extend(tr.map(|i| ids[i]));
                }
                let mut bound = source;
                bound.extend(corners);
                if !planar(&bound) {
                    tubes.push(hull(bound, work)?)
                }
                if triangles.len() > 6144 || vertices.len() > 1024 || tubes.len() > 1024 {
                    return Err(fail("trimmed source fan cell budget exceeded"));
                }
            }
        }
    }
    Ok(())
}
