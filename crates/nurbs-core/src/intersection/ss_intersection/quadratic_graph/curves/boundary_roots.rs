//! Original graph equation on an actual rational native-UV trim definition.
use super::*;
use arithmetic::P;
#[derive(Clone, Copy, Debug)]
pub struct GraphBoundaryRootLimits {
    pub max_operations: usize,
    pub max_depth: usize,
    pub clock_tolerance: f64,
    pub world_tolerance: f64,
}
impl Default for GraphBoundaryRootLimits {
    fn default() -> Self {
        Self {
            max_operations: 1000000,
            max_depth: 64,
            clock_tolerance: 1e-8,
            world_tolerance: 1e-7,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct GraphBoundaryRoot {
    /// Unit clock on the original complete clamped boundary definition.
    pub clock: [f64; 2],
    /// Outward transport, never a snapped representative parameter.
    pub boundary_parameter: [f64; 2],
    pub native_uv: [[[f64; 2]; 2]; 2],
    pub world: [[f64; 2]; 3],
    /// Sign of d(H_first-H_second)/d(boundary unit clock) at this simple root.
    pub crossing_sign: i8,
}
#[derive(Clone, Debug)]
pub struct QualifiedGraphBoundaryRoots {
    original: QualifiedQuadraticGraphSS,
    boundary: Curve,
    side: usize,
    limits: GraphBoundaryRootLimits,
    roots: Vec<GraphBoundaryRoot>,
    operations: usize,
}
impl QualifiedGraphBoundaryRoots {
    pub fn original(&self) -> &QualifiedQuadraticGraphSS {
        &self.original
    }
    pub fn boundary(&self) -> &Curve {
        &self.boundary
    }
    pub fn side(&self) -> usize {
        self.side
    }
    pub fn roots(&self) -> &[GraphBoundaryRoot] {
        &self.roots
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        self.original.reverify()?;
        let fresh =
            qualify_graph_boundary_roots(&self.original, &self.boundary, self.side, self.limits)?;
        check(
            fresh.roots == self.roots && fresh.operations == self.operations,
            "Original graph boundary root ownership changed",
        )
    }
}
fn value(p: &P, t: I, b: &mut Arithmetic) -> Result<I> {
    let hull = I::new(
        p.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
        p.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max),
    )?;
    let beta = b.sub(I::point(1.), t)?.intersect(0., 1.)?;
    let mut row = p.clone();
    for n in (1..row.len()).rev() {
        for i in 0..n {
            let a = b.mul(row[i], beta)?;
            let c = b.mul(row[i + 1], t)?;
            row[i] = b.add(a, c)?;
        }
    }
    row[0].intersect(hull.lo, hull.hi)
}
fn normalized(
    original: &QualifiedQuadraticGraphSS,
    unit: &Curve,
    side: usize,
    b: &mut Arithmetic,
) -> Result<Rational> {
    let mut r = expressions::actual(unit, b)?;
    for a in 0..2 {
        let d = original.data.domains[side][a];
        let span = b.sub(I::point(d[1]), I::point(d[0]))?;
        check(span.lo > 0., "Boundary native span positivity unresolved")?;
        let base = b.scale(&r.denominator, I::point(d[0]))?;
        for (p, base) in r.numerator[a].iter_mut().zip(base) {
            let delta = b.sub(*p, base)?;
            *p = b.div(delta, span)?;
        }
    }
    Ok(r)
}
fn equation(original: &QualifiedQuadraticGraphSS, uv: &Rational, b: &mut Arithmetic) -> Result<P> {
    let mut coefficients = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            b.tick(1)?;
            coefficients[i][j] = exact_add(
                original.data.heights[0][i][j],
                -original.data.heights[1][i][j],
            )
            .ok_or_else(|| crate::input("Original graph difference lost its exact identity"))?;
            check(
                i + j <= 2 || coefficients[i][j] == 0.,
                "Original difference is not quadratic",
            )?;
        }
    }
    let mut out = vec![I::point(0.); 2 * uv.degree() + 1];
    for i in 0..3 {
        for j in 0..3 - i {
            if coefficients[i][j] == 0. {
                continue;
            }
            let u = b.power(&uv.numerator[0], i)?;
            let v = b.power(&uv.numerator[1], j)?;
            let w = b.power(&uv.denominator, 2 - i - j)?;
            let uv = b.product(&u, &v)?;
            let term = b.product(&uv, &w)?;
            let term = b.scale(&term, I::point(coefficients[i][j]))?;
            out = b.sum(&out, &term)?;
        }
    }
    Ok(out)
}
fn enclosure(
    original: &QualifiedQuadraticGraphSS,
    uv: &Rational,
    clock: [f64; 2],
    b: &mut Arithmetic,
) -> Result<OriginalGraphSSStation> {
    let t = I::new(clock[0], clock[1])?;
    let denominator = value(&uv.denominator, t, b)?;
    let mut normalized = [I::point(0.); 2];
    for a in 0..2 {
        let numerator = value(&uv.numerator[a], t, b)?;
        normalized[a] = b.div(numerator, denominator)?.intersect(0., 1.)?;
    }
    let remaining = (b.max - b.used).min(4096);
    if remaining == 0 {
        return Err(crate::resource(
            "Original boundary root enclosure work exhausted",
        ));
    }
    let mut w = Work::new(remaining)?;
    let result = original.enclose(normalized, &mut w)?;
    b.tick(w.used)?;
    Ok(result)
}
/// Complete census of simple roots of the ORIGINAL quadratic graph difference
/// along one actual clamped positive-rational native-UV Bézier boundary.
/// Entire boundary membership in the original rectangle is required. A failed
/// census returns no partial owner; endpoint, multiple or coincident events
/// without a fresh existence/uniqueness proof remain explicit refusals.
pub fn qualify_graph_boundary_roots(
    original: &QualifiedQuadraticGraphSS,
    boundary: &Curve,
    side: usize,
    limits: GraphBoundaryRootLimits,
) -> Result<QualifiedGraphBoundaryRoots> {
    check(
        side < 2
            && (1..=1000000).contains(&limits.max_operations)
            && limits.max_depth <= 64
            && limits.clock_tolerance.is_finite()
            && limits.clock_tolerance > 0.
            && limits.clock_tolerance <= 1.
            && limits.world_tolerance.is_finite()
            && limits.world_tolerance > 0.,
        "Invalid original boundary root limits",
    )?;
    boundary.validate()?;
    check(
        (1..=8).contains(&boundary.degree)
            && !boundary.periodic
            && boundary.control_points.len() == boundary.degree + 1
            && boundary.control_points[0].len() == 2,
        "Boundary roots need one complete native-UV Bezier definition of degree1..8",
    )?;
    let domain = boundary.domain();
    check(
        boundary.knots[..=boundary.degree]
            .iter()
            .all(|k| *k == domain[0])
            && boundary.knots[boundary.degree + 1..]
                .iter()
                .all(|k| *k == domain[1]),
        "Boundary clock must be clamped",
    )?;
    check(
        original.kind() != QuadraticGraphSSKind::CoincidentPatch,
        "Coincident original supports need region overlap ownership",
    )?;
    let mut b = Arithmetic {
        used: 0,
        max: limits.max_operations,
    };
    b.tick(1)?;
    let mut unit = boundary.clone();
    unit.knots = vec![0.; unit.degree + 1];
    unit.knots.extend(vec![1.; unit.degree + 1]);
    proof::domain(&unit, original.data.domains[side], &mut b)?;
    let uv = normalized(original, &unit, side, &mut b)?;
    let polynomial = equation(original, &uv, &mut b)?;
    let degree = polynomial.len() - 1;
    let derivative = polynomial
        .windows(2)
        .map(|p| {
            let d = b.sub(p[1], p[0])?;
            b.mul(d, I::point(degree as f64))
        })
        .collect::<Result<P>>()?;
    let mut roots = Vec::new();
    let mut pending = vec![([0., 1.], false, 0)];
    while let Some((clock, exists, depth)) = pending.pop() {
        b.tick(1)?;
        let t = I::new(clock[0], clock[1])?;
        let f = value(&polynomial, t, &mut b)?;
        if f.lo > 0. || f.hi < 0. {
            continue;
        }
        let df = value(&derivative, t, &mut b)?;
        if df.lo > 0. || df.hi < 0. {
            let mid = b.midpoint(t)?;
            let fm = value(&polynomial, I::point(mid), &mut b)?;
            b.tick(1)?;
            let correction = fm.div_signed(df)?;
            let n = b.sub(I::point(mid), correction)?;
            if n.hi < clock[0] || n.lo > clock[1] {
                continue;
            }
            let root = [n.lo.max(clock[0]), n.hi.min(clock[1])];
            let proven = exists || (n.lo > clock[0] && n.hi < clock[1]);
            if proven {
                let station = enclosure(original, &uv, root, &mut b)?;
                let widths = station
                    .world
                    .map(|p| b.sub(I::point(p[1]), I::point(p[0])))
                    .into_iter()
                    .collect::<Result<Vec<_>>>()?;
                let world_width = b.norm_upper(&widths)?;
                let width = b.sub(I::point(root[1]), I::point(root[0]))?.hi;
                if width <= limits.clock_tolerance && world_width <= limits.world_tolerance {
                    let span = b.sub(I::point(domain[1]), I::point(domain[0]))?;
                    let parameter = b.mul(I::new(root[0], root[1])?, span)?;
                    let parameter = b
                        .add(I::point(domain[0]), parameter)?
                        .intersect(domain[0], domain[1])?;
                    roots.push(GraphBoundaryRoot {
                        clock: root,
                        boundary_parameter: [parameter.lo, parameter.hi],
                        native_uv: [station.first_uv, station.second_uv],
                        world: station.world,
                        crossing_sign: if df.lo > 0. { 1 } else { -1 },
                    });
                    continue;
                }
                if depth < limits.max_depth && root[0] < root[1] && root != clock {
                    pending.push((root, true, depth + 1));
                    continue;
                }
            }
        }
        check(
            depth < limits.max_depth,
            "Original boundary root census unresolved at depth limit",
        )?;
        b.tick(3)?;
        let split = clock[0] + (clock[1] - clock[0]) * 0.4375;
        check(
            clock[0] < split && split < clock[1],
            "Original boundary root parameter precision exhausted",
        )?;
        pending.push(([split, clock[1]], false, depth + 1));
        pending.push(([clock[0], split], false, depth + 1));
    }
    roots.sort_by(|a, c| a.clock[0].total_cmp(&c.clock[0]));
    check(
        roots.windows(2).all(|p| p[0].clock[1] < p[1].clock[0]),
        "Boundary root clocks are not distinctly owned",
    )?;
    Ok(QualifiedGraphBoundaryRoots {
        original: original.clone(),
        boundary: boundary.clone(),
        side,
        limits,
        roots,
        operations: b.used,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_root_bounds_actual_weights_clocks_and_original_sources_are_replayed() {
        let first = crate::polynomial::graph(
            [0., 1., 0., 1.],
            &[vec![0.4375, -1., 1.], vec![-1., 0., 0.], vec![1., 0., 0.]],
        )
        .unwrap();
        let second =
            crate::polynomial::graph([0., 1., 0., 1.], &[vec![0., 0.], vec![0., 0.]]).unwrap();
        let sources = FiberSources::new(&first, &second).unwrap();
        let original = qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
        let boundary = Curve::from_polyline(vec![vec![0., 0.5625], vec![1., 0.5625]]).unwrap();
        let owner =
            qualify_graph_boundary_roots(&original, &boundary, 0, Default::default()).unwrap();
        let mut changed = owner.clone();
        changed.roots[0].world[0][0] += 1.;
        assert!(changed.reverify().is_err());
        let mut changed = owner.clone();
        changed.roots[0].clock = [0.125, 0.25];
        assert!(changed.reverify().is_err());
        let mut changed = owner.clone();
        changed.boundary.weights[1] = 2.;
        assert!(changed.reverify().is_err());
        let mut changed = owner.clone();
        for k in &mut changed.boundary.knots {
            *k += 1.;
        }
        assert!(changed.reverify().is_err());
        let mut a = first;
        let mut b = second;
        for s in [&mut a, &mut b] {
            for p in s.control_points.iter_mut().flatten() {
                p[2] += 1.;
            }
        }
        let sources = FiberSources::new(&a, &b).unwrap();
        let mut changed = owner;
        changed.original = qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
        assert!(changed.reverify().is_err());
    }
}
