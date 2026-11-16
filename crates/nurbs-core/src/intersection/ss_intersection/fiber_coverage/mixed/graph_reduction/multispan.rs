//! Exact original B-spline blossom restrictions on a common CLOSED knot grid.
//! No inserted/rounded poles, stitched samples or locally inferred XY frame.
use super::*;
fn charge(work: &mut Work) -> Result<()> {
    if work.used == work.max {
        return Err(crate::resource(
            "Original graph restriction budget exhausted",
        ));
    }
    work.used += 1;
    Ok(())
}
fn admit_multi(s: &Surface) -> Result<()> {
    s.validate()?;
    check(
        !s.periodic_u
            && !s.periodic_v
            && (1..=8).contains(&s.degree_u)
            && (1..=8).contains(&s.degree_v)
            && s.control_points.len() <= 128
            && s.control_points[0].len() <= 128
            && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Need bounded nonperiodic original graph spans",
    )?;
    for (k, p, n) in [
        (&s.knots_u, s.degree_u, s.control_points.len()),
        (&s.knots_v, s.degree_v, s.control_points[0].len()),
    ] {
        check(
            k[..=p].iter().all(|x| *x == k[p]) && k[n..].iter().all(|x| *x == k[n]),
            "Need clamped original graph domains",
        )?;
        let mut i = p + 1;
        while i < n {
            let mut j = i + 1;
            while j < n && k[j] == k[i] {
                j += 1;
            }
            check(j - i <= p, "Discontinuous original graph knot")?;
            i = j;
        }
    }
    Ok(())
}
fn normalized_knots(s: &Surface, axis: usize) -> Result<Vec<Q>> {
    let (k, p, n) = if axis == 0 {
        (&s.knots_u, s.degree_u, s.control_points.len())
    } else {
        (&s.knots_v, s.degree_v, s.control_points[0].len())
    };
    let lo = q(k[p])?;
    let width = q(k[n])? - &lo;
    k[p..=n]
        .iter()
        .map(|x| Ok((q(*x)? - &lo) / &width))
        .collect()
}
fn frame_multi(s: &Surface, work: &mut Work) -> Result<[Q; 4]> {
    admit_multi(s)?;
    let origin = [q(s.control_points[0][0][0])?, q(s.control_points[0][0][1])?];
    let delta = [
        q(s.control_points[s.control_points.len() - 1][0][0])? - &origin[0],
        q(s.control_points[0][s.control_points[0].len() - 1][1])? - &origin[1],
    ];
    check(
        delta.iter().all(|x| !x.is_zero()),
        "Degenerate original graph XY frame",
    )?;
    if !constant_weights(s) {
        // Rational frames are proposal data until every homogeneous span
        // has verified the same global coefficients.
        return Ok([origin[0].clone(), origin[1].clone(), delta[0].clone(), delta[1].clone()]);
    }
    let mut greville = Vec::new();
    for (k, p, n) in [
        (&s.knots_u, s.degree_u, s.control_points.len()),
        (&s.knots_v, s.degree_v, s.control_points[0].len()),
    ] {
        let lo = q(k[p])?;
        let width = q(k[n])? - &lo;
        let mut g = Vec::new();
        for i in 0..n {
            charge(work)?;
            let mut sum = Q::zero();
            for x in &k[i + 1..=i + p] {
                sum += q(*x)?;
            }
            g.push((sum / Q::from_integer(p.into()) - &lo) / &width);
        }
        greville.push(g);
    }
    for (i, row) in s.control_points.iter().enumerate() {
        for (j, p) in row.iter().enumerate() {
            charge(work)?;
            check(
                q(p[0])? == &origin[0] + &delta[0] * &greville[0][i]
                    && q(p[1])? == &origin[1] + &delta[1] * &greville[1][j],
                "Whole original Greville affine XY identity failed",
            )?;
        }
    }
    Ok([
        origin[0].clone(),
        origin[1].clone(),
        delta[0].clone(),
        delta[1].clone(),
    ])
}
fn restrict(
    mut_base: Vec<Q>,
    knots: &[Q],
    p: usize,
    span: usize,
    range: &[Q; 2],
    work: &mut Work,
) -> Result<Vec<Q>> {
    let mut out = Vec::new();
    for b in 0..=p {
        let mut d = mut_base.clone();
        for r in 1..=p {
            let t = &range[usize::from(r > p - b)];
            for j in (r..=p).rev() {
                charge(work)?;
                let i = span - p + j;
                let den = &knots[i + p - r + 1] - &knots[i];
                check(
                    den > Q::zero(),
                    "Original graph blossom denominator vanished",
                )?;
                let a = (t - &knots[i]) / den;
                d[j] = (&Q::from_integer(1.into()) - &a) * &d[j - 1] + a * &d[j];
            }
        }
        out.push(d[p].clone());
    }
    Ok(out)
}
fn net_with(
    s: &Surface,
    cell: &Cell,
    work: &mut Work,
    control: impl Fn(usize, usize) -> Result<Q>,
) -> Result<Net> {
    let mut spans = [0; 2];
    let mut ranges = Vec::new();
    let mut all_knots = Vec::new();
    for (axis, (k, p, n)) in [
        (&s.knots_u, s.degree_u, s.control_points.len()),
        (&s.knots_v, s.degree_v, s.control_points[0].len()),
    ]
    .into_iter()
    .enumerate()
    {
        let knots = k.iter().map(|x| q(*x)).collect::<Result<Vec<_>>>()?;
        let width = &knots[n] - &knots[p];
        let range = [
            &knots[p] + &width * &cell[axis][0],
            &knots[p] + &width * &cell[axis][1],
        ];
        spans[axis] = (p..n)
            .find(|&i| knots[i] < knots[i + 1] && knots[i] <= range[0] && range[1] <= knots[i + 1])
            .ok_or_else(|| crate::input("Common graph cell crosses an original knot"))?;
        ranges.push(range);
        all_knots.push(knots);
    }
    let (p, r) = (s.degree_u, s.degree_v);
    let (iu, iv) = (spans[0], spans[1]);
    let mut net = vec![vec![Q::zero(); r + 1]; p + 1];
    for j in 0..=r {
        let base = (iu - p..=iu)
            .map(|i| control(i, iv - r + j))
            .collect::<Result<Vec<_>>>()?;
        let col = restrict(base, &all_knots[0], p, iu, &ranges[0], work)?;
        for i in 0..=p {
            net[i][j] = col[i].clone();
        }
    }
    for row in &mut net {
        *row = restrict(row.clone(), &all_knots[1], r, iv, &ranges[1], work)?;
    }
    Ok(net)
}
fn net(s: &Surface, cell: &Cell, work: &mut Work) -> Result<Net> {
    net_with(s, cell, work, |i, j| q(s.control_points[i][j][2]))
}

fn rational_height(
    s: &Surface,
    cell: &Cell,
    frame: &[Q; 4],
    work: &mut Work,
) -> Result<(Net, Net)> {
    let weights = net_with(s, cell, work, |i, j| q(s.weights[i][j]))?;
    let coordinate = |axis: usize, work: &mut Work| {
        net_with(s, cell, work, |i, j| Ok(q(s.control_points[i][j][axis])? * q(s.weights[i][j])?))
    };
    for axis in 0..2 {
        let [lo, hi] = cell[axis].each_ref().map(|u| &frame[axis] + &frame[axis + 2] * u);
        let affine = if axis == 0 { vec![vec![lo], vec![hi]] } else { vec![vec![lo, hi]] };
        charge(work)?;
        check(
            elevate(&coordinate(axis, work)?, axis) == rational::product(&weights, &affine),
            "Original rational knot-cell affine projection identity failed",
        )?;
    }
    Ok((coordinate(2, work)?, weights))
}

pub(in crate::intersection::ss_intersection) fn equivalent_multispan_affine_surface(s: &Surface) -> Option<Surface> {
    admit_multi(s).ok()?;
    if s.control_points.len() <= s.degree_u + 1 && s.control_points[0].len() <= s.degree_v + 1 {
        return None;
    }
    let last_u = s.control_points.len() - 1;
    let last_v = s.control_points[0].len() - 1;
    let mut coefficients = Vec::with_capacity(3);
    for axis in 0..3 {
        let origin = q(s.control_points[0][0][axis]).ok()?;
        let du = q(s.control_points[last_u][0][axis]).ok()? - &origin;
        let dv = q(s.control_points[0][last_v][axis]).ok()? - &origin;
        if q(s.control_points[last_u][last_v][axis]).ok()? != &origin + &du + &dv {
            return None;
        }
        coefficients.push([origin, du, dv]);
    }
    if !(0..3).any(|axis| {
        let [_, du, dv] = &coefficients[axis];
        let [_, other_du, other_dv] = &coefficients[(axis + 1) % 3];
        du * other_dv != dv * other_du
    }) {
        return None;
    }
    let mut work = Work { used: 0, max: 100_000 };
    let mut u = normalized_knots(s, 0).ok()?;
    let mut v = normalized_knots(s, 1).ok()?;
    u.dedup();
    v.dedup();
    if (u.len() - 1) * (v.len() - 1) > 64 {
        return None;
    }
    for a in u.windows(2) {
        for b in v.windows(2) {
            let cell = [[a[0].clone(), a[1].clone()], [b[0].clone(), b[1].clone()]];
            let weights = net_with(s, &cell, &mut work, |i, j| q(s.weights[i][j])).ok()?;
            for (axis, [origin, du, dv]) in coefficients.iter().enumerate() {
                let affine = a.iter().map(|u| {
                    b.iter().map(|v| origin + du * u + dv * v).collect()
                }).collect();
                let numerator = net_with(s, &cell, &mut work, |i, j| {
                    Ok(q(s.control_points[i][j][axis])? * q(s.weights[i][j])?)
                }).ok()?;
                charge(&mut work).ok()?;
                if elevate(&elevate(&numerator, 0), 1) != rational::product(&weights, &affine) {
                    return None;
                }
            }
        }
    }
    crate::surface_injectivity::exact_affine_xy::affine_corners(s)
}

pub(super) fn derive(source: &FiberSources, work: &mut Work) -> Result<Vec<(Cell, Net)>> {
    let (a, b) = (source.first(), source.second());
    let polynomial = constant_weights(a) && constant_weights(b);
    check(
        polynomial || [a, b].iter().any(|s| {
            s.control_points.len() > s.degree_u + 1
                || s.control_points[0].len() > s.degree_v + 1
        }),
        "Single-span rational carriers use direct affine alignment",
    )?;
    let frame = frame_multi(a, work)?;
    check(frame == frame_multi(b, work)?, "Original graph global XY frames differ")?;
    let mut cuts = Vec::new();
    for axis in 0..2 {
        let mut k = normalized_knots(a, axis)?;
        k.extend(normalized_knots(b, axis)?);
        k.sort();
        k.dedup();
        cuts.push(k);
    }
    check(
        (cuts[0].len() - 1) * (cuts[1].len() - 1) <= 64,
        "Original graph knot product exceeds 64",
    )?;
    let mut patches = Vec::new();
    for u in cuts[0].windows(2) {
        for v in cuts[1].windows(2) {
            let cell = [[u[0].clone(), u[1].clone()], [v[0].clone(), v[1].clone()]];
            let (mut x, mut y) = if polynomial {
                (net(a, &cell, work)?, net(b, &cell, work)?)
            } else {
                let height = |surface, work: &mut Work| {
                    if constant_weights(surface) {
                        Ok((net(surface, &cell, work)?, vec![vec![Q::from_integer(1.into())]]))
                    } else {
                        rational_height(surface, &cell, &frame, work)
                    }
                };
                let (a, wa) = height(a, work)?;
                let (b, wb) = height(b, work)?;
                charge(work)?;
                (rational::product(&a, &wb), rational::product(&b, &wa))
            };
            while x.len() < y.len() {
                x = elevate(&x, 0);
            }
            while y.len() < x.len() {
                y = elevate(&y, 0);
            }
            while x[0].len() < y[0].len() {
                x = elevate(&x, 1);
            }
            while y[0].len() < x[0].len() {
                y = elevate(&y, 1);
            }
            let gap = y
                .iter()
                .zip(&x)
                .map(|(y, x)| y.iter().zip(x).map(|(y, x)| y - x).collect())
                .collect();
            patches.push((cell, gap));
        }
    }
    Ok(patches)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_multispan_projection_is_proved_on_every_homogeneous_cell() {
        let original = source();
        let mut rational = original.first().clone();
        rational.degree_u = 2;
        rational.knots_u = vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.];
        rational.control_points = [0., 0.125, 0.5, 0.875, 1.]
            .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .to_vec();
        rational.weights = [1., 2., 3., 2., 1.].map(|w| vec![w; 2]).to_vec();
        let mut upper = original.first().clone();
        for point in upper.control_points.iter_mut().flatten() {
            point[2] = 1.;
        }
        let pair = FiberSources::new(&rational, &upper).unwrap();
        let mut work = Work { used: 0, max: 100_000 };
        let patches = derive(&pair, &mut work).unwrap();
        assert_eq!(patches.len(), 2);
        assert!(patches.iter().all(|(_, gap)| gap.iter().flatten().all(Q::is_positive)));
        let mut oblique = rational.clone();
        for point in oblique.control_points.iter_mut().flatten() {
            let (x, y) = (point[0], point[1]);
            point.copy_from_slice(&[x + y, x - y, x + 2. * y]);
        }
        let view = equivalent_multispan_affine_surface(&oblique).unwrap();
        assert_eq!(view.control_points[1][1], [2., 0., 3.]);
        assert_eq!(view.knots_u, [0., 0., 1., 1.]);
        let mut polynomial = oblique.clone();
        for (row, x) in polynomial.control_points.iter_mut().zip([0., 0.25, 0.5, 0.75, 1.]) {
            *row = [0., 1.].map(|y| vec![x + y, x - y, x + 2. * y]).to_vec();
        }
        polynomial.weights = vec![vec![1.; 2]; 5];
        assert_eq!(equivalent_multispan_affine_surface(&polynomial).unwrap(), view);
        oblique.control_points[3][0][2] += 0.03125;
        assert!(equivalent_multispan_affine_surface(&oblique).is_none());
        rational.control_points[3][0][0] += 0.03125;
        let pair = FiberSources::new(&rational, &upper).unwrap();
        assert!(derive(&pair, &mut work).is_err());
    }

    fn source() -> FiberSources {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut b = a.clone();
        b.degree_u = 2;
        b.degree_v = 2;
        b.knots_u = vec![0., 0., 0., 0.25, 0.5, 0.75, 1., 1., 1.];
        b.knots_v = vec![0., 0., 0., 1., 1., 1.];
        let x = [0., 0.125, 0.375, 0.625, 0.875, 1.];
        let z = [0.0625, -0.0625, 0.0625, 0.0625, -0.0625, 0.0625];
        let v = [0.25, -0.25, 0.25];
        b.control_points = (0..6)
            .map(|i| {
                (0..3)
                    .map(|j| vec![x[i], j as f64 / 2., z[i] + v[j]])
                    .collect()
            })
            .collect();
        b.weights = vec![vec![1.; 3]; 6];
        FiberSources::new(&a, &b).unwrap()
    }
    fn basis(i: usize, p: usize, t: &Q, k: &[f64], n: usize) -> Q {
        if p == 0 {
            return if (q(k[i]).unwrap() <= *t && *t < q(k[i + 1]).unwrap())
                || (*t == q(k[n]).unwrap() && i == n - 1)
            {
                Q::from_integer(1.into())
            } else {
                Q::zero()
            };
        }
        let mut out = Q::zero();
        let d = q(k[i + p]).unwrap() - q(k[i]).unwrap();
        if !d.is_zero() {
            out += (t - q(k[i]).unwrap()) / d * basis(i, p - 1, t, k, n);
        }
        let d = q(k[i + p + 1]).unwrap() - q(k[i + 1]).unwrap();
        if !d.is_zero() {
            out += (q(k[i + p + 1]).unwrap() - t) / d * basis(i + 1, p - 1, t, k, n);
        }
        out
    }
    #[test]
    fn all_common_closed_cells_restrict_the_original_cox_basis_exactly() {
        let source = source();
        let mut work = Work {
            used: 0,
            max: 100000,
        };
        let patches = derive(&source, &mut work).unwrap();
        assert_eq!(patches.len(), 4);
        assert!(work.used > 0 && work.used < work.max);
        for (index, (cell, net)) in patches.iter().enumerate() {
            assert_eq!(
                *cell,
                [
                    [
                        Q::new(index.into(), 4.into()),
                        Q::new((index + 1).into(), 4.into())
                    ],
                    [q(0.).unwrap(), q(1.).unwrap()]
                ]
            );
            for i in 0..=4 {
                for j in 0..=4 {
                    let u = Q::new(i.into(), 4.into());
                    let v = Q::new(j.into(), 4.into());
                    let actual_u =
                        cell[0][0].clone() + (cell[0][1].clone() - cell[0][0].clone()) * &u;
                    let actual_v = v.clone();
                    let s = source.second();
                    let mut expected = Q::zero();
                    for a in 0..s.control_points.len() {
                        for b in 0..s.control_points[0].len() {
                            expected +=
                                basis(a, s.degree_u, &actual_u, &s.knots_u, s.control_points.len())
                                    * basis(
                                        b,
                                        s.degree_v,
                                        &actual_v,
                                        &s.knots_v,
                                        s.control_points[0].len(),
                                    )
                                    * q(s.control_points[a][b][2]).unwrap();
                        }
                    }
                    let mut evaluated = Q::zero();
                    for a in 0..net.len() {
                        for b in 0..net[0].len() {
                            let c = |n: usize, k: usize| {
                                (0..k).fold(1usize, |x, l| x * (n - l) / (l + 1))
                            };
                            evaluated += &net[a][b]
                                * Q::from_integer(c(net.len() - 1, a).into())
                                * u.pow(a as i32)
                                * (Q::from_integer(1.into()) - &u).pow((net.len() - 1 - a) as i32)
                                * Q::from_integer(c(net[0].len() - 1, b).into())
                                * v.pow(b as i32)
                                * (Q::from_integer(1.into()) - &v)
                                    .pow((net[0].len() - 1 - b) as i32);
                        }
                    }
                    assert_eq!(evaluated, expected, "{index},{i},{j}");
                }
            }
        }
    }
    #[test]
    fn source_projection_damage_and_shared_restriction_exhaustion_refuse() {
        let source = source();
        let mut work = Work { used: 0, max: 1 };
        let error = derive(&source, &mut work).unwrap_err();
        assert_eq!(error.code, crate::RESOURCE_LIMIT);
        assert_eq!(work.used, 1);
        let mut b = source.second().clone();
        b.control_points[2][1][0] = b.control_points[2][1][0].next_up();
        let changed = FiberSources::new(source.first(), &b).unwrap();
        assert!(
            derive(
                &changed,
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
    }
}
