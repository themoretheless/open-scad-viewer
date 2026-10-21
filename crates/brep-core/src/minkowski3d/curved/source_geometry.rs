//! Source geometry: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn geometry(
    m: &Model,
    depth: u8,
    limit: &Q,
    work: &mut Work,
    version: u32,
) -> Result<(Input, Vec<Poly>, Q)> {
    m.validate()?;
    if m.bodies.is_empty() || m.faces.len() > 64 || depth > 3 {
        return Err(fail(
            "NURBS source needs bounded solid bodies and depth <=3",
        ));
    }
    if m.faces
        .iter()
        .map(|f| f.surface.control_points.iter().map(Vec::len).sum::<usize>())
        .sum::<usize>()
        > 8192
    {
        return Err(fail("NURBS source exceeds 8192 surface controls"));
    }
    let mut uses = Vec::new();
    for body in &m.bodies {
        for shell in std::iter::once(body.outer_shell).chain(body.inner_shells.iter().copied()) {
            if !m.shells[shell].closed {
                return Err(fail("NURBS source shell is open"));
            }
            uses.extend(m.shells[shell].faces.iter().cloned())
        }
    }
    let mut stations = BTreeSet::from([Q::zero(), Q::one()]);
    for u in &uses {
        if version == 1 {
            rectangle(m, u.face)?;
        }
        let s = &m.faces[u.face].surface;
        if !(1..=8).contains(&s.degree_u)
            || !(1..=8).contains(&s.degree_v)
            || s.control_points
                .iter()
                .flatten()
                .any(|p| p.len() != 3 || p.iter().any(|x| !x.is_finite() || x.abs() > 1e6))
        {
            return Err(fail("NURBS source degree or coordinate budget exceeded"));
        }
        if version >= 2 {
            sb::continuous(s, work)?;
        }
        for (knots, degree, count) in [
            (&s.knots_u, s.degree_u, s.control_points.len()),
            (&s.knots_v, s.degree_v, s.control_points[0].len()),
        ] {
            let a = Q::from_float(knots[degree]).unwrap();
            let b = Q::from_float(knots[count]).unwrap();
            for k in &knots[degree..=count] {
                let x = (Q::from_float(*k).unwrap() - &a) / (&b - &a);
                stations.insert(x.clone());
                stations.insert(Q::one() - x);
            }
        }
    }
    if version >= 2 {
        trimmed::stations(m, &uses, &mut stations)?;
    }
    for i in 0..=(1usize << depth) {
        stations.insert(Q::new(i.into(), (1usize << depth).into()));
    }
    let stations = stations.into_iter().collect::<Vec<_>>();
    if stations.len() > 33
        || uses.iter().filter(|u| rectangle(m, u.face).is_ok()).count()
            * (stations.len() - 1).pow(2)
            > 512
    {
        return Err(fail("NURBS exact chart cell budget exceeded"));
    }
    let mut vertices = BTreeMap::<P, u32>::new();
    let mut triangles = Vec::new();
    let mut seams = trimmed::Seams::new();
    let mut tubes = Vec::new();
    let mut maximum = Q::zero();
    let mut has_trimmed = false;
    for use_ in uses {
        if rectangle(m, use_.face).is_err() {
            has_trimmed = true;
            trimmed::append(
                m,
                &use_,
                &stations,
                depth,
                &mut vertices,
                &mut triangles,
                &mut seams,
                &mut tubes,
                &mut maximum,
                limit,
                work,
            )?;
            continue;
        }
        let s = &m.faces[use_.face].surface;
        let da = [
            Q::from_float(s.knots_u[s.degree_u]).unwrap(),
            Q::from_float(s.knots_u[s.control_points.len()]).unwrap(),
        ];
        let db = [
            Q::from_float(s.knots_v[s.degree_v]).unwrap(),
            Q::from_float(s.knots_v[s.control_points[0].len()]).unwrap(),
        ];
        for u in stations.windows(2) {
            for v in stations.windows(2) {
                work.charge(1)?;
                let a = |t: &Q| &da[0] + t * (&da[1] - &da[0]);
                let b = |t: &Q| &db[0] + t * (&db[1] - &db[0]);
                let h = sb::cell(s, [a(&u[0]), a(&u[1])], [b(&v[0]), b(&v[1])])?;
                let c = sb::corners(&h);
                let err = sb::error(&h);
                maximum = maximum.max(err.iter().cloned().max().unwrap());
                if &maximum * integer(2) > *limit {
                    return Err(fail(
                        "continuous NURBS source deviation exceeds the requested bound",
                    ));
                }
                for edge in sb::edges(&h) {
                    trimmed::insert_seam(sb::Trace::Bezier(edge), use_.reversed, &mut seams, work)?;
                }
                let ids = c
                    .iter()
                    .map(|p| {
                        let next = vertices.len() as u32;
                        *vertices.entry(p.clone()).or_insert(next)
                    })
                    .collect::<Vec<_>>();
                let diagonals = if dot(&sub(&c[0], &c[2]), &sub(&c[0], &c[2]))
                    >= dot(&sub(&c[1], &c[3]), &sub(&c[1], &c[3]))
                {
                    [[0, 1, 2], [0, 2, 3]]
                } else {
                    [[0, 1, 3], [1, 2, 3]]
                };
                for mut t in diagonals {
                    if zero(&cross(&sub(&c[t[1]], &c[t[0]]), &sub(&c[t[2]], &c[t[0]]))) {
                        continue;
                    }
                    if use_.reversed {
                        t.swap(1, 2)
                    }
                    triangles.extend(t.map(|i| ids[i]));
                }
                if !sb::planar(&h) {
                    let points = h.iter().flatten().map(sb::point).collect::<Vec<_>>();
                    let lo: P = std::array::from_fn(|i| {
                        points
                            .iter()
                            .map(|p| p[i].clone())
                            .min()
                            .unwrap()
                            .max(c.iter().map(|p| p[i].clone()).min().unwrap() - &err[i])
                    });
                    let hi: P = std::array::from_fn(|i| {
                        points
                            .iter()
                            .map(|p| p[i].clone())
                            .max()
                            .unwrap()
                            .min(c.iter().map(|p| p[i].clone()).max().unwrap() + &err[i])
                    });
                    if (0..3).all(|i| lo[i] < hi[i]) {
                        let bounds = bounding_box(&[lo, hi])?;
                        if version == 1 {
                            tubes.push(bounds)
                        } else {
                            let mut tube = Some(hull(points, work)?);
                            for f in &bounds.faces {
                                if let Some(t) = tube {
                                    tube = t.clip(&plane(f), work)?
                                }
                            }
                            if let Some(t) = tube {
                                tubes.push(t)
                            }
                        }
                    }
                }
            }
        }
    }
    trimmed::verify_seams(seams, work)?;
    let mut points = vec![[Q::zero(), Q::zero(), Q::zero()]; vertices.len()];
    for (p, i) in vertices {
        points[i as usize] = p
    }
    Ok((
        input_chain(&points, &triangles, !has_trimmed)?,
        tubes,
        maximum,
    ))
}
