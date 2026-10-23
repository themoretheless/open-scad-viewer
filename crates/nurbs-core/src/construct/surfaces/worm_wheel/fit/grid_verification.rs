//! Grid verification: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn verify(
    source: &Source,
    domain: ChartDomain,
    u: &[f64],
    b: &[f64],
    surfaces: &[Surface],
    max_patches: usize,
    op: &Operator,
) -> Result<GridCertificate> {
    source.validate(domain)?;
    source.replay()?;
    validate_layout(domain, u, b, surfaces.len(), max_patches)?;
    let nb = b.len() - 1;
    let is_physical_y = source.physical_y();
    let mut report = GridCertificate {
        error_upper: 0.,
        interpolation_error_upper: 0.,
        control_difference_upper: 0.,
        directional_error_upper: [0.; 2],
        source_regular: true,
        saved_regular: true,
        patches: surfaces.len(),
    };
    for (i, s) in surfaces.iter().enumerate() {
        let iu = i / nb;
        let ib = i % nb;
        let cell = ChartDomain {
            u: [u[iu], u[iu + 1]],
            beta: [b[ib], b[ib + 1]],
        };
        s.validate()?;
        check(
            s.degree_u == 5
                && s.degree_v == 5
                && s.knots_u.len() == 12
                && s.knots_u[..6].iter().all(|&x| x == cell.u[0])
                && s.knots_u[6..].iter().all(|&x| x == cell.u[1])
                && s.knots_v.len() == 12
                && s.knots_v[..6].iter().all(|&x| x == cell.beta[0])
                && s.knots_v[6..].iter().all(|&x| x == cell.beta[1])
                && !s.periodic_u
                && !s.periodic_v
                && s.control_points.len() == 6
                && s.control_points
                    .iter()
                    .all(|r| r.len() == 6 && r.iter().all(|p| p.len() == 3))
                && s.weights.iter().flatten().all(|w| *w == 1.),
            "Wheel fit requires original quintic tensor Bernstein patches and exact source domains",
        )?;
        if is_physical_y {
            let values = physical_y_controls(cell.beta)?;
            check(
                s.control_points.iter().all(|row| {
                    row.iter()
                        .zip(values)
                        .all(|(p, y)| p[1].to_bits() == y.to_bits())
                }),
                "Axial grid lost its canonical physical-Y controls or end planes",
            )?;
        }
        let (regular, sixth) = source.cell(cell)?;
        report.source_regular &= regular;
        report.saved_regular &= saved_regular(s, cell)?;
        let ideal = ideal_net(source, cell, op)?;
        let mut delta = 0_f64;
        for j in 0..6 {
            for k in 0..6 {
                let p = &s.control_points[j][k];
                let actual = [I::point(p[0]), I::point(p[1]), I::point(p[2])];
                delta = delta.max(box_distance(&actual, &ideal[j][k])?.1);
            }
        }
        let terms = interpolation_error(sixth, cell, op)?;
        let remainder = I::point(terms[0]).add(I::point(terms[1]))?.hi;
        report.control_difference_upper = report.control_difference_upper.max(delta);
        report.interpolation_error_upper = report.interpolation_error_upper.max(remainder);
        for d in 0..2 {
            report.directional_error_upper[d] = report.directional_error_upper[d].max(terms[d]);
        }
        report.error_upper = report
            .error_upper
            .max(I::point(remainder).add(I::point(delta))?.hi);
        if iu > 0 {
            let previous = &surfaces[(iu - 1) * nb + ib];
            check(
                previous.control_points[5] == s.control_points[0],
                "Wheel NURBS U seam differs in actual original controls",
            )?;
        }
        if ib > 0 {
            let previous = &surfaces[i - 1];
            check(
                (0..6).all(|j| previous.control_points[j][5] == s.control_points[j][0]),
                "Wheel NURBS beta seam differs in actual original controls",
            )?;
        }
    }
    Ok(report)
}

pub(super) fn validate_layout(
    domain: ChartDomain,
    u: &[f64],
    b: &[f64],
    surface_count: usize,
    max_patches: usize,
) -> Result<()> {
    check(
        u.len() >= 2
            && b.len() >= 2
            && u.len() <= 4097
            && b.len() <= 4097
            && u.first() == Some(&domain.u[0])
            && u.last() == Some(&domain.u[1])
            && b.first() == Some(&domain.beta[0])
            && b.last() == Some(&domain.beta[1])
            && u.iter().chain(b).all(|x| x.is_finite())
            && u.windows(2).all(|x| x[0] < x[1])
            && b.windows(2).all(|x| x[0] < x[1])
            && (1..=4096).contains(&max_patches)
            && (u.len() - 1) * (b.len() - 1) == surface_count
            && surface_count <= max_patches,
        "Wheel NURBS grid must cover the original rectangle with a complete bounded patch census",
    )
}

pub(super) fn saved_regular(s: &Surface, domain: ChartDomain) -> Result<bool> {
    let mut du = [I::point(0.); 3];
    let mut db = du;
    let mut lo = [[f64::INFINITY; 3]; 2];
    let mut hi = [[f64::NEG_INFINITY; 3]; 2];
    for axis in 0..2 {
        let range = if axis == 0 { domain.u } else { domain.beta };
        let scale = I::point(5.).div(I::point(range[1]).sub(I::point(range[0]))?)?;
        for i in 0..6 {
            for j in 0..6 {
                if (axis == 0 && i == 5) || (axis == 1 && j == 5) {
                    continue;
                }
                let next = if axis == 0 {
                    &s.control_points[i + 1][j]
                } else {
                    &s.control_points[i][j + 1]
                };
                for d in 0..3 {
                    let value = I::point(next[d])
                        .sub(I::point(s.control_points[i][j][d]))?
                        .mul(scale)?;
                    lo[axis][d] = lo[axis][d].min(value.lo);
                    hi[axis][d] = hi[axis][d].max(value.hi);
                }
            }
        }
    }
    for d in 0..3 {
        du[d] = I::new(lo[0][d], hi[0][d])?;
        db[d] = I::new(lo[1][d], hi[1][d])?;
    }
    let cross = [
        du[1].mul(db[2])?.sub(du[2].mul(db[1])?)?,
        du[2].mul(db[0])?.sub(du[0].mul(db[2])?)?,
        du[0].mul(db[1])?.sub(du[1].mul(db[0])?)?,
    ];
    Ok(cross.iter().any(|x| x.lo > 0. || x.hi < 0.))
}

pub(super) fn interpolation_error(sixth: [[I; 3]; 2], domain: ChartDomain, op: &Operator) -> Result<[f64; 2]> {
    let mut out = [0.; 2];
    for axis in 0..2 {
        let range = if axis == 0 { domain.u } else { domain.beta };
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let coefficient = norm(sixth[axis])?;
        out[axis] = I::point(coefficient)
            .mul(sixth_power(width)?)?
            .mul(I::point(op.weight))?
            .hi;
    }
    Ok([out[0], I::point(out[1]).mul(I::point(op.lebesgue))?.hi])
}

pub(super) fn node(range: [f64; 2], i: usize) -> Result<[f64; 2]> {
    if i == 0 {
        return Ok([range[0]; 2]);
    }
    if i == 5 {
        return Ok([range[1]; 2]);
    }
    let x = I::point(range[0]).add(
        I::point(range[1])
            .sub(I::point(range[0]))?
            .mul(I::point(NODES[i]))?,
    )?;
    Ok([x.lo.max(range[0]), x.hi.min(range[1])])
}

pub(super) fn ideal_net(source: &Source, domain: ChartDomain, op: &Operator) -> Result<[[[I; 3]; 6]; 6]> {
    let mut samples = [[[I::point(0.); 3]; 6]; 6];
    let u_nodes = [
        node(domain.u, 0)?,
        node(domain.u, 1)?,
        node(domain.u, 2)?,
        node(domain.u, 3)?,
        node(domain.u, 4)?,
        node(domain.u, 5)?,
    ];
    // The implicit stock root depends on beta, not on the transverse sample.
    // Prove it once for each complete beta node box; immutable ownership keeps
    // this local reuse bound to the same original source and radius.
    for j in 0..6 {
        let beta = node(domain.beta, j)?;
        let row_source = source.at_beta(beta)?;
        for i in 0..6 {
            let p = row_source.point(ChartDomain {
                u: u_nodes[i],
                beta,
            })?;
            samples[i][j] = [iv(p[0])?, iv(p[1])?, iv(p[2])?];
        }
    }
    for row in &mut samples {
        *row = transform(row, op)?;
    }
    let mut net = samples;
    for j in 0..6 {
        let data = std::array::from_fn(|i| samples[i][j]);
        let b = transform(&data, op)?;
        for i in 0..6 {
            net[i][j] = b[i];
        }
    }
    Ok(net)
}

pub(super) fn norm(v: [I; 3]) -> Result<f64> {
    let mut sum = I::point(0.);
    for x in v {
        let a = I::point(x.lo.abs().max(x.hi.abs()));
        sum = sum.add(a.mul(a)?)?;
    }
    Ok(sum.hi.max(0.).sqrt().next_up())
}

pub(super) fn sixth_power(x: I) -> Result<I> {
    let xx = x.mul(x)?;
    xx.mul(xx)?.mul(xx)
}

/// Uniform C0 grids refine anisotropically until the unchanged world tolerance,
/// original chart regularity and actual saved-patch regularity are all proved.
/// No singular/budget leaf is dropped; incomplete fits explicitly refuse.
pub fn fit_chart(
    spec: Spec,
    flank: Flank,
    domain: ChartDomain,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedEnvelopeGrid> {
    let grid = fit(
        Source::Original(spec, flank),
        domain,
        tolerance,
        max_patches,
    )?;
    Ok(QualifiedEnvelopeGrid { grid, flank })
}

pub(super) fn fit(
    source: Source,
    domain: ChartDomain,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedGrid> {
    source.validate(domain)?;
    source.replay()?;
    check(
        domain.u[0] < domain.u[1]
            && domain.beta[0] < domain.beta[1]
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=4096).contains(&max_patches),
        "Wheel NURBS fit needs nonempty source rectangle, positive tolerance and bounded grid",
    )?;
    let op = operator()?;
    let mut nu = 1;
    let mut nb = 1;
    loop {
        let u = breaks(domain.u, nu);
        let beta = breaks(domain.beta, nb);
        let report = build(&source, &u, &beta, &op)
            .and_then(|s| verify(&source, domain, &u, &beta, &s, max_patches, &op).map(|r| (s, r)));
        let axis = match report {
            Ok((surfaces, certificate)) => {
                if certificate.error_upper <= tolerance
                    && certificate.source_regular
                    && certificate.saved_regular
                {
                    let packed = pack(&surfaces, &u, &beta)?;
                    return Ok(QualifiedGrid {
                        source,
                        domain,
                        u,
                        beta,
                        surfaces,
                        packed,
                        certificate,
                        tolerance,
                    });
                }
                if !certificate.source_regular || !certificate.saved_regular {
                    usize::from(nu > nb)
                } else {
                    usize::from(
                        certificate.directional_error_upper[1]
                            > certificate.directional_error_upper[0],
                    )
                }
            }
            Err(_) => usize::from(nu > nb),
        };
        check(
            nu * nb * 2 <= max_patches,
            "Wheel NURBS original-source error/regularity remains unproved within the patch budget",
        )?;
        if axis == 0 {
            nu *= 2;
        } else {
            nb *= 2;
        }
    }
}
