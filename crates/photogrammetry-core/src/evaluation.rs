//! Bidirectional point-sample evaluation in a caller-established coordinate frame.
//! No registration, scale fitting, or inferred ground truth is performed here.
use crate::Result;
use math_core::{Acceleration, nearest_neighbor_accelerated};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

type Point = math_core::V3;
const MAX_POINTS: usize = 2_000_000;
const NONE: usize = usize::MAX;

/// Tiny deterministic FNV-style hasher for voxel keys; the std SipHash build
/// dominates small cell maps. Cell order affects only sample order, never the
/// sorted distance summary.
#[derive(Default)]
struct VoxelHasher(u64);
impl Hasher for VoxelHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    fn write_i64(&mut self, v: i64) {
        self.0 = (self.0 ^ v as u64).wrapping_mul(0x100000001b3);
    }
}

#[derive(Clone, Debug)]
pub struct EvaluationOptions {
    /// Distance tolerance in the same units as both input clouds.
    pub tolerance: f64,
    /// Optional common voxel grid for density-normalized sampling of both clouds.
    /// Origin is zero in the established common frame. None preserves input density.
    pub voxel_size: Option<f64>,
    /// `Cpu` (default) builds an exact KD-tree per cloud, O(log n) per query.
    /// `Gpu`/`Cuda` instead run a brute-force batch nearest-neighbor kernel:
    /// measured (`examples/kdtree_vs_gpu.rs`, RTX 5090) 3-11x faster than the
    /// KD-tree from 2K to 1M points per cloud despite the worse asymptotic
    /// complexity, because the GPU's parallelism outweighs the KD-tree's
    /// pointer-chasing recursion at these sizes; re-measure for your own
    /// hardware and cloud sizes before relying on this for a hard real-time
    /// budget. Requires this crate's `gpu` (or `cuda`) feature; otherwise
    /// silently uses the KD-tree regardless of this setting, since the
    /// alternative — a plain CPU brute-force scan — would be a regression.
    pub acceleration: Acceleration,
}

#[derive(Clone, Debug)]
pub struct DistanceSummary {
    pub samples: usize,
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
    pub maximum: f64,
    pub within_tolerance: usize,
}

#[derive(Clone, Debug)]
pub struct CloudEvaluation {
    pub input_reconstructed: usize,
    pub input_reference: usize,
    /// Reconstructed sample to reference sample: geometric accuracy direction.
    pub reconstructed_to_reference: DistanceSummary,
    /// Reference sample to reconstructed sample: completeness direction.
    pub reference_to_reconstructed: DistanceSummary,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub symmetric_mean: f64,
}

fn validate(points: &[Point]) -> Result<()> {
    if points.is_empty() || points.len() > MAX_POINTS {
        return Err(crate::error(
            "Evaluation expects 1 to 2000000 samples per cloud",
        ));
    }
    if points
        .iter()
        .flatten()
        .any(|v| !v.is_finite() || v.abs() > 1e12)
    {
        return Err(crate::error(
            "Evaluation coordinates must be finite and at most 1e12 in magnitude",
        ));
    }
    Ok(())
}

fn sampled(
    points: &[Point],
    voxel: Option<f64>,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Vec<Point>> {
    let Some(size) = voxel else {
        return Ok(points.to_vec());
    };
    let mut cells = HashMap::<[i64; 3], (Point, usize), BuildHasherDefault<VoxelHasher>>::default();
    for (i, p) in points.iter().enumerate() {
        if i % 4096 == 0 && !progress("evaluation_sample", i, points.len()) {
            return Err(crate::error("Cancelled"));
        }
        let mut key = [0; 3];
        for k in 0..3 {
            let q = (p[k] / size).floor();
            if !q.is_finite() || q.abs() >= (1i64 << 60) as f64 {
                return Err(crate::error(
                    "Evaluation voxel size is too small for these coordinates",
                ));
            }
            key[k] = q as i64;
        }
        let cell = cells.entry(key).or_insert(([0.; 3], 0));
        cell.1 += 1;
        for k in 0..3 {
            cell.0[k] += (p[k] - cell.0[k]) / cell.1 as f64;
        }
    }
    Ok(cells.into_values().map(|(centroid, _)| centroid).collect())
}

struct Node {
    point: Point,
    left: usize,
    right: usize,
    axis: usize,
}
struct Tree {
    nodes: Vec<Node>,
    root: usize,
}

impl Tree {
    fn new(
        mut points: Vec<Point>,
        progress: &mut impl FnMut(&str, usize, usize) -> bool,
    ) -> Result<Self> {
        let total = points.len();
        let mut tree = Self {
            nodes: Vec::with_capacity(total),
            root: NONE,
        };
        tree.root = tree.build(&mut points, 0, total, progress)?;
        Ok(tree)
    }
    fn build(
        &mut self,
        points: &mut [Point],
        level: usize,
        total: usize,
        progress: &mut impl FnMut(&str, usize, usize) -> bool,
    ) -> Result<usize> {
        if points.is_empty() {
            return Ok(NONE);
        }
        if self.nodes.len().is_multiple_of(4096)
            && !progress("evaluation_index", self.nodes.len(), total)
        {
            return Err(crate::error("Cancelled"));
        }
        let axis = level % 3;
        let mid = points.len() / 2;
        points.select_nth_unstable_by(mid, |a, b| {
            a[axis]
                .total_cmp(&b[axis])
                .then(a[(axis + 1) % 3].total_cmp(&b[(axis + 1) % 3]))
                .then(a[(axis + 2) % 3].total_cmp(&b[(axis + 2) % 3]))
        });
        let index = self.nodes.len();
        self.nodes.push(Node {
            point: points[mid],
            left: NONE,
            right: NONE,
            axis,
        });
        let (left, right) = points.split_at_mut(mid);
        let l = self.build(left, level + 1, total, progress)?;
        let r = self.build(&mut right[1..], level + 1, total, progress)?;
        self.nodes[index].left = l;
        self.nodes[index].right = r;
        Ok(index)
    }
    fn nearest(&self, point: Point, keep_going: &mut impl FnMut() -> bool) -> Result<f64> {
        let mut squared = f64::INFINITY;
        self.search(self.root, point, &mut squared, &mut 0, keep_going)?;
        Ok(squared.sqrt())
    }
    fn search(
        &self,
        index: usize,
        point: Point,
        best: &mut f64,
        until_check: &mut usize,
        keep_going: &mut impl FnMut() -> bool,
    ) -> Result<()> {
        if index == NONE {
            return Ok(());
        }
        // Countdown to the next cancellation check; same cadence as a modulo
        // on the visited count, without the division on every node.
        if *until_check == 0 {
            if !keep_going() {
                return Err(crate::error("Cancelled"));
            }
            *until_check = 4096;
        }
        *until_check -= 1;
        let n = &self.nodes[index];
        let distance = (0..3).map(|k| (point[k] - n.point[k]).powi(2)).sum::<f64>();
        *best = best.min(distance);
        let delta = point[n.axis] - n.point[n.axis];
        let (near, far) = if delta <= 0. {
            (n.left, n.right)
        } else {
            (n.right, n.left)
        };
        self.search(near, point, best, until_check, keep_going)?;
        if delta * delta < *best {
            self.search(far, point, best, until_check, keep_going)?;
        }
        Ok(())
    }
}

fn summarize(mut values: Vec<f64>, tolerance: f64) -> DistanceSummary {
    values.sort_unstable_by(f64::total_cmp);
    let samples = values.len();
    let quantile = |q: f64| {
        let position = (samples - 1) as f64 * q;
        let lower = position.floor() as usize;
        let upper = position.ceil() as usize;
        values[lower] + (values[upper] - values[lower]) * position.fract()
    };
    DistanceSummary {
        samples,
        mean: values.iter().sum::<f64>() / samples as f64,
        median: quantile(0.5),
        p95: quantile(0.95),
        maximum: values[samples - 1],
        within_tolerance: values.iter().filter(|&&d| d <= tolerance).count(),
    }
}

fn distances(
    queries: &Tree,
    tree: &Tree,
    tolerance: f64,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<DistanceSummary> {
    let mut values = Vec::with_capacity(queries.nodes.len());
    for (i, n) in queries.nodes.iter().enumerate() {
        if i % 1024 == 0 && !progress("evaluation_distance", i, queries.nodes.len()) {
            return Err(crate::error("Cancelled"));
        }
        values.push(tree.nearest(n.point, &mut || {
            progress("evaluation_distance", i, queries.nodes.len())
        })?);
    }
    Ok(summarize(values, tolerance))
}

/// Same result as [`distances`], via a single batch GPU/CUDA nearest-neighbor
/// call instead of building a KD-tree; see [`EvaluationOptions::acceleration`].
fn distances_accelerated(
    queries: &[Point],
    targets: &[Point],
    tolerance: f64,
    acceleration: Acceleration,
) -> DistanceSummary {
    let values = nearest_neighbor_accelerated(queries, targets, acceleration)
        .into_iter()
        .map(|(_, squared)| squared.sqrt())
        .collect();
    summarize(values, tolerance)
}

/// Both inputs must already use the same frame and scale. Reference points must
/// describe the evaluated/observable domain; unknown reference space is not inferred.
/// Metrics are point-to-point, not distances to the interior of mesh triangles.
pub fn evaluate_clouds(
    reconstructed: &[Point],
    reference: &[Point],
    options: &EvaluationOptions,
    mut progress: impl FnMut(&str, usize, usize) -> bool,
) -> Result<CloudEvaluation> {
    if !progress("evaluation", 0, 1) {
        return Err(crate::error("Cancelled"));
    }
    validate(reconstructed)?;
    validate(reference)?;
    if !options.tolerance.is_finite()
        || options.tolerance <= 0.
        || options
            .voxel_size
            .is_some_and(|v| !v.is_finite() || v <= 0.)
    {
        return Err(crate::error(
            "Evaluation tolerance and optional voxel size must be finite and positive",
        ));
    }
    let a = sampled(reconstructed, options.voxel_size, &mut progress)?;
    let b = sampled(reference, options.voxel_size, &mut progress)?;
    let (accuracy, completeness) = if options.acceleration.is_gpu() && cfg!(feature = "gpu") {
        if !progress("evaluation_index", 0, 1) {
            return Err(crate::error("Cancelled"));
        }
        let accuracy = distances_accelerated(&a, &b, options.tolerance, options.acceleration);
        let completeness = distances_accelerated(&b, &a, options.tolerance, options.acceleration);
        if !progress("evaluation_distance", 1, 1) {
            return Err(crate::error("Cancelled"));
        }
        (accuracy, completeness)
    } else {
        // The trees hold the same point multisets as the sampled vectors, and the
        // sorted summaries do not depend on query order, so the clones of the
        // sampled clouds are unnecessary.
        let ta = Tree::new(a, &mut progress)?;
        let tb = Tree::new(b, &mut progress)?;
        let accuracy = distances(&ta, &tb, options.tolerance, &mut progress)?;
        let completeness = distances(&tb, &ta, options.tolerance, &mut progress)?;
        (accuracy, completeness)
    };
    let precision = accuracy.within_tolerance as f64 / accuracy.samples as f64;
    let recall = completeness.within_tolerance as f64 / completeness.samples as f64;
    let f1 = if precision + recall == 0. {
        0.
    } else {
        2. * precision * recall / (precision + recall)
    };
    Ok(CloudEvaluation {
        input_reconstructed: reconstructed.len(),
        input_reference: reference.len(),
        symmetric_mean: (accuracy.mean + completeness.mean) / 2.,
        reconstructed_to_reference: accuracy,
        reference_to_reconstructed: completeness,
        precision,
        recall,
        f1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> EvaluationOptions {
        EvaluationOptions {
            tolerance: 0.01,
            voxel_size: None,
            acceleration: Acceleration::Cpu,
        }
    }
    #[test]
    fn removing_difficult_surface_improves_one_direction_but_loses_recall() {
        let reference = [[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [3., 0., 0.]];
        let noisy = [[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [3., 0.005, 0.]];
        let full = evaluate_clouds(&noisy, &reference, &options(), |_, _, _| true).unwrap();
        let partial = evaluate_clouds(&noisy[..2], &reference, &options(), |_, _, _| true).unwrap();
        assert!(partial.reconstructed_to_reference.mean < full.reconstructed_to_reference.mean);
        assert_eq!(partial.precision, 1.);
        assert_eq!(partial.recall, 0.5);
        assert!(partial.f1 < full.f1);
    }
    #[test]
    fn no_scale_fit_hides_wrong_size() {
        let reference = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let scaled = [[0., 0., 0.], [2., 0., 0.], [0., 2., 0.]];
        let report = evaluate_clouds(&scaled, &reference, &options(), |_, _, _| true).unwrap();
        assert_eq!(report.precision, 1. / 3.);
        assert_eq!(report.recall, 1. / 3.);
        assert_eq!(report.reconstructed_to_reference.maximum, 1.);
    }
    #[test]
    fn exact_tree_matches_brute_force_on_nonuniform_cloud() {
        let p = (0..700)
            .map(|i| {
                [
                    ((i * 97) % 301) as f64 / 30.,
                    ((i * 29) % 71) as f64,
                    ((i * 17) % 89) as f64,
                ]
            })
            .collect::<Vec<_>>();
        let tree = Tree::new(p.clone(), &mut |_, _, _| true).unwrap();
        for i in 0..150 {
            let q = [i as f64 * 0.31, (i % 65) as f64 + 0.23, (i % 89) as f64];
            let expected = p
                .iter()
                .map(|x| (0..3).map(|k| (q[k] - x[k]).powi(2)).sum::<f64>().sqrt())
                .fold(f64::INFINITY, f64::min);
            assert!((tree.nearest(q, &mut || true).unwrap() - expected).abs() < 1e-12);
        }
    }
    #[test]
    fn cancellation_is_checked_inside_an_adversarial_nearest_query() {
        let sphere = (0..20000)
            .map(|i| {
                let z = 1. - 2. * (i as f64 + 0.5) / 20000.;
                let angle = i as f64 * 2.399963229728653;
                let r = (1. - z * z).sqrt();
                [r * angle.cos(), r * angle.sin(), z]
            })
            .collect::<Vec<_>>();
        let tree = Tree::new(sphere, &mut |_, _, _| true).unwrap();
        let mut checks = 0;
        let result = tree.nearest([0., 0., 0.], &mut || {
            checks += 1;
            checks < 2
        });
        assert_eq!(result.unwrap_err().message, "Cancelled");
        assert_eq!(checks, 2);
    }
    #[test]
    fn common_voxel_sampling_prevents_duplicate_density_from_changing_f1() {
        let reference = [[0., 0., 0.], [1., 0., 0.]];
        let base = [[0., 0., 0.], [10., 0., 0.]];
        let mut duplicate = vec![[0., 0., 0.]; 500];
        duplicate.push([10., 0., 0.]);
        let opt = EvaluationOptions {
            voxel_size: Some(0.1),
            ..options()
        };
        let a = evaluate_clouds(&base, &reference, &opt, |_, _, _| true).unwrap();
        let b = evaluate_clouds(&duplicate, &reference, &opt, |_, _, _| true).unwrap();
        assert_eq!(a.f1, b.f1);
        assert_eq!(b.reconstructed_to_reference.samples, 2);
    }
    #[test]
    fn accelerated_matches_kdtree_when_the_gpu_feature_is_off() {
        // Without this crate's `gpu`/`cuda` feature compiled in, requesting
        // `Gpu`/`Cuda` must silently keep using the exact KD-tree rather than
        // falling through to a slow CPU brute-force scan; with the feature
        // on it exercises the real accelerated kernel (f32), so allow a
        // small tolerance instead of bit-exact equality either way.
        let p = (0..300)
            .map(|i| {
                [
                    ((i * 97) % 301) as f64 / 30.,
                    (i % 71) as f64,
                    (i % 89) as f64,
                ]
            })
            .collect::<Vec<_>>();
        let q = (0..250)
            .map(|i| {
                [
                    ((i * 53) % 199) as f64 / 20.,
                    (i % 61) as f64,
                    (i % 83) as f64,
                ]
            })
            .collect::<Vec<_>>();
        let cpu = evaluate_clouds(&p, &q, &options(), |_, _, _| true).unwrap();
        for acceleration in [Acceleration::Gpu, Acceleration::Cuda] {
            let opt = EvaluationOptions {
                acceleration,
                ..options()
            };
            let accelerated = evaluate_clouds(&p, &q, &opt, |_, _, _| true).unwrap();
            assert!(
                (cpu.reconstructed_to_reference.mean - accelerated.reconstructed_to_reference.mean)
                    .abs()
                    < 1e-3
            );
            assert!(
                (cpu.reference_to_reconstructed.mean - accelerated.reference_to_reconstructed.mean)
                    .abs()
                    < 1e-3
            );
        }
    }
    #[test]
    fn invalid_and_cancelled_evaluation_are_explicit() {
        let p = [[0., 0., 0.]];
        assert!(evaluate_clouds(&p, &[], &options(), |_, _, _| true).is_err());
        assert!(evaluate_clouds(&[[f64::NAN, 0., 0.]], &p, &options(), |_, _, _| true).is_err());
        assert_eq!(
            evaluate_clouds(&p, &p, &options(), |_, _, _| false)
                .unwrap_err()
                .message,
            "Cancelled"
        );
        assert_eq!(
            evaluate_clouds(&p, &p, &options(), |stage, _, _| stage
                != "evaluation_distance")
            .unwrap_err()
            .message,
            "Cancelled"
        );
    }
}
