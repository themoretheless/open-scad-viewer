//! Exact diagonal elimination for two polynomial graph carriers. Complete
//! TWO-dimensional coverage implies complete coverage of the original product:
//! the identical injective affine XY frame forces equal normalized parameters.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
type Net = Vec<Vec<Q>>;
type Cell = [[Q; 2]; 2];
mod cells;
use cells::{cut, diagnostic, full_cell};
mod multispan;
mod replay;
#[derive(Clone, Debug, PartialEq)]
enum Node {
    Chart(usize),
    Image { index: usize, axis: usize },
    Excluded,
    Split { axis: usize, children: [usize; 2] },
}
#[derive(Clone, Debug)]
pub(super) struct Proof {
    nodes: Vec<Node>,
    patch_roots: usize,
}
fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::input("Nonfinite graph coefficient"))
}
fn admit(s: &Surface) -> Result<()> {
    s.validate()?;
    check(
        !s.periodic_u
            && !s.periodic_v
            && (1..=8).contains(&s.degree_u)
            && (1..=8).contains(&s.degree_v)
            && s.control_points.len() == s.degree_u + 1
            && s.control_points[0].len() == s.degree_v + 1
            && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Need one spatial degree1..8 polynomial graph span",
    )?;
    for (d, k) in [(s.degree_u, &s.knots_u), (s.degree_v, &s.knots_v)] {
        check(
            k.len() == 2 * (d + 1)
                && k[..=d].iter().all(|v| *v == k[d])
                && k[d + 1..].iter().all(|v| *v == k[d + 1]),
            "Need full clamped graph domains",
        )?;
    }
    let w = s.weights[0][0];
    check(
        w > 0. && s.weights.iter().flatten().all(|&v| v == w),
        "Graph reduction requires positive constant weights",
    )
}
fn frame(s: &Surface) -> Result<[Q; 4]> {
    admit(s)?;
    let x = q(s.control_points[0][0][0])?;
    let y = q(s.control_points[0][0][1])?;
    let dx = q(s.control_points[s.degree_u][0][0])? - &x;
    let dy = q(s.control_points[0][s.degree_v][1])? - &y;
    check(
        !dx.is_zero() && !dy.is_zero(),
        "Graph projection must be globally injective",
    )?;
    for i in 0..=s.degree_u {
        for j in 0..=s.degree_v {
            check(
                q(s.control_points[i][j][0])? == &x + &dx * Q::new(i.into(), s.degree_u.into())
                    && q(s.control_points[i][j][1])?
                        == &y + &dy * Q::new(j.into(), s.degree_v.into()),
                "Original graph XY affine coefficient identity failed",
            )?;
        }
    }
    Ok([x, y, dx, dy])
}
fn elevate(n: &Net, axis: usize) -> Net {
    let (du, dv) = (n.len() - 1, n[0].len() - 1);
    let mut out =
        vec![vec![Q::zero(); dv + 1 + usize::from(axis == 1)]; du + 1 + usize::from(axis == 0)];
    for i in 0..out.len() {
        for j in 0..out[0].len() {
            let k = if axis == 0 { i } else { j };
            let d = if axis == 0 { du } else { dv };
            if k > 0 {
                let a = if axis == 0 {
                    &n[i - 1][j]
                } else {
                    &n[i][j - 1]
                };
                out[i][j] += a * Q::new(k.into(), (d + 1).into());
            }
            if k <= d {
                let a = &n[i][j];
                out[i][j] += a * Q::new((d + 1 - k).into(), (d + 1).into());
            }
        }
    }
    out
}
fn derive(source: &FiberSources) -> Result<Net> {
    let (a, b) = (source.first(), source.second());
    check(
        frame(a)? == frame(b)?,
        "Different original affine graph frames",
    )?;
    let mut nets = vec![];
    for s in [a, b] {
        let mut n = s
            .control_points
            .iter()
            .map(|row| row.iter().map(|p| q(p[2])).collect::<Result<Vec<_>>>())
            .collect::<Result<Vec<_>>>()?;
        while n.len() <= a.degree_u.max(b.degree_u) {
            n = elevate(&n, 0);
        }
        while n[0].len() <= a.degree_v.max(b.degree_v) {
            n = elevate(&n, 1);
        }
        nets.push(n);
    }
    Ok(nets[1]
        .iter()
        .zip(&nets[0])
        .map(|(b, a)| b.iter().zip(a).map(|(b, a)| b - a).collect())
        .collect())
}
fn split(n: &Net, axis: usize) -> [Net; 2] {
    let (nu, nv) = (n.len(), n[0].len());
    let mut out = [vec![vec![Q::zero(); nv]; nu], vec![vec![Q::zero(); nv]; nu]];
    for line in 0..if axis == 0 { nv } else { nu } {
        let count = if axis == 0 { nu } else { nv };
        let mut p = (0..count)
            .map(|k| {
                if axis == 0 {
                    n[k][line].clone()
                } else {
                    n[line][k].clone()
                }
            })
            .collect::<Vec<_>>();
        for step in 0..count {
            if axis == 0 {
                out[0][step][line] = p[0].clone();
                out[1][count - step - 1][line] = p[count - step - 1].clone();
            } else {
                out[0][line][step] = p[0].clone();
                out[1][line][count - step - 1] = p[count - step - 1].clone();
            }
            for k in 0..count - step - 1 {
                p[k] = (&p[k] + &p[k + 1]) / Q::from_integer(2.into());
            }
        }
    }
    out
}
fn excluded(n: &Net) -> bool {
    n.iter().flatten().all(|v| v.is_positive()) || n.iter().flatten().all(|v| v.is_negative())
}
fn contained(cell: &Cell, domain: Box4, f: &QualifiedFiber) -> Result<bool> {
    let actual = f.domain();
    for k in 0..4 {
        let lo = q(domain[k][0])?;
        let width = q(domain[k][1])? - &lo;
        if &lo + &width * &cell[k % 2][0] < q(actual[k][0])?
            || &lo + &width * &cell[k % 2][1] > q(actual[k][1])?
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn root_image(cell: &Cell, n: &Net, axis: usize) -> Option<[[Q; 2]; 2]> {
    if axis >= 2 {
        return None;
    }
    let count = if axis == 0 { n.len() } else { n[0].len() };
    let lines = if axis == 0 { n[0].len() } else { n.len() };
    let mut derivatives = vec![];
    let mut centers = vec![];
    for line in 0..lines {
        let mut p = (0..count)
            .map(|k| {
                if axis == 0 {
                    n[k][line].clone()
                } else {
                    n[line][k].clone()
                }
            })
            .collect::<Vec<_>>();
        for k in 0..count - 1 {
            derivatives.push((&p[k + 1] - &p[k]) * Q::from_integer((count - 1).into()));
        }
        for level in 1..count {
            for k in 0..count - level {
                p[k] = (&p[k] + &p[k + 1]) / Q::from_integer(2.into());
            }
        }
        centers.push(p[0].clone());
    }
    let dlo = derivatives.iter().min()?.clone();
    let dhi = derivatives.iter().max()?.clone();
    if dlo <= Q::zero() && dhi >= Q::zero() {
        return None;
    }
    let flo = centers.iter().min()?;
    let fhi = centers.iter().max()?;
    let quotients = [flo / &dlo, flo / &dhi, fhi / &dlo, fhi / &dhi];
    let half = Q::new(1.into(), 2.into());
    let lo = Q::zero().max(&half - quotients.iter().max()?);
    let hi = Q::from_integer(1.into()).min(&half - quotients.iter().min()?);
    let mut roots = cell.clone();
    let width = &roots[axis][1] - &roots[axis][0];
    roots[axis] = [&roots[axis][0] + &width * lo, &roots[axis][0] + &width * hi];
    Some(roots)
}
fn contained_image(cell: &[[Q; 2]; 2], domain: Box4, f: &QualifiedFiber) -> Result<bool> {
    if cell.iter().any(|v| v[0] > v[1]) {
        return Ok(true);
    }
    let actual = f.domain();
    for k in 0..4 {
        let lo = q(domain[k][0])?;
        let width = q(domain[k][1])? - &lo;
        if &lo + &width * &cell[k % 2][0] < q(actual[k][0])?
            || &lo + &width * &cell[k % 2][1] > q(actual[k][1])?
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn derive_patches(source: &FiberSources, work: &mut Work) -> Result<Vec<(Cell, Net)>> {
    if let Ok(net) = derive(source) {
        return Ok(vec![(full_cell(), net)]);
    }
    multispan::derive(source, work)
}
fn tick(work: &mut Work) -> Result<()> {
    check(work.used < work.max, "Exact reduced graph work exhausted")?;
    work.used += 1;
    Ok(())
}
fn report(cells: usize, excluded_cells: usize, rectangles: usize) -> FiberCoverageReport {
    FiberCoverageReport {
        status: FiberCoverageStatus::Complete,
        reason: "complete-original-graph-reduction",
        cells,
        excluded_cells,
        rectangles,
        requested_entire_surfaces: true,
        unresolved_box: None,
    }
}
/// None means this exact reduction family does not apply. A failed admitted
/// reduction reports unresolved coverage; it never becomes an empty root set.
pub(super) fn construct(
    fibers: &[QualifiedFiber],
    limits: CoverageLimits,
) -> Result<Option<MixedFiberCoverageResult>> {
    limits.validate()?;
    if fibers.is_empty() {
        return Ok(None);
    }
    let domain = natural(fibers[0].source());
    validate(fibers, domain)?;
    let mut work = Work {
        used: 0,
        max: limits.max_rectangles,
    };
    let patches = match derive_patches(fibers[0].source(), &mut work) {
        Ok(patches) => patches,
        Err(error) if error.code == crate::RESOURCE_LIMIT => return Err(error),
        Err(_) => return Ok(None),
    };
    if patches.len() > limits.max_cells {
        return Err(crate::resource(
            "Original graph knot census exceeds cell budget",
        ));
    }
    let patch_roots = patches.len();
    let mut r = MixedFiberCoverageResult {
        coverage: None,
        report: report(1, 0, 0),
    };
    r.report.status = FiberCoverageStatus::Unresolved;
    r.report.reason = "reduced-graph-unproved";
    for f in fibers {
        if replay_fiber(f, &mut work).is_err() {
            r.report.rectangles = work.used;
            return Ok(Some(r));
        }
    }
    let mut nodes = vec![Node::Excluded; patch_roots];
    let mut pending: Vec<_> = patches
        .into_iter()
        .enumerate()
        .map(|(i, (cell, net))| (i, cell, net, 0))
        .collect();
    let mut excludes = 0;
    let mut covered = 0;
    while let Some((id, cell, net, depth)) = pending.pop() {
        if tick(&mut work).is_err() {
            r.report.reason = "reduced-graph-work-budget";
            r.report.unresolved_box = Some(diagnostic(&cell, domain)?);
            break;
        }
        let mut owner = None;
        for (i, f) in fibers.iter().enumerate() {
            if contained(&cell, domain, f)? {
                owner = Some(i);
                break;
            }
        }
        if let Some(index) = owner {
            nodes[id] = Node::Chart(index);
            covered += 1;
            continue;
        }
        let mut image_owner = None;
        for axis in 0..2 {
            if let Some(roots) = root_image(&cell, &net, axis) {
                for (index, f) in fibers.iter().enumerate() {
                    if contained_image(&roots, domain, f)? {
                        image_owner = Some((index, axis));
                        break;
                    }
                }
            }
            if image_owner.is_some() {
                break;
            }
        }
        if let Some((index, axis)) = image_owner {
            nodes[id] = Node::Image { index, axis };
            covered += 1;
            continue;
        }
        if excluded(&net) {
            nodes[id] = Node::Excluded;
            excludes += 1;
            continue;
        }
        if depth == limits.max_depth || nodes.len() + 2 > limits.max_cells {
            r.report.reason = "reduced-graph-subdivision-budget";
            // Diagnostic cell in ORIGINAL native coordinates, rounded outward.
            r.report.unresolved_box = Some(diagnostic(&cell, domain)?);
            break;
        }
        let axis = usize::from(&cell[1][1] - &cell[1][0] > &cell[0][1] - &cell[0][0]);
        let cells = cut(&cell, axis)?;
        let nets = split(&net, axis);
        let children = [nodes.len(), nodes.len() + 1];
        nodes.extend([Node::Excluded, Node::Excluded]);
        nodes[id] = Node::Split { axis, children };
        for k in (0..2).rev() {
            pending.push((children[k], cells[k].clone(), nets[k].clone(), depth + 1));
        }
    }
    r.report.cells = nodes.len();
    r.report.excluded_cells = excludes;
    r.report.rectangles = work.used;
    if pending.is_empty() && r.report.unresolved_box.is_none() && covered > 0 {
        r.report = report(nodes.len(), excludes, work.used);
        r.coverage = Some(QualifiedMixedFiberCoverage {
            fibers: fibers.to_vec(),
            domain,
            nodes: vec![],
            limits,
            report: r.report.clone(),
            reduction: Some(Proof { nodes, patch_roots }),
        });
    }
    Ok(Some(r))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_reduction_replay_rejects_missing_cells_and_forged_exclusions() {
        let mut a = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![vec![0.; 3]; 3]; 3],
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let q = [0.25, -0.25, 0.25];
        for i in 0..3 {
            for j in 0..3 {
                a.control_points[i][j] = vec![i as f64 / 2., j as f64 / 2., 0.];
            }
        }
        let mut b = a.clone();
        for i in 0..3 {
            for j in 0..3 {
                b.control_points[i][j][2] = q[i] + q[j] - 0.0625;
            }
        }
        let source = super::super::super::super::FiberSources::new(&a, &b).unwrap();
        let owner = crate::ss_intersection::construct_qualified_closed_ss_branches(
            &source,
            Default::default(),
        )
        .unwrap();
        let coverage = owner.owner().proof().coverage();
        coverage.reverify().unwrap();
        assert!(coverage.reduction.is_some());
        let mut bad = coverage.clone();
        bad.reduction.as_mut().unwrap().nodes[0] = Node::Excluded;
        assert!(bad.reverify().is_err());
        let mut bad = coverage.clone();
        bad.reduction.as_mut().unwrap().nodes[0] = Node::Chart(usize::MAX);
        assert!(bad.reverify().is_err());
        let mut bad = coverage.clone();
        bad.reduction.as_mut().unwrap().nodes.pop();
        assert!(bad.reverify().is_err());
        let mut bad = coverage.clone();
        bad.report.rectangles += 1;
        assert!(bad.reverify().is_err());
    }
}
