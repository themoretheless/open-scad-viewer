//! Support plan: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn plan(original: &Arc<OwnedOriginalPeriodicWires>, max: usize) -> Result<Plan> {
    if !(1..=1000000).contains(&max) {
        return Err(refused("Invalid complete natural boundary work allowance"));
    }
    super::super::periodic_replay::verify_arc(original, |x| x.reverify_source())?;
    let supporting = original.surface();
    let cover = supporting.pieces();
    let s = supporting.surface();
    if s.control_points.iter().flatten().any(|p| p.len() != 3) {
        return Err(refused(
            "Natural support world curves need an actual three-dimensional source surface",
        ));
    }
    let chart = cover.chart();
    let groups: Vec<Vec<(usize, usize, bool)>> = match cover.wrapped() {
        [false, false] => vec![vec![
            (1, 0, false),
            (0, 1, false),
            (1, 1, true),
            (0, 0, true),
        ]],
        [true, false] => vec![vec![(1, 0, false)], vec![(1, 1, true)]],
        [false, true] => vec![vec![(0, 0, true)], vec![(0, 1, false)]],
        [true, true] => vec![],
    };
    let mut p = Plan {
        sources: cover.sources().to_vec(),
        uses: original.source_uses().to_vec(),
        roles: Vec::new(),
        boundaries: Vec::new(),
        work: 0,
    };
    for wire in 0..p.uses.len() {
        bump(&mut p.work, max)?;
        p.roles
            .push(OriginalPeriodicSupportWireRole::Authored { wire });
    }
    for group in groups {
        let wire = p.uses.len();
        let mut uses = Vec::new();
        for (axis, end, reversed) in group {
            bump(&mut p.work, max)?;
            let varying = 1 - axis;
            let domain = chart[varying];
            let mut a = [0.; 2];
            a[axis] = chart[axis][end];
            a[varying] = domain[0];
            let mut b = a;
            b[varying] = domain[1];
            let curve = Curve {
                degree: 1,
                knots: vec![domain[0], domain[0], domain[1], domain[1]],
                control_points: vec![a.to_vec(), b.to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            };
            curve.validate()?;
            let source = p.sources.len();
            p.sources.push(curve);
            uses.push(OriginalPeriodicSourceUse { source, reversed });
            p.boundaries.push(OriginalPeriodicNaturalBoundary {
                fixed_axis: axis,
                end,
                source,
                wire,
                reversed,
                level: ratio(&exact(chart[axis][end])?),
                native_domain: [ratio(&exact(domain[0])?), ratio(&exact(domain[1])?)],
                world: world_curve(s, axis, end, &mut p.work, max)?,
            });
        }
        p.uses.push(uses);
        p.roles
            .push(OriginalPeriodicSupportWireRole::NaturalSupport);
    }
    if p.sources.len() > 32 || p.uses.len() > 32 {
        return Err(refused(
            "Authored plus natural support sources exceed the complete periodic census allowance",
        ));
    }
    Ok(p)
}

pub(super) fn world_curve(
    s: &Surface,
    axis: usize,
    end: usize,
    work: &mut usize,
    max: usize,
) -> Result<OriginalNaturalBoundaryCurve> {
    let degree = [s.degree_u, s.degree_v];
    let controls = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    let fixed = knots[axis]
        .iter()
        .copied()
        .map(exact)
        .collect::<Result<Vec<_>>>()?;
    let level = fixed[if end == 0 {
        degree[axis]
    } else {
        controls[axis]
    }]
    .clone();
    let values = basis(degree[axis], &fixed, &level, work, max)?;
    let varying = 1 - axis;
    let mut net = Vec::new();
    for j in 0..controls[varying] {
        let mut h = std::array::from_fn::<_, 4, _>(|_| Q::zero());
        for (i, b) in values.iter().enumerate() {
            bump(work, max)?;
            let (u, v) = if axis == 0 { (i, j) } else { (j, i) };
            let w = exact(s.weights[u][v])?;
            let bw = b * w;
            for c in 0..3 {
                h[c] += &bw * exact(s.control_points[u][v][c])?;
            }
            h[3] += bw;
        }
        if h[3] <= Q::zero() {
            return Err(refused(
                "Natural boundary world control has nonpositive exact weight",
            ));
        }
        net.push(h.map(|q| ratio(&q)));
    }
    Ok(OriginalNaturalBoundaryCurve {
        degree: degree[varying],
        knots: knots[varying]
            .iter()
            .copied()
            .map(exact)
            .collect::<Result<Vec<_>>>()?
            .iter()
            .map(ratio)
            .collect(),
        controls: net,
    })
}

/// Closed native-domain endpoint convention: right limit at the lower bound,
/// left limit at the upper bound. Repeated zero spans do not become 0/0.
pub(super) fn basis(degree: usize, knots: &[Q], t: &Q, work: &mut usize, max: usize) -> Result<Vec<Q>> {
    if knots.len() < degree + 2 || knots.windows(2).any(|k| k[0] > k[1]) {
        return Err(refused("Invalid original natural boundary knot vector"));
    }
    let n = knots.len() - degree - 1;
    if degree >= n || t < &knots[degree] || t > &knots[n] || knots[degree] >= knots[n] {
        return Err(refused(
            "Natural boundary query is outside the original active knot domain",
        ));
    }
    let span = if t == &knots[n] {
        (degree..n)
            .rev()
            .find(|&i| knots[i] < *t && *t <= knots[i + 1])
    } else {
        (degree..n).find(|&i| knots[i] <= *t && *t < knots[i + 1])
    }
    .ok_or_else(|| refused("Natural boundary has no nonempty original endpoint span"))?;
    let mut values = vec![Q::zero(); knots.len() - 1];
    values[span] = Q::one();
    for order in 1..=degree {
        let mut next = Vec::new();
        for i in 0..values.len() - 1 {
            bump(work, max)?;
            let mut v = Q::zero();
            let a = &knots[i + order] - &knots[i];
            if a > Q::zero() {
                v += (t - &knots[i]) / a * &values[i];
            }
            let b = &knots[i + order + 1] - &knots[i + 1];
            if b > Q::zero() {
                v += (&knots[i + order + 1] - t) / b * &values[i + 1];
            }
            next.push(v);
        }
        values = next;
    }
    if values.len() != n
        || values.iter().any(|v| v < &Q::zero())
        || values.iter().cloned().sum::<Q>() != Q::one()
    {
        return Err(refused(
            "Exact natural boundary basis does not partition the original source",
        ));
    }
    Ok(values)
}
