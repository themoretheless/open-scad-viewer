//! Orientation-independent even/odd regions with a certified Jordan-loop forest.
//! Original curves, weights and clocks are never reversed or reparameterized.
use crate::{
    Result, check,
    curve::Curve,
    curve_distance,
    trim_domain::{Location, TrimDomain},
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub max_loops: usize,
    pub max_curves: usize,
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_loops: 16,
            max_curves: 256,
            pairs: 100000,
            cells: 100000,
            domain_cells: 1000000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub parent: Option<usize>,
    pub depth: usize,
    pub original_winding: i32,
    /// Canonical material traversal: even depth CCW, odd depth CW. Reverse
    /// coedge order AND directions; never change any authored curve coefficients.
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component {
    pub outer: usize,
    pub holes: Vec<usize>,
}
pub struct Report {
    pub proven: bool,
    pub reason: &'static str,
    pub loops: Vec<super::Report>,
    /// inside[i][j] certifies the entire boundary of i lies inside j.
    pub inside: Vec<Vec<Option<bool>>>,
    pub nodes: Option<Vec<Node>>,
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
    pub problem_loops: Option<[usize; 2]>,
}
pub struct Classification {
    pub location: Location,
    pub containing_loops: Vec<usize>,
    pub cells: usize,
    pub unresolved_loops: Vec<usize>,
}
pub struct Domain {
    loops: Vec<Vec<Curve>>,
    tolerance: f64,
    limits: Limits,
    proof: Report,
    components: Vec<Component>,
}
impl Domain {
    pub fn loops(&self) -> &[Vec<Curve>] {
        &self.loops
    }
    pub fn proof(&self) -> &Report {
        &self.proof
    }
    pub fn nodes(&self) -> &[Node] {
        self.proof
            .nodes
            .as_ref()
            .expect("published forest is proven")
    }
    pub fn components(&self) -> &[Component] {
        &self.components
    }
    /// Coedge indices/directions give canonical material orientation without
    /// rounding a reversed knot vector or altering an original UV definition.
    pub fn coedges(&self, index: usize) -> Result<Vec<(usize, bool)>> {
        let c = self
            .loops
            .get(index)
            .ok_or_else(|| crate::input("Loop index outside original forest"))?;
        let reverse = self.nodes()[index].reversed;
        Ok(if reverse {
            (0..c.len()).rev().map(|i| (i, true)).collect()
        } else {
            (0..c.len()).map(|i| (i, false)).collect()
        })
    }
    /// Even/odd material membership of the ACTUAL supplied UV loops. Native
    /// interval winding classifies every loop on the WHOLE rectangle; limits or
    /// boundary bands keep the complete result unresolved, never a partial parity.
    pub fn classify(&self, rectangle: [[f64; 2]; 2], max_cells: usize) -> Result<Classification> {
        check(
            (1..=1000000).contains(&max_cells)
                && rectangle
                    .iter()
                    .all(|r| r.iter().all(|x| x.is_finite()) && r[0] <= r[1]),
            "Choose a finite rectangle and bounded compound winding work",
        )?;
        let mut out = Classification {
            location: Location::Unresolved,
            containing_loops: vec![],
            cells: 0,
            unresolved_loops: vec![],
        };
        for (i, l) in self.loops.iter().enumerate() {
            if out.cells == max_cells {
                out.unresolved_loops.extend(i..self.loops.len());
                break;
            }
            let d = TrimDomain::new(std::slice::from_ref(l), self.tolerance)?;
            let r = d.classify(rectangle, (max_cells - out.cells).min(100000))?;
            out.cells += r.cells;
            match r.location {
                Location::Inside => out.containing_loops.push(i),
                Location::Outside => {}
                Location::Unresolved => out.unresolved_loops.push(i),
            }
        }
        if out.unresolved_loops.is_empty() {
            out.location = if out.containing_loops.len() % 2 == 1 {
                Location::Inside
            } else {
                Location::Outside
            };
        }
        Ok(out)
    }
    pub fn reverify(&self) -> Result<()> {
        let r = inspect(&self.loops, self.tolerance, self.limits)?;
        check(
            r.proven && r.nodes == self.proof.nodes && r.inside == self.proof.inside,
            "Nested region differs from fresh original-loop qualification",
        )
    }
}
fn validate(loops: &[Vec<Curve>], tolerance: f64, l: Limits) -> Result<()> {
    check(
        (1..=64).contains(&l.max_loops)
            && !loops.is_empty()
            && loops.len() <= l.max_loops
            && (2..=256).contains(&l.max_curves)
            && loops.iter().all(|v| (2..=l.max_curves).contains(&v.len()))
            && loops.iter().map(Vec::len).sum::<usize>() <= l.max_curves
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&l.pairs)
            && (1..=100000).contains(&l.cells)
            && (1..=1000000).contains(&l.domain_cells),
        "Choose bounded nesting work, positive UV tolerance and finite loop/curve counts",
    )?;
    // Validate EVERY later input before an easy loop or exhausted budget can
    // hide malformed geometry. No caller-positive proof reports are accepted.
    for c in loops.iter().flatten() {
        c.validate()?;
        check(
            c.control_points[0].len() == 2 && c.weights.iter().all(|w| *w > 0.),
            "Nested UV loops require positive-weight 2D curves",
        )?;
    }
    Ok(())
}
fn forest(out: &Report) -> Option<Vec<Node>> {
    let n = out.inside.len();
    let mut nodes = vec![];
    let yes = |i: usize, j: usize| out.inside[i][j] == Some(true);
    for i in 0..n {
        for j in 0..n {
            if i != j && yes(i, j) {
                if yes(j, i) {
                    return None;
                }
                for k in 0..n {
                    if yes(j, k) && !yes(i, k) {
                        return None;
                    }
                }
            }
        }
        let containers: Vec<_> = (0..n).filter(|&j| yes(i, j)).collect();
        let parent = if containers.is_empty() {
            None
        } else {
            let closest: Vec<_> = containers
                .iter()
                .copied()
                .filter(|&j| containers.iter().all(|&k| j == k || yes(j, k)))
                .collect();
            if closest.len() != 1 {
                return None;
            }
            Some(closest[0])
        };
        let winding = out.loops[i].winding[0]?;
        let depth = containers.len();
        nodes.push(Node {
            parent,
            depth,
            original_winding: winding,
            reversed: winding != if depth % 2 == 0 { 1 } else { -1 },
        });
    }
    if nodes.iter().any(|node| {
        node.parent
            .is_some_and(|p| nodes[p].depth + 1 != node.depth)
    }) {
        return None;
    }
    Some(nodes)
}
/// Prove all input loops simple and mutually separated, then classify one EXACT
/// endpoint of each connected boundary against every other Jordan domain. Full
/// separation makes that winding constant on the whole boundary; Jordan nesting
/// gives containment of the bounded interiors. The complete relation must form
/// a transitive laminar forest before any depth, hole or component is published.
/// Touching/crossing/limits remain unproven, not invented intersection diagnoses.
pub fn inspect(loops: &[Vec<Curve>], tolerance: f64, limits: Limits) -> Result<Report> {
    validate(loops, tolerance, limits)?;
    let n = loops.len();
    let mut out = Report {
        proven: false,
        reason: "loop-simplicity-unproven",
        loops: vec![],
        inside: vec![vec![None; n]; n],
        nodes: None,
        pairs: 0,
        cells: 0,
        domain_cells: 0,
        problem_loops: None,
    };
    for (i, l) in loops.iter().enumerate() {
        if out.pairs == limits.pairs
            || out.cells == limits.cells
            || out.domain_cells == limits.domain_cells
        {
            out.problem_loops = Some([i, i]);
            return Ok(out);
        }
        let audit = super::inspect(
            std::slice::from_ref(l),
            tolerance,
            limits.pairs - out.pairs,
            ((limits.cells - out.cells) / (n - i + 1)).max(1),
            ((limits.domain_cells - out.domain_cells) / (n - i + 1)).max(1),
        )?;
        out.pairs += audit.pairs;
        out.cells += audit.cells;
        out.domain_cells += audit.domain_cells;
        let valid = audit.valid == Some(true);
        out.loops.push(audit);
        if !valid {
            out.problem_loops = Some([i, i]);
            return Ok(out);
        }
        out.inside[i][i] = Some(false);
    }
    out.reason = "loop-separation-unproven";
    for a in 0..n {
        for b in a + 1..n {
            for ca in &loops[a] {
                for cb in &loops[b] {
                    if out.pairs == limits.pairs || out.cells == limits.cells {
                        out.problem_loops = Some([a, b]);
                        return Ok(out);
                    }
                    // Every pair is mandatory. Let a difficult pair use the
                    // remaining shared allowance; charge its ACTUAL visits.
                    // Fixed per-pair quotas can refuse a valid curved pair
                    // while leaving most of the global allowance unused.
                    let quota = limits.cells - out.cells;
                    out.pairs += 1;
                    let r = match curve_distance::prove_separation(ca, cb, tolerance, quota) {
                        Ok(r) => r,
                        Err(e) if e.code == "NURBS_RESOURCE_LIMIT" => {
                            out.cells += quota;
                            out.problem_loops = Some([a, b]);
                            return Ok(out);
                        }
                        Err(e) => return Err(e),
                    };
                    out.cells += r.cells;
                    if r.distance_interval_mm[0] <= 0. {
                        out.problem_loops = Some([a, b]);
                        return Ok(out);
                    }
                }
            }
        }
    }
    out.reason = "loop-containment-unproven";
    let domains = loops
        .iter()
        .map(|l| TrimDomain::new(std::slice::from_ref(l), tolerance))
        .collect::<Result<Vec<_>>>()?;
    for i in 0..n {
        for (j, d) in domains.iter().enumerate() {
            if i != j {
                if out.domain_cells == limits.domain_cells {
                    out.problem_loops = Some([i, j]);
                    return Ok(out);
                }
                let p = &loops[i][0].control_points[0];
                let r = d.classify_point(
                    [p[0], p[1]],
                    (limits.domain_cells - out.domain_cells).min(100000),
                )?;
                out.domain_cells += r.cells;
                out.inside[i][j] = match r.location {
                    Location::Inside => Some(true),
                    Location::Outside => Some(false),
                    Location::Unresolved => {
                        out.problem_loops = Some([i, j]);
                        return Ok(out);
                    }
                };
            }
        }
    }
    out.reason = "containment-forest-unproven";
    let Some(nodes) = forest(&out) else {
        return Ok(out);
    };
    out.nodes = Some(nodes);
    out.proven = true;
    out.reason = "disjoint-Jordan-nesting-forest-proven";
    Ok(out)
}
pub fn build(loops: &[Vec<Curve>], tolerance: f64, limits: Limits) -> Result<Domain> {
    let proof = inspect(loops, tolerance, limits)?;
    check(
        proof.proven,
        "Complete original-loop nesting remains unproven",
    )?;
    let nodes = proof.nodes.as_ref().unwrap();
    let mut components = vec![];
    for (i, node) in nodes.iter().enumerate() {
        if node.depth % 2 == 0 {
            components.push(Component {
                outer: i,
                holes: nodes
                    .iter()
                    .enumerate()
                    .filter_map(|(j, n)| (n.parent == Some(i)).then_some(j))
                    .collect(),
            });
        }
    }
    Ok(Domain {
        loops: loops.to_vec(),
        tolerance,
        limits,
        proof,
        components,
    })
}
