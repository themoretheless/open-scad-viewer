//! Rational chart collars composed from original Bernstein controls.
//! Position bounds do not certify derivative jets, G2, trim ownership or a solid.
use crate::{Error, Result, curve::Curve, interval_eval::Interval as I, surface::Surface};
type Poly = Vec<Vec<I>>;
mod jet_difference;
pub use jet_difference::{G2Report, JetReport, inspect_g2, inspect_g2_opposite_cross, inspect_g2_cross_frame, inspect_jets};
struct Work {
    used: usize,
    max: usize,
}
impl Work {
    fn charge(&mut self, n: usize) -> Result<()> {
        if n > self.max - self.used {
            return Err(Error::new(
                "NURBS_SURFACE_COLLAR_WORK_LIMIT",
                "Collar coefficient work exhausted",
            ));
        }
        self.used += n;
        Ok(())
    }
}
fn refused(s: &str) -> Error {
    Error::new("NURBS_SURFACE_COLLAR_UNPROVEN", s)
}
fn choose(n: usize, k: usize) -> f64 {
    let mut v = 1_u64;
    for i in 0..k.min(n - k) {
        v = v * (n - i) as u64 / (i + 1) as u64;
    }
    v as f64
}
fn mul(a: &Poly, b: &Poly, w: &mut Work) -> Result<Poly> {
    let (p, q, r, s) = (a.len() - 1, b.len() - 1, a[0].len() - 1, b[0].len() - 1);
    let mut out = vec![vec![I::point(0.); r + s + 1]; p + q + 1];
    for i in 0..=p {
        for j in 0..=q {
            for k in 0..=r {
                for l in 0..=s {
                    w.charge(8)?;
                    let u = I::point(choose(p, i))
                        .mul(I::point(choose(q, j)))?
                        .div(I::point(choose(p + q, i + j)))?;
                    let v = I::point(choose(r, k))
                        .mul(I::point(choose(s, l)))?
                        .div(I::point(choose(r + s, k + l)))?;
                    out[i + j][k + l] =
                        out[i + j][k + l].add(a[i][k].mul(b[j][l])?.mul(u.mul(v)?)?)?;
                }
            }
        }
    }
    Ok(out)
}
fn powers(p: &Poly, n: usize, w: &mut Work) -> Result<Vec<Poly>> {
    let mut out = vec![vec![vec![I::point(1.)]]];
    for i in 0..n {
        out.push(mul(&out[i], p, w)?);
    }
    Ok(out)
}
fn bezier(knots: &[f64], degree: usize, n: usize) -> bool {
    n == degree + 1
        && knots[..n].iter().all(|&x| x == knots[degree])
        && knots[n..].iter().all(|&x| x == knots[n])
}
pub struct Report {
    pub surface: Surface,
    pub coefficient_work: usize,
    /// Whole stored surface versus S(P(u)+v*direction), in millimetres.
    pub position_error_upper_mm: f64,
}
/// Compose one original rational NURBS knot interval with a rational Bezier pcurve
/// and a constant transverse chart direction. Output parameters are normalized.
/// Every mapped chart control stays inside one immutable original knot
/// rectangle. Interval Bernstein arithmetic bounds the actual stored rounding
/// over the WHOLE collar. This bound concerns positions, never derivative jets.
pub fn construct(
    source: &Surface,
    pcurve: &Curve,
    direction: [f64; 2],
    tolerance_mm: f64,
    max_work: usize,
) -> Result<Report> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(refused("Need a positive finite stored-position tolerance"));
    }
    let Composition {
        lift,
        degree,
        sum,
        coefficient_work,
    } = compose(source, pcurve, direction, max_work)?;
    let mut work = Work {
        used: coefficient_work,
        max: max_work,
    };
    let midpoint = |x: I| x.lo * 0.5 + x.hi * 0.5;
    let mut weights = vec![vec![0.; sum + 1]; degree + 1];
    let mut points = vec![vec![vec![0.; 3]; sum + 1]; degree + 1];
    let mut residual = [0_f64; 4];
    let mut extent = [0_f64; 3];
    let mut lower_weight = f64::INFINITY;
    for i in 0..=degree {
        for j in 0..=sum {
            lower_weight = lower_weight.min(lift[3][i][j].lo);
            let weight = midpoint(lift[3][i][j]);
            if weight <= 0. || !weight.is_finite() || lower_weight <= 0. {
                return Err(refused("Positive mapped denominator is unresolved"));
            }
            weights[i][j] = weight;
            for k in 0..4 {
                let saved = if k == 3 {
                    I::point(weight)
                } else {
                    let point = midpoint(lift[k][i][j]) / weight;
                    if !point.is_finite() {
                        return Err(refused("Stored collar exceeds finite numeric range"));
                    }
                    points[i][j][k] = point;
                    extent[k] = extent[k].max(point.abs());
                    work.charge(1)?;
                    I::point(point).mul(I::point(weight))?
                };
                work.charge(1)?;
                let delta = lift[k][i][j].sub(saved)?;
                residual[k] = residual[k].max(delta.lo.abs()).max(delta.hi.abs());
            }
        }
    }
    let mut squared = I::point(0.);
    for k in 0..3 {
        work.charge(5)?;
        let bound = I::point(residual[k])
            .add(I::point(extent[k]).mul(I::point(residual[3]))?)?
            .div(I::point(lower_weight))?;
        squared = squared.add(bound.mul(bound)?)?;
    }
    let error = squared.hi.sqrt().next_up();
    if !error.is_finite() || error > tolerance_mm {
        return Err(refused(
            "Whole stored collar rounding exceeds the requested tolerance",
        ));
    }
    let surface = Surface {
        degree_u: degree,
        degree_v: sum,
        knots_u: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        knots_v: [vec![0.; sum + 1], vec![1.; sum + 1]].concat(),
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(Report {
        surface,
        coefficient_work: work.used,
        position_error_upper_mm: error,
    })
}

struct Composition {
    lift: Vec<Poly>,
    degree: usize,
    sum: usize,
    coefficient_work: usize,
}
// No rounded reference surface is used by the jet or G2 inspectors.
fn compose(
    source: &Surface,
    pcurve: &Curve,
    direction: [f64; 2],
    max_work: usize,
) -> Result<Composition> {
    compose_at_origin(source, pcurve, direction, max_work, [0.; 3])
}

// Homogeneous Cox-de Boor numerators on one ORIGINAL knot interval.
// No floating-point knot insertion or rounded extracted reference is used.
fn span_basis(
    map: &[Poly],
    axis: usize,
    knots: &[f64],
    degree: usize,
    span: usize,
    p: usize,
    work: &mut Work,
) -> Result<Vec<Poly>> {
    let mut previous = std::collections::BTreeMap::new();
    previous.insert(span, vec![vec![I::point(1.)]]);
    for depth in 1..=degree {
        let mut next = std::collections::BTreeMap::new();
        for i in span - depth..=span {
            let mut value = vec![vec![I::point(0.); depth + 1]; p * depth + 1];
            for (old, low, high, complement) in [
                (i, knots[i], knots[i + depth], false),
                (i + 1, knots[i + 1], knots[i + depth + 1], true),
            ] {
                if high == low {
                    continue;
                }
                let Some(basis) = previous.get(&old) else {
                    continue;
                };
                let denominator = I::point(high).sub(I::point(low))?;
                let mut factor = vec![vec![I::point(0.); 2]; p + 1];
                for a in 0..=p {
                    for b in 0..2 {
                        work.charge(5)?;
                        factor[a][b] = if complement {
                            I::point(high)
                                .mul(map[2][a][b])?
                                .sub(map[axis][a][b])?
                                .div(denominator)?
                        } else {
                            map[axis][a][b]
                                .sub(I::point(low).mul(map[2][a][b])?)?
                                .div(denominator)?
                        };
                    }
                }
                let term = mul(basis, &factor, work)?;
                for a in 0..value.len() {
                    for b in 0..value[0].len() {
                        work.charge(1)?;
                        value[a][b] = value[a][b].add(term[a][b])?;
                    }
                }
            }
            next.insert(i, value);
        }
        previous = next;
    }
    Ok((span - degree..=span)
        .map(|i| previous.remove(&i).unwrap())
        .collect())
}
fn original_span(s: &Surface, p: &Curve, direction: [f64; 2], axis: usize) -> Result<usize> {
    let (knots, degree, count) = if axis == 0 {
        (&s.knots_u, s.degree_u, s.control_points.len())
    } else {
        (&s.knots_v, s.degree_v, s.control_points[0].len())
    };
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for point in &p.control_points {
        let end = if direction[axis] == 0. {
            I::point(point[axis])
        } else if let Some(exact) =
            crate::exact_curve_segments::exact_add(point[axis], direction[axis])
        {
            I::point(exact)
        } else {
            I::point(point[axis]).add(I::point(direction[axis]))?
        };
        lo = lo.min(point[axis]).min(end.lo);
        hi = hi.max(point[axis]).max(end.hi);
    }
    (degree..count)
        .find(|&i| knots[i] < knots[i + 1] && knots[i] <= lo && hi <= knots[i + 1])
        .ok_or_else(|| refused("Complete collar crosses an original knot interval"))
}

fn compose_at_origin(
    source: &Surface,
    pcurve: &Curve,
    direction: [f64; 2],
    max_work: usize,
    origin: [f64; 3],
) -> Result<Composition> {
    source.validate()?;
    pcurve.validate()?;
    let (du, dv, p) = (source.degree_u, source.degree_v, pcurve.degree);
    let sum = du + dv;
    let degree = p.saturating_mul(sum);
    if source.periodic_u
        || source.periodic_v
        || pcurve.periodic
        || source.control_points[0][0].len() != 3
        || pcurve.control_points[0].len() != 2
        || degree > 16
        || sum > 16
        || !(1..=2000000).contains(&max_work)
        || direction.iter().any(|x| !x.is_finite())
        || !bezier(&pcurve.knots, p, pcurve.control_points.len())
    {
        return Err(refused(
            "Need bounded original spline charts and finite collar work",
        ));
    }
    let spans=[original_span(source,pcurve,direction,0)?,original_span(source,pcurve,direction,1)?];
    let offsets=[spans[0]-du,spans[1]-dv];
    let domains = [
        [source.knots_u[spans[0]],source.knots_u[spans[0]+1]],
        [source.knots_v[spans[1]],source.knots_v[spans[1]+1]],
    ];
    let mut work = Work {
        used: 0,
        max: max_work,
    };
    let mut map = vec![vec![vec![I::point(0.); 2]; p + 1]; 3];
    for (i, point) in pcurve.control_points.iter().enumerate() {
        for k in 0..2 {
            work.charge(2)?;
            let end = if direction[k] == 0. {
                I::point(point[k])
            } else if let Some(exact)=crate::exact_curve_segments::exact_add(point[k],direction[k]) {
                I::point(exact)
            } else {
                I::point(point[k]).add(I::point(direction[k]))?
            };
            if point[k] < domains[k][0]
                || point[k] > domains[k][1]
                || end.lo < domains[k][0]
                || end.hi > domains[k][1]
            {
                return Err(refused(
                    "Complete mapped collar is not inside the original chart",
                ));
            }
            for j in 0..2 {
                work.charge(2)?;
                let coordinate = if j == 0 { I::point(point[k]) } else { end };
                map[k][i][j] = coordinate.mul(I::point(pcurve.weights[i]))?;
            }
        }
        map[2][i] = vec![I::point(pcurve.weights[i]); 2];
    }
    let mut basis = Vec::new();
    for (axis, n) in [(0, du), (1, dv)] {
        let single=if axis==0 {bezier(&source.knots_u,du,source.control_points.len())}
            else {bezier(&source.knots_v,dv,source.control_points[0].len())};
        if !single {
            let knots=if axis==0 {&source.knots_u} else {&source.knots_v};
            basis.push(span_basis(&map,axis,knots,n,spans[axis],p,&mut work)?);
            continue;
        }
        let mut coordinate = map[axis].clone();
        let mut complement = map[2].clone();
        for i in 0..=p {
            for j in 0..2 {
                work.charge(5)?;
                coordinate[i][j] = coordinate[i][j]
                    .sub(I::point(domains[axis][0]).mul(map[2][i][j])?)?
                    .div(I::point(domains[axis][1]).sub(I::point(domains[axis][0]))?)?;
                complement[i][j] = map[2][i][j].sub(coordinate[i][j])?;
            }
        }
        let up = powers(&coordinate, n, &mut work)?;
        let down = powers(&complement, n, &mut work)?;
        let mut row = Vec::new();
        for i in 0..=n {
            let mut term = mul(&up[i], &down[n - i], &mut work)?;
            for values in &mut term {
                for x in values {
                    work.charge(1)?;
                    *x = x.mul(I::point(choose(n, i)))?;
                }
            }
            row.push(term);
        }
        basis.push(row);
    }
    let mut lift = vec![vec![vec![I::point(0.); sum + 1]; degree + 1]; 4];
    for i in 0..=du {
        for j in 0..=dv {
            let term = mul(&basis[0][i], &basis[1][j], &mut work)?;
            for k in 0..4 {
                work.charge(1)?;
                let cp = if k == 3 {
                    I::point(source.weights[i+offsets[0]][j+offsets[1]])
                } else {
                    let point = I::point(source.control_points[i+offsets[0]][j+offsets[1]][k]);
                    let centered = if origin[k] == 0. {
                        point
                    } else {
                        work.charge(1)?;
                        point.sub(I::point(origin[k]))?
                    };
                    centered.mul(I::point(source.weights[i+offsets[0]][j+offsets[1]]))?
                };
                for a in 0..=degree {
                    for b in 0..=sum {
                        work.charge(2)?;
                        lift[k][a][b] = lift[k][a][b].add(term[a][b].mul(cp)?)?;
                    }
                }
            }
        }
    }
    Ok(Composition {
        lift,
        degree,
        sum,
        coefficient_work: work.used,
    })
}
