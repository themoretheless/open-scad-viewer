//! Merge supported observations into shared vertices, then remap observed faces.
//! This is surfel fusion, not volumetric TSDF or a watertight remesher. Different
//! view tessellations can still overlap when they do not share all three vertices.
use super::{
    cancelled,
    consistency::{ObservedPatch, Sample},
    DenseDiagnostics, Surface,
};
use crate::{math::*, Result};
use std::collections::{HashMap, HashSet};

struct Surfel {
    /// Fixed anchor bounds correspondence; accumulated averages cannot drift
    /// through a chain of successively close but incompatible observations.
    anchor: V3,
    normal: Option<V3>,
    position_sum: V3,
    color_sum: V3,
    weight: f64,
    radius: f64,
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
            sources,
        }
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
        self.sources[sample.source / 64] |= 1 << (sample.source % 64);
    }
}
fn grid_key(p: V3, step: f64) -> [i64; 3] {
    p.map(|x| (x / step).floor() as i64)
}

pub(super) fn fuse(
    patches: &[ObservedPatch],
    enabled: bool,
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
    let mut grid = HashMap::<[i64; 3], Vec<u32>>::new();
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
                if enabled {
                    grid.entry(key).or_default().push(id);
                }
                id
            };
            ids.push(id);
        }
        patch_indices.push(ids);
    }
    let mut surface = Surface::default();
    for surfel in surfels {
        surface
            .positions
            .push(scale(surfel.position_sum, 1. / surfel.weight));
        surface.colors.push(
            surfel
                .color_sum
                .map(|x| (x / surfel.weight).round().clamp(0., 255.) as u8),
        );
    }
    let mut unique = HashSet::new();
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
        let surface = fuse(&[a, b], true, &mut diagnostics, &mut |_, _, _| true).unwrap();
        assert_eq!(surface.positions.len(), 3);
        assert_eq!(surface.triangles.len(), 1);
        assert_eq!(diagnostics.fused_samples, 3);
        assert!(surface.colors.iter().all(|&c| c == [175; 3]));
        assert!(surface.positions.iter().all(|p| p[2] == 4.));
    }
    #[test]
    fn fusion_keeps_opposite_sides_and_separated_parallel_layers() {
        for (height, normal) in [(4.0001, [0., 0., 1.]), (4.3, [0., 0., -1.])] {
            let mut diagnostics = DenseDiagnostics::default();
            let surface = fuse(
                &[plane(0, 4., [0., 0., -1.]), plane(1, height, normal)],
                true,
                &mut diagnostics,
                &mut |_, _, _| true,
            )
            .unwrap();
            assert_eq!(surface.positions.len(), 6);
            assert_eq!(surface.triangles.len(), 2);
            assert_eq!(diagnostics.fused_samples, 0);
        }
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
            let surface = fuse(
                &patches,
                enabled,
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
            fuse(
                &[plane(0, 4., [0., 0., -1.])],
                true,
                &mut DenseDiagnostics::default(),
                &mut |_, _, _| false
            )
            .unwrap_err(),
            "Cancelled"
        );
    }
}
