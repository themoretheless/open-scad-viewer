//! Complete interval DP over no-Steiner quadrangulations of one simple even
//! polygon. Floating cost ranks proposals; exact original predicates gate them.
use super::*;
use projection::Projector;
pub(super) fn allowed(
    p: &mut Projector,
    n: usize,
    forbidden: &BTreeSet<(usize, usize)>,
) -> Result<Vec<Vec<bool>>> {
    let mut diagonal = vec![vec![false; n]; n];
    for i in 0..n {
        for j in i + 1..n {
            let valid = !forbidden.contains(&edge(i, j)) && p.diagonal(i, j)?;
            diagonal[i][j] = valid;
            diagonal[j][i] = valid;
        }
    }
    Ok(diagonal)
}
pub(super) fn solve(
    p: &mut Projector,
    points: &[[f64; 3]],
    forbidden: &BTreeSet<(usize, usize)>,
    work: &mut usize,
    max: usize,
) -> Result<Vec<[usize; 4]>> {
    let n = points.len();
    let diagonal = allowed(p, n, forbidden)?;
    let mut cost = vec![vec![f64::INFINITY; n]; n];
    let mut split = vec![vec![None; n]; n];
    for i in 0..n - 1 {
        if diagonal[i][i + 1] {
            cost[i][i + 1] = 0.;
        }
    }
    for size in (4..=n).step_by(2) {
        for i in 0..=n - size {
            let j = i + size - 1;
            if !diagonal[i][j] {
                continue;
            }
            for a in (i + 1..j).step_by(2) {
                for b in (a + 1..j).step_by(2) {
                    if *work == max {
                        return Err(error("Quad retile candidate allowance exhausted"));
                    }
                    *work += 1;
                    let base = cost[i][a] + cost[a][b] + cost[b][j];
                    if !base.is_finite() {
                        continue;
                    }
                    let q = [i, a, b, j];
                    if !p.quad(q)? {
                        continue;
                    }
                    let score = base
                        + (0..4)
                            .map(|k| {
                                let a = points[q[k]];
                                let b = points[q[(k + 1) % 4]];
                                (0..3).map(|c| (a[c] - b[c]).powi(2)).sum::<f64>()
                            })
                            .sum::<f64>();
                    if score < cost[i][j] {
                        cost[i][j] = score;
                        split[i][j] = Some((a, b));
                    }
                }
            }
        }
    }
    if !cost[0][n - 1].is_finite() {
        return Err(error(
            "No admitted original-boundary quadrangulation without new controls",
        ));
    }
    let mut todo = vec![(0, n - 1)];
    let mut faces = Vec::new();
    while let Some((i, j)) = todo.pop() {
        if j == i + 1 {
            continue;
        }
        let (a, b) = split[i][j].ok_or_else(|| error("Missing original quadrangulation split"))?;
        faces.push([i, a, b, j]);
        todo.extend([(i, a), (a, b), (b, j)]);
    }
    Ok(faces)
}
/// Independent replay: exact valid diagonals/quad triangulations, one oriented
/// disk with the literal original boundary, and no projected edge crossings.
pub(super) fn verify(
    p: &mut Projector,
    points: &[[f64; 3]],
    faces: &[[usize; 4]],
    forbidden: &BTreeSet<(usize, usize)>,
) -> Result<()> {
    let n = points.len();
    let boundary = p.boundary_count();
    if n < boundary
        || faces.len() != n - boundary / 2 - 1
        || faces.iter().flatten().any(|v| *v >= n)
    {
        return Err(error("Invalid retiled original disk size"));
    }
    let local = Cage {
        vertices: points.to_vec(),
        faces: faces.iter().map(|f| f.to_vec()).collect(),
    };
    local.validate()?;
    if region::counts(&local) != (1, 1, 1) {
        return Err(error(
            "Replacement quads are not one original-boundary disk",
        ));
    }
    let uses = local.build_edges();
    for (&(a, b), u) in &uses {
        if forbidden.contains(&edge(a, b)) || !p.diagonal(a, b)? || !p.clear_edge(a, b, n)? {
            return Err(error(
                "Replacement diagonal lacks original projected admission",
            ));
        }
        if u.len() == 1
            && !(a < boundary
                && b < boundary
                && ((a + 1) % boundary == b || (b + 1) % boundary == a))
        {
            return Err(error("Replacement has an invented boundary edge"));
        }
    }
    for i in 0..boundary {
        let a = i;
        let b = (i + 1) % boundary;
        if !uses
            .get(&edge(a, b))
            .is_some_and(|u| u.len() == 1 && u[0].1 == (a < b))
        {
            return Err(error(
                "Replacement must retain every oriented boundary edge",
            ));
        }
    }
    for &f in faces {
        if !p.quad(f)? {
            return Err(error(
                "Replacement quad is projected-degenerate or inverted",
            ));
        }
    }
    let edges = uses.keys().copied().collect::<Vec<_>>();
    for (i, &(a, b)) in edges.iter().enumerate() {
        for &(c, d) in &edges[i + 1..] {
            if [a, b].iter().any(|v| *v == c || *v == d) {
                continue;
            }
            if p.intersects(a, b, c, d)? {
                return Err(error(
                    "Replacement control edges cross in the original projection",
                ));
            }
        }
    }
    Ok(())
}
