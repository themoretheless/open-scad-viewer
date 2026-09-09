//! Merge supported observations into shared vertices, then remap observed faces.
//! This is surfel fusion, not volumetric TSDF or a watertight remesher. Different
//! view tessellations can still overlap when they do not share all three vertices.
use super::volume::{FastMap, FastSet};
use super::{
    cancelled,
    consistency::{ObservedPatch, Sample},
    DenseDiagnostics, Surface,
};
use crate::{math::*, Result};

struct Surfel {
    /// Fixed anchor bounds correspondence; accumulated averages cannot drift
    /// through a chain of successively close but incompatible observations.
    anchor: V3,
    normal: Option<V3>,
    position_sum: V3,
    color_sum: V3,
    weight: f64,
    radius: f64,
    /// Observations accumulated so far; reported when the merge pass absorbs
    /// this surfel into another vertex.
    samples: u32,
    sources: [u64; 4],
}
impl Surfel {
    fn new(sample: &Sample, radius: f64) -> Self {
        let mut sources = [0; 4];
        sources[sample.source / 64] |= 1 << (sample.source % 64);
        Self {
            anchor: sample.position,
            normal: sample.normal,
            position_sum: scale(sample.position, sample.weight),
            color_sum: sample.color.map(|v| v as f64 * sample.weight),
            weight: sample.weight,
            radius,
            samples: 1,
            sources,
        }
    }
    fn centroid(&self) -> V3 {
        scale(self.position_sum, 1. / self.weight)
    }
    fn accepts(&self, sample: &Sample, radius: f64) -> Option<f64> {
        // Preserve each view's sampling resolution and never count one image twice.
        if self.sources[sample.source / 64] & (1 << (sample.source % 64)) != 0 {
            return None;
        }
        let (Some(a), Some(b)) = (self.normal, sample.normal) else {
            return None;
        };
        if dot(a, b) < 0.9 {
            return None;
        }
        let delta = sub(sample.position, self.anchor);
        let limit = self.radius.min(radius);
        let distance = norm(delta);
        if distance > limit {
            return None;
        }
        // Tangential tolerance is broader than normal displacement: do not smooth
        // two parallel layers together just because they fall in the same voxel.
        if dot(delta, a).abs() > 0.2 * limit || dot(delta, b).abs() > 0.2 * limit {
            return None;
        }
        Some(distance)
    }
    fn add(&mut self, sample: &Sample) {
        self.position_sum = add(self.position_sum, scale(sample.position, sample.weight));
        self.color_sum = add(
            self.color_sum,
            sample.color.map(|v| v as f64 * sample.weight),
        );
        self.weight += sample.weight;
        self.samples += 1;
        self.sources[sample.source / 64] |= 1 << (sample.source % 64);
    }
    /// Absorb another compatible surfel during the optional merge pass.
    fn absorb(&mut self, other: &Surfel) {
        self.position_sum = add(self.position_sum, other.position_sum);
        self.color_sum = add(self.color_sum, other.color_sum);
        self.weight += other.weight;
        self.samples += other.samples;
        for k in 0..4 {
            self.sources[k] |= other.sources[k];
        }
    }
    /// Centroid-based counterpart of `accepts` for the merge pass: same normal,
    /// distance and tangential tolerances, but compared between the accumulated
    /// averages of both groups instead of a fixed anchor.
    fn merges_with(&self, other: &Surfel) -> bool {
        if self
            .sources
            .iter()
            .zip(&other.sources)
            .any(|(a, b)| a & b != 0)
        {
            return false;
        }
        let (Some(a), Some(b)) = (self.normal, other.normal) else {
            return false;
        };
        if dot(a, b) < 0.9 {
            return false;
        }
        let delta = sub(other.centroid(), self.centroid());
        let limit = self.radius.min(other.radius);
        if norm(delta) > limit {
            return false;
        }
        dot(delta, a).abs() <= 0.2 * limit && dot(delta, b).abs() <= 0.2 * limit
    }
}
fn grid_key(p: V3, step: f64) -> [i64; 3] {
    p.map(|x| (x / step).floor() as i64)
}

/// Optional second pass: merge compatible surfels whose fixed anchors rejected
/// each other even though their accumulated averages agree. Returns the remap
/// from original to compacted vertex id plus the alive flags, or None when
/// nothing merged (caller then keeps the previous byte-identical output).
/// Merges always target the lower id, which stays alive once processed, so
/// `merged_into` never forms chains and the order is deterministic.
fn consolidate(
    surfels: &mut [Surfel],
    grid: &FastMap<[i64; 3], Vec<u32>>,
    step: f64,
    diagnostics: &mut DenseDiagnostics,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Option<(Vec<u32>, Vec<bool>)>> {
    let mut merged_into: Vec<u32> = (0..surfels.len() as u32).collect();
    let mut candidates = Vec::new();
    for i in 0..surfels.len() {
        if i % 1024 == 0 {
            cancelled(progress, "fusion-merge", i, surfels.len())?;
        }
        if merged_into[i] != i as u32 || surfels[i].normal.is_none() {
            continue;
        }
        candidates.clear();
        // Candidates register under their anchor cell; an anchor can trail the
        // group average by up to one acceptance radius, so two cells around the
        // centroid are needed to see every mergeable surfel.
        let key = grid_key(surfels[i].centroid(), step);
        for z in -2..=2 {
            for y in -2..=2 {
                for x in -2..=2 {
                    let neighbor = [
                        key[0].saturating_add(x),
                        key[1].saturating_add(y),
                        key[2].saturating_add(z),
                    ];
                    let Some(ids) = grid.get(&neighbor) else {
                        continue;
                    };
                    candidates.extend(ids.iter().copied().filter(|&j| j as usize > i));
                }
            }
        }
        candidates.sort_unstable();
        candidates.dedup();
        for j in candidates.iter().map(|&j| j as usize) {
            if merged_into[j] != j as u32 {
                continue;
            }
            // i < j, so the split yields two disjoint surfels.
            let (lower, upper) = surfels.split_at_mut(j);
            let (target, source) = (&mut lower[i], &upper[0]);
            if target.merges_with(source) {
                let samples = source.samples;
                target.absorb(source);
                merged_into[j] = i as u32;
                diagnostics.fused_samples += samples as usize;
            }
        }
    }
    if merged_into.iter().enumerate().all(|(i, &r)| r == i as u32) {
        return Ok(None);
    }
    let mut remap = vec![u32::MAX; surfels.len()];
    let mut alive = vec![false; surfels.len()];
    let mut next = 0u32;
    for (i, &root) in merged_into.iter().enumerate() {
        if root == i as u32 {
            alive[i] = true;
            remap[i] = next;
            next += 1;
        }
    }
    for (i, &root) in merged_into.iter().enumerate() {
        if root != i as u32 {
            remap[i] = remap[root as usize];
        }
    }
    Ok(Some((remap, alive)))
}

#[cfg(test)]
pub(super) fn fuse(
    patches: &[ObservedPatch],
    enabled: bool,
    diagnostics: &mut DenseDiagnostics,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Surface> {
    fuse_consolidating(patches, enabled, false, diagnostics, progress)
}

pub(super) fn fuse_consolidating(
    patches: &[ObservedPatch],
    enabled: bool,
    merge_pass: bool,
    diagnostics: &mut DenseDiagnostics,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Surface> {
    cancelled(progress, "fusion", 0, patches.len())?;
    let mut footprints: Vec<_> = patches
        .iter()
        .flat_map(|p| p.samples.iter().map(|s| s.footprint))
        .filter(|x| x.is_finite() && *x > 0.)
        .collect();
    if footprints.is_empty() {
        return Ok(Surface::default());
    }
    let middle = footprints.len() / 2;
    footprints.select_nth_unstable_by(middle, f64::total_cmp);
    let step = (footprints[middle] * 0.9).max(1e-12);
    drop(footprints);
    let mut grid = FastMap::<[i64; 3], Vec<u32>>::default();
    let mut surfels = Vec::<Surfel>::new();
    let mut patch_indices = Vec::with_capacity(patches.len());
    for (patch_index, patch) in patches.iter().enumerate() {
        let mut ids = Vec::with_capacity(patch.samples.len());
        for (sample_index, sample) in patch.samples.iter().enumerate() {
            if sample_index % 256 == 0 {
                cancelled(progress, "fusion", patch_index, patches.len())?;
            }
            let key = grid_key(sample.position, step);
            let radius = (sample.footprint * 0.8).min(step);
            let mut best = None;
            if enabled && sample.normal.is_some() {
                for z in -1..=1 {
                    for y in -1..=1 {
                        for x in -1..=1 {
                            let neighbor = [
                                key[0].saturating_add(x),
                                key[1].saturating_add(y),
                                key[2].saturating_add(z),
                            ];
                            let Some(candidates) = grid.get(&neighbor) else {
                                continue;
                            };
                            for &id in candidates {
                                if let Some(distance) = surfels[id as usize].accepts(sample, radius)
                                {
                                    if best.is_none_or(|(d, best_id)| {
                                        distance < d || (distance == d && id < best_id)
                                    }) {
                                        best = Some((distance, id));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let id = if let Some((_, id)) = best {
                surfels[id as usize].add(sample);
                diagnostics.fused_samples += 1;
                id
            } else {
                let id = surfels.len() as u32;
                surfels.push(Surfel::new(sample, radius));
                // A surfel without a normal can never accept a candidate.
                if enabled && sample.normal.is_some() {
                    grid.entry(key).or_default().push(id);
                }
                id
            };
            ids.push(id);
        }
        patch_indices.push(ids);
    }
    // Opt-in consolidation; without it (or without merges) the surfel order
    // and the patch mapping are exactly the previous ones.
    let merged = if merge_pass {
        consolidate(&mut surfels, &grid, step, diagnostics, progress)?
    } else {
        None
    };
    let mut surface = Surface::default();
    for (old, surfel) in surfels.into_iter().enumerate() {
        if merged.as_ref().is_some_and(|(_, alive)| !alive[old]) {
            continue;
        }
        surface
            .positions
            .push(scale(surfel.position_sum, 1. / surfel.weight));
        surface.colors.push(
            surfel
                .color_sum
                .map(|x| (x / surfel.weight).round().clamp(0., 255.) as u8),
        );
    }
    if let Some((remap, _)) = &merged {
        for ids in &mut patch_indices {
            for id in ids.iter_mut() {
                *id = remap[*id as usize];
            }
        }
    }
    let mut unique = FastSet::default();
    for (patch_index, (patch, ids)) in patches.iter().zip(patch_indices).enumerate() {
        cancelled(progress, "surface", patch_index, patches.len())?;
        for &triangle in &patch.triangles {
            let mapped = triangle.map(|i| ids[i as usize]);
            if mapped[0] == mapped[1] || mapped[1] == mapped[2] || mapped[0] == mapped[2] {
                continue;
            }
            let [a, b, c] = mapped.map(|i| surface.positions[i as usize]);
            let area = cross(sub(b, a), sub(c, a));
            if norm(area) < step * step * 1e-8 {
                continue;
            }
            let [oa, ob, oc] = triangle.map(|i| patch.samples[i as usize].position);
            if dot(area, cross(sub(ob, oa), sub(oc, oa))) <= 0. {
                continue;
            }
            let mut key = mapped;
            key.sort_unstable();
            if unique.insert(key) {
                surface.triangles.push(mapped);
            }
        }
    }
    Ok(surface)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plane(source: usize, z: f64, normal: V3) -> ObservedPatch {
        ObservedPatch {
            samples: [[0., 0., z], [1., 0., z], [0., 1., z]]
                .map(|position| Sample {
                    position,
                    normal: Some(normal),
                    color: [100; 3],
                    footprint: 1.,
                    weight: 1.,
                    source,
                })
                .to_vec(),
            triangles: vec![[0, 1, 2]],
        }
    }
    #[test]
    fn overlapping_planes_share_vertices_and_faces_without_changing_source() {
        let a = plane(0, 4., [0., 0., -1.]);
        let mut b = plane(1, 4., [0., 0., -1.]);
        for sample in &mut b.samples {
            sample.color = [200; 3];
            sample.weight = 3.;
        }
        let mut diagnostics = DenseDiagnostics::default();
        let surface = fuse_consolidating(&[a, b], true, false, &mut diagnostics, &mut |_, _, _| true).unwrap();
        assert_eq!(surface.positions.len(), 3);
        assert_eq!(surface.triangles.len(), 1);
        assert_eq!(diagnostics.fused_samples, 3);
        assert!(surface.colors.iter().all(|&c| c == [175; 3]));
        assert!(surface.positions.iter().all(|p| p[2] == 4.));
    }
    #[test]
    fn fusion_keeps_opposite_sides_and_separated_parallel_layers() {
        for merge_pass in [false, true] {
            for (height, normal) in [(4.0001, [0., 0., 1.]), (4.3, [0., 0., -1.])] {
                let mut diagnostics = DenseDiagnostics::default();
                let surface = fuse_consolidating(
                    &[plane(0, 4., [0., 0., -1.]), plane(1, height, normal)],
                    true,
                    merge_pass,
                    &mut diagnostics,
                    &mut |_, _, _| true,
                )
                .unwrap();
                assert_eq!(surface.positions.len(), 6);
                assert_eq!(surface.triangles.len(), 2);
                assert_eq!(diagnostics.fused_samples, 0);
            }
        }
    }
    #[test]
    fn merge_pass_merges_observations_rejected_by_a_fixed_anchor() {
        // Views at 0.0 and 0.5 share a surfel, but 1.0 lies beyond the fixed
        // anchor radius; its centroid sits well inside it, so only the merge
        // pass joins all three views into one vertex per position.
        let patches = || {
            [(0, 0.), (1, 0.5), (2, 1.0)].map(|(source, x)| ObservedPatch {
                samples: [[x, 0., 4.], [x, 1., 4.], [x, 0., 5.]]
                    .map(|position| Sample {
                        position,
                        normal: Some([0., 0., -1.]),
                        color: [100; 3],
                        footprint: 1.,
                        weight: 1.,
                        source,
                    })
                    .to_vec(),
                triangles: vec![[0, 1, 2]],
            })
        };
        let mut diagnostics = DenseDiagnostics::default();
        let before = fuse_consolidating(&patches(), true, false, &mut diagnostics, &mut |_, _, _| true)
            .unwrap();
        assert_eq!(before.positions.len(), 6);
        assert_eq!(before.triangles.len(), 2);
        assert_eq!(diagnostics.fused_samples, 3);
        let mut diagnostics = DenseDiagnostics::default();
        let after = fuse_consolidating(&patches(), true, true, &mut diagnostics, &mut |_, _, _| true)
            .unwrap();
        assert_eq!(after.positions.len(), 3);
        assert_eq!(after.triangles.len(), 1);
        assert_eq!(diagnostics.fused_samples, 6);
        assert!(after.positions.iter().all(|p| p[0] == 0.5));
        // The pass is deterministic and disabled fusion keeps it inert.
        let repeat = fuse_consolidating(&patches(), true, true, &mut DenseDiagnostics::default(), &mut |_, _, _| true)
            .unwrap();
        assert_eq!(
            after.positions.iter().map(|p| p.map(f64::to_bits)).collect::<Vec<_>>(),
            repeat.positions.iter().map(|p| p.map(f64::to_bits)).collect::<Vec<_>>()
        );
        assert_eq!(after.triangles, repeat.triangles);
        let disabled = fuse_consolidating(&patches(), false, true, &mut DenseDiagnostics::default(), &mut |_, _, _| true)
            .unwrap();
        assert_eq!(disabled.positions.len(), 9);
    }
    #[test]
    fn fusion_does_not_merge_same_image_or_unknown_normals_and_can_be_disabled() {
        let a = plane(0, 4., [0., 0., -1.]);
        let mut b = plane(1, 4., [0., 0., -1.]);
        for sample in &mut b.samples {
            sample.normal = None;
        }
        for (patches, enabled) in [
            ([a.clone_for_test(), a.clone_for_test()], true),
            ([a.clone_for_test(), b], true),
            ([a, plane(1, 4., [0., 0., -1.])], false),
        ] {
            let surface = fuse_consolidating(
                &patches,
                enabled,
                false,
                &mut DenseDiagnostics::default(),
                &mut |_, _, _| true,
            )
            .unwrap();
            assert_eq!(surface.positions.len(), 6);
        }
    }
    impl ObservedPatch {
        fn clone_for_test(&self) -> Self {
            Self {
                samples: self.samples.clone(),
                triangles: self.triangles.clone(),
            }
        }
    }
    #[test]
    fn fusion_cancellation_is_explicit() {
        assert_eq!(
            fuse_consolidating(
                &[plane(0, 4., [0., 0., -1.])],
                true,
                false,
                &mut DenseDiagnostics::default(),
                &mut |_, _, _| false
            )
            .unwrap_err(),
            "Cancelled"
        );
    }
}
