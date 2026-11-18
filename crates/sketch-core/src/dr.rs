use crate::residuals::jacobian_into;
use crate::solver::rank_flat;
use crate::{Constraint, Result, Sketch};

/// One connected component of the decomposition, with diagnostics in original indices.
#[derive(Clone, Debug, PartialEq)]
pub struct RigidCluster {
    pub points: Vec<usize>,
    pub circles: Vec<usize>,
    pub constraints: Vec<usize>,
    pub degrees_of_freedom: usize,
    pub redundant_equations: usize,
    pub overconstrained_constraints: Vec<usize>,
    pub underconstrained_points: Vec<usize>,
    pub rigid: bool,
}

/// A bounded decomposition step. Independence is evaluated at the input geometry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrStep {
    pub constraint: usize,
    pub clusters: Vec<usize>,
    pub independent: bool,
}

/// Generic rigidity of the bar subgraph: points joined by `Distance`
/// constraints, decomposed by the (2,3)-pebble game before numeric solving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarRigidity {
    /// Constraint index of every bar, in the order given to the pebble game.
    pub constraints: Vec<usize>,
    /// Edge indices in `analysis` refer to positions in `constraints`;
    /// vertices are point indices.
    pub analysis: crate::PebbleAnalysis,
}

impl BarRigidity {
    /// Distance constraints that are structurally redundant for every
    /// generic placement (for example the sixth bar of K4).
    pub fn redundant_constraints(&self) -> Vec<usize> {
        self.analysis
            .redundant_edges
            .iter()
            .map(|&edge| self.constraints[edge])
            .collect()
    }
}

/// Structural/numerical DR plan consumed by the existing global solver pipeline.
/// Clusters are connected components with a first-order (Jacobian rank)
/// rigidity check at the input geometry; `bars` adds the combinatorial
/// pebble-game decomposition of the distance subgraph into rigid components.
/// Neither is a global rigidity certificate.
#[derive(Clone, Debug, PartialEq)]
pub struct DrPlan {
    pub clusters: Vec<RigidCluster>,
    pub steps: Vec<DrStep>,
    pub characteristic_scale: f64,
    pub bars: BarRigidity,
}

#[derive(Clone, Debug)]
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, value: usize) -> usize {
        let mut root = value;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut current = value;
        while self.parent[current] != current {
            let next = self.parent[current];
            self.parent[current] = root;
            current = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent[rb] = ra;
        }
    }
}

fn constraint_primitives(sketch: &Sketch, constraint: &Constraint) -> Vec<usize> {
    let mut primitives = Vec::with_capacity(8);
    let mut point = |p: usize| primitives.push(p);
    macro_rules! circle {
        ($c:expr) => {
            primitives.push(sketch.points.len() + $c)
        };
    }
    match *constraint {
        Constraint::Fix { point: p, .. } => point(p),
        Constraint::Horizontal { a, b }
        | Constraint::Vertical { a, b }
        | Constraint::Coincident { a, b }
        | Constraint::Distance { a, b, .. }
        | Constraint::FixAngle { a, b, .. } => {
            point(a);
            point(b);
        }
        Constraint::Parallel { a, b, c, d }
        | Constraint::Perpendicular { a, b, c, d }
        | Constraint::EqualLength { a, b, c, d }
        | Constraint::Angle { a, b, c, d, .. } => {
            for p in [a, b, c, d] {
                point(p);
            }
        }
        Constraint::PointToLineDistance { point: p, a, b, .. }
        | Constraint::Midpoint { point: p, a, b }
        | Constraint::PointOnLine { point: p, a, b } => {
            for p in [p, a, b] {
                point(p);
            }
        }
        Constraint::Symmetric {
            a,
            b,
            axis_a,
            axis_b,
        } => {
            for p in [a, b, axis_a, axis_b] {
                point(p);
            }
        }
        Constraint::Collinear { a, b, c } => {
            for p in [a, b, c] {
                point(p);
            }
        }
        Constraint::Radius { circle: c, .. } => circle!(c),
        Constraint::Concentric { a, b } | Constraint::EqualRadius { a, b } => {
            circle!(a);
            circle!(b);
        }
        Constraint::PointOnCircle {
            point: p,
            circle: c,
        }
        | Constraint::PointOnArc {
            point: p,
            circle: c,
            ..
        } => {
            point(p);
            circle!(c);
        }
        Constraint::TangentLineCircle { a, b, circle: c } => {
            point(a);
            point(b);
            circle!(c);
        }
        Constraint::TangentCircles { a, b, .. } => {
            circle!(a);
            circle!(b);
        }
        Constraint::EqualAngle {
            a,
            b,
            c,
            d,
            e,
            f,
            g,
            h,
        } => {
            for p in [a, b, c, d, e, f, g, h] {
                point(p);
            }
        }
    }
    primitives.sort_unstable();
    primitives.dedup();
    primitives
}

/// Builds the DR plan used before nonlinear iteration.
pub fn plan_dr(sketch: &Sketch, tolerance: f64) -> Result<DrPlan> {
    sketch.validate()?;
    crate::validate_tolerance(tolerance)?;
    let n_points = sketch.points.len();
    let n_primitives = n_points + sketch.circles.len();
    let mut union = UnionFind::new(n_primitives);
    let constraint_primitives: Vec<_> = sketch
        .constraints
        .iter()
        .map(|c| constraint_primitives(sketch, c))
        .collect();
    for primitives in &constraint_primitives {
        if let Some(&first) = primitives.first() {
            for &primitive in &primitives[1..] {
                union.union(first, primitive);
            }
        }
    }
    let mut roots = Vec::new();
    let mut root_to_cluster = vec![usize::MAX; n_primitives];
    for primitive in 0..n_primitives {
        let root = union.find(primitive);
        if root_to_cluster[root] == usize::MAX {
            root_to_cluster[root] = roots.len();
            roots.push(root);
        }
    }

    let mut x: Vec<_> = sketch
        .points
        .iter()
        .flatten()
        .copied()
        .chain(sketch.circles.iter().map(|c| c.radius))
        .collect();
    let n = x.len();
    let mut row_offsets = Vec::with_capacity(sketch.constraints.len());
    let mut m = 0;
    for constraint in &sketch.constraints {
        row_offsets.push(m);
        m += Sketch::constraint_residual_len(constraint);
    }
    let mut global_jacobian = vec![0.0; m * n];
    jacobian_into(sketch, &row_offsets, &mut x, &mut global_jacobian, n);
    let scale = sketch.characteristic_scale();
    if scale <= 0.0 || !scale.is_finite() || global_jacobian.iter().any(|v| !v.is_finite()) {
        return Err(crate::numerical_error("Non-finite DR Jacobian"));
    }
    for value in &mut global_jacobian {
        *value /= scale;
    }

    let mut clusters = Vec::with_capacity(roots.len());
    for (cluster_index, &root) in roots.iter().enumerate() {
        let points: Vec<_> = (0..n_points)
            .filter(|&p| root_to_cluster[union.find(p)] == cluster_index)
            .collect();
        let circles: Vec<_> = (0..sketch.circles.len())
            .filter(|&c| root_to_cluster[union.find(n_points + c)] == cluster_index)
            .collect();
        let constraints: Vec<_> = constraint_primitives
            .iter()
            .enumerate()
            .filter_map(|(ci, primitives)| {
                primitives
                    .first()
                    .and_then(|&p| (root_to_cluster[union.find(p)] == cluster_index).then_some(ci))
            })
            .collect();
        let mut local_vars: Vec<usize> = points.iter().flat_map(|&p| [2 * p, 2 * p + 1]).collect();
        local_vars.extend(circles.iter().map(|&c| 2 * n_points + c));
        let local_n = local_vars.len();
        let local_m: usize = constraints
            .iter()
            .map(|&ci| Sketch::constraint_residual_len(&sketch.constraints[ci]))
            .sum();
        let mut local_jacobian = vec![0.0; local_m * local_n];
        let mut local_row = 0;
        for &ci in &constraints {
            let len = Sketch::constraint_residual_len(&sketch.constraints[ci]);
            for equation in 0..len {
                for (local_col, &global_col) in local_vars.iter().enumerate() {
                    local_jacobian[(local_row + equation) * local_n + local_col] =
                        global_jacobian[(row_offsets[ci] + equation) * n + global_col];
                }
            }
            local_row += len;
        }
        let mut rank_buffer = local_jacobian.clone();
        let rank = rank_flat(&mut rank_buffer, local_m, local_n);
        let redundant_equations = local_m.saturating_sub(rank);
        let mut pivot_columns = vec![false; local_n];
        let mut elimination = local_jacobian.clone();
        let mut elimination_row = 0;
        for col in 0..local_n {
            if elimination_row >= local_m {
                break;
            }
            let Some(pivot) = (elimination_row..local_m).max_by(|&a, &b| {
                elimination[a * local_n + col]
                    .abs()
                    .total_cmp(&elimination[b * local_n + col].abs())
            }) else {
                break;
            };
            if elimination[pivot * local_n + col].abs() < 1e-7 {
                continue;
            }
            if elimination_row != pivot {
                for column in col..local_n {
                    elimination.swap(elimination_row * local_n + column, pivot * local_n + column);
                }
            }
            pivot_columns[col] = true;
            let inverse = 1.0 / elimination[elimination_row * local_n + col];
            for row in elimination_row + 1..local_m {
                let factor = elimination[row * local_n + col] * inverse;
                if factor != 0.0 {
                    for column in col..local_n {
                        elimination[row * local_n + column] -=
                            factor * elimination[elimination_row * local_n + column];
                    }
                }
            }
            elimination_row += 1;
        }
        let underconstrained_points = points
            .iter()
            .enumerate()
            .filter_map(|(local, &point)| {
                (!pivot_columns[2 * local] || !pivot_columns[2 * local + 1]).then_some(point)
            })
            .collect();

        // Ordered row reduction attributes each dependent equation to its constraint.
        let mut basis: Vec<(usize, Vec<f64>)> = Vec::with_capacity(local_n);
        let mut overconstrained_constraints = Vec::new();
        let mut row = 0;
        for &ci in &constraints {
            let len = Sketch::constraint_residual_len(&sketch.constraints[ci]);
            let mut dependent = false;
            for equation in 0..len {
                let mut candidate = local_jacobian
                    [(row + equation) * local_n..(row + equation + 1) * local_n]
                    .to_vec();
                for &(pivot, ref basis_row) in &basis {
                    let factor = candidate[pivot];
                    if factor != 0.0 {
                        for col in 0..local_n {
                            candidate[col] -= factor * basis_row[col];
                        }
                    }
                }
                if let Some((pivot, &value)) = candidate
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
                {
                    if value.abs() >= 1e-7 {
                        for entry in &mut candidate {
                            *entry /= value;
                        }
                        basis.push((pivot, candidate));
                    } else {
                        dependent = true;
                    }
                }
            }
            if dependent {
                overconstrained_constraints.push(ci);
            }
            row += len;
        }
        let degrees_of_freedom = local_n.saturating_sub(rank);
        let rigid = degrees_of_freedom == 0
            || (degrees_of_freedom <= 3
                && ((points.len() >= 2 && circles.is_empty()) || rank > 0 && !points.is_empty()));
        let _ = root;
        clusters.push(RigidCluster {
            points,
            circles,
            constraints,
            degrees_of_freedom,
            redundant_equations,
            overconstrained_constraints,
            underconstrained_points: if rigid {
                Vec::new()
            } else {
                underconstrained_points
            },
            rigid,
        });
    }

    let steps = constraint_primitives
        .iter()
        .enumerate()
        .map(|(ci, primitives)| {
            let mut step_clusters: Vec<_> = primitives
                .iter()
                .map(|&p| root_to_cluster[union.find(p)])
                .collect();
            step_clusters.sort_unstable();
            step_clusters.dedup();
            let independent = !clusters
                .iter()
                .any(|cluster| cluster.overconstrained_constraints.contains(&ci));
            DrStep {
                constraint: ci,
                clusters: step_clusters,
                independent,
            }
        })
        .collect();
    let (bar_constraints, bar_edges): (Vec<usize>, Vec<(usize, usize)>) = sketch
        .constraints
        .iter()
        .enumerate()
        .filter_map(|(ci, constraint)| match *constraint {
            Constraint::Distance { a, b, .. } if a != b => Some((ci, (a, b))),
            _ => None,
        })
        .unzip();
    let bars = BarRigidity {
        analysis: crate::pebble_game_2d(n_points, &bar_edges)?,
        constraints: bar_constraints,
    };
    Ok(DrPlan {
        clusters,
        steps,
        characteristic_scale: scale,
        bars,
    })
}
