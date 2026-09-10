//! Experimental bounded projective signed-distance integration.
//! Unknown nodes never create a surface. A shared edge cache joins view geometry.
use super::{cancelled, consistency::ObservedPatch, DenseOptions, DepthMap, Surface};
use crate::{math::*, Reconstruction, Result};
use std::collections::{HashMap, HashSet};

/// Multiply-rotate hasher for small integer keys, in the spirit of FxHash.
/// Hash iteration order never reaches the output; only lookup cost changes.
#[derive(Default)]
pub(crate) struct FastHasher {
    hash: u64,
}
impl FastHasher {
    #[inline]
    fn fold(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(26) ^ word).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}
impl std::hash::Hasher for FastHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.fold(u64::from_le_bytes(word));
        }
    }
    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.fold(value as u64);
    }
    #[inline]
    fn write_i32(&mut self, value: i32) {
        self.fold(value as u32 as u64);
    }
    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.fold(value);
    }
    #[inline]
    fn write_i64(&mut self, value: i64) {
        self.fold(value as u64);
    }
    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.fold(value as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}
pub(crate) type FastBuild = std::hash::BuildHasherDefault<FastHasher>;
pub(crate) type FastMap<K, V> = HashMap<K, V, FastBuild>;
pub(crate) type FastSet<T> = HashSet<T, FastBuild>;

type Key = [i32; 3];
const MAX_NODES: usize = 600_000;
use super::limits::{MAX_SURFACE_VERTICES, MAX_SURFACE_TRIANGLES};
/// Conservative collection-capacity allowance, additional to dense-map planning.
pub(super) const WORKING_BYTES: usize = 256 * 1024 * 1024;
const CORNERS: [Key; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [1, 1, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [1, 1, 1],
    [0, 1, 1],
];
const TETS: [[usize; 4]; 6] = [
    [0, 1, 2, 6],
    [0, 2, 3, 6],
    [0, 3, 7, 6],
    [0, 7, 4, 6],
    [0, 4, 5, 6],
    [0, 5, 1, 6],
];

#[derive(Clone, Copy)]
struct Node {
    distance: f64,
    color: V3,
}

enum AttemptError { NodeBudget, Other(String) }
impl From<String> for AttemptError {
    fn from(value: String) -> Self { Self::Other(value) }
}
impl From<&str> for AttemptError {
    fn from(value: &str) -> Self { Self::Other(value.into()) }
}

pub(super) fn reconstruct(
    patches: &[ObservedPatch],
    maps: &[DepthMap],
    sparse: &Reconstruction,
    options: &DenseOptions,
    diagnostics: &mut super::DenseDiagnostics,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Surface> {
    bounded_attempts(diagnostics, progress, |scale, progress| {
        reconstruct_at_scale(scale, patches, maps, sparse, options, progress)
    })
}

fn bounded_attempts<P: FnMut(&str, usize, usize) -> bool>(
    diagnostics: &mut super::DenseDiagnostics,
    progress: &mut P,
    mut run: impl FnMut(f64, &mut P) -> std::result::Result<Surface, AttemptError>,
) -> Result<Surface> {
    for attempt in 0..4 {
        let spacing_scale = 1.25f64.powi(attempt);
        diagnostics.volume_attempts = attempt as usize + 1;
        diagnostics.volume_spacing_scale = Some(spacing_scale);
        match run(spacing_scale, progress) {
            Err(AttemptError::NodeBudget) if attempt < 3 => {
                cancelled(progress, "volume-coarsen", attempt as usize + 1, 3)?;
            }
            Ok(surface) => return Ok(surface),
            Err(AttemptError::NodeBudget) => return Err("Volume node budget exceeded".into()),
            Err(AttemptError::Other(error)) => return Err(error),
        }
    }
    unreachable!()
}

fn reconstruct_at_scale(
    spacing_scale: f64,
    patches: &[ObservedPatch],
    maps: &[DepthMap],
    sparse: &Reconstruction,
    options: &DenseOptions,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> std::result::Result<Surface, AttemptError> {
    cancelled(progress, "volume", 0, 1)?;
    let mut footprints: Vec<_> = patches
        .iter()
        .flat_map(|p| p.samples.iter())
        .filter(|s| s.footprint.is_finite() && s.footprint > 0.)
        .map(|s| s.footprint)
        .collect();
    if footprints.is_empty() {
        return Ok(Surface::default());
    }
    let mid = footprints.len() / 2;
    footprints.select_nth_unstable_by(mid, f64::total_cmp);
    let step = footprints[mid] * 0.5 * spacing_scale;
    let depth_tolerance = footprints[mid] * 0.75;
    if !step.is_finite() || step <= 1e-12 {
        return Err("Invalid volume spacing".into());
    }
    let mut origin = [f64::INFINITY; 3];
    for sample in patches.iter().flat_map(|p| p.samples.iter()) {
        for k in 0..3 {
            origin[k] = origin[k].min(sample.position[k]);
        }
    }
    let mut colors: Vec<Vec<[u8; 3]>> = maps.iter().map(|m| vec![[0; 3]; m.depth.len()]).collect();
    let mut triangles: Vec<Vec<u8>> = maps.iter().map(|m| vec![0; m.depth.len()]).collect();
    // First-map-wins lookup replaces repeated linear scans over maps.
    let mut image_to_map = vec![None; maps.iter().map(|m| m.image + 1).max().unwrap_or(0)];
    for (mi, map) in maps.iter().enumerate() {
        if image_to_map[map.image].is_none() {
            image_to_map[map.image] = Some(mi);
        }
    }
    let mut keys = FastSet::default();
    for (pi, patch) in patches.iter().enumerate() {
        let mut used = vec![false; patch.samples.len()];
        for triangle in &patch.triangles {
            for &i in triangle {
                used[i as usize] = true;
            }
            let source = patch.samples[triangle[0] as usize].source;
            let mi = image_to_map.get(source).copied().flatten().unwrap();
            let map = &maps[mi];
            let camera = sparse.cameras[source].as_ref().unwrap();
            let pixels = triangle.map(|i| {
                camera
                    .project(patch.samples[i as usize].position)
                    .unwrap()
                    .map(|v| (v / map.step).round() as usize)
            });
            let x = pixels.iter().map(|p| p[0]).min().unwrap();
            let y = pixels.iter().map(|p| p[1]).min().unwrap();
            let bit = if pixels.contains(&[x, y]) { 1 } else { 2 };
            triangles[mi][y * map.width + x] |= bit;
        }
        for (si, sample) in patch.samples.iter().enumerate() {
            if si % 256 == 0 {
                cancelled(progress, "volume", pi, patches.len())?;
            }
            let Some(mi) = image_to_map.get(sample.source).copied().flatten() else {
                continue;
            };
            let map = &maps[mi];
            let camera = sparse.cameras[map.image].as_ref().unwrap();
            let Some(pixel) = camera.project(sample.position) else {
                continue;
            };
            let x = (pixel[0] / map.step).round() as usize;
            let y = (pixel[1] / map.step).round() as usize;
            if x >= map.width || y >= map.height {
                continue;
            }
            colors[mi][y * map.width + x] = sample.color;
            // Orphan points do not establish a continuous local surface.
            if !used[si] {
                continue;
            }
            let cell = sub(sample.position, origin).map(|v| (v / step).floor());
            if cell
                .iter()
                .any(|v| !v.is_finite() || *v < 0. || *v > i32::MAX as f64 - 4.)
            {
                return Err("Volume coordinate exceeds bounded grid".into());
            }
            let cell = cell.map(|v| v as i32);
            for z in -2..=2 {
                for y in -2..=2 {
                    for x in -2..=2 {
                        let key = [cell[0] + x, cell[1] + y, cell[2] + z];
                        // Below the cap, insert already performs the membership lookup.
                        if keys.len() == MAX_NODES && !keys.contains(&key) {
                            return Err(AttemptError::NodeBudget);
                        }
                        keys.insert(key);
                    }
                }
            }
        }
    }
    let mut keys: Vec<_> = keys.into_iter().collect();
    keys.sort_unstable();
    let mut field = FastMap::default();
    for (ki, &key) in keys.iter().enumerate() {
        if ki % 512 == 0 {
            cancelled(progress, "volume-integrate", ki, keys.len())?;
        }
        let point = position(key, origin, step);
        let (mut distance, mut weight, mut color, mut views) = (0., 0., [0.; 3], 0);
        for (mi, map) in maps.iter().enumerate() {
            let camera = sparse.cameras[map.image].as_ref().unwrap();
            let camera_point = camera.camera_point(point);
            let Some(pixel) = camera.project_camera_point(camera_point) else {
                continue;
            };
            let Some((depth, w, rgb)) = interpolate::<true>(
                map,
                &triangles[mi],
                &colors[mi],
                pixel.map(|v| v / map.step),
            ) else {
                continue;
            };
            let z = camera_point[2];
            let signed = depth - z;
            // A bounded band, not a prior about unseen free/occupied space.
            if signed.abs() > depth_tolerance * 3. {
                continue;
            }
            if !w.is_finite() || w <= 0. {
                continue;
            }
            distance += signed * w;
            weight += w;
            color = add(color, scale(rgb, w));
            views += 1;
        }
        // Input samples have already passed reciprocal multi-view consistency.
        // Requiring two fully retained maps again erodes narrow observed features.
        if views >= 1 && weight > 0. {
            field.insert(
                key,
                Node {
                    distance: distance / weight,
                    color: scale(color, 1. / weight),
                },
            );
        }
    }
    let surface = extract(&keys, &field, origin, step, progress)?;
    let mut valid = vec![false; surface.positions.len()];
    for (i, &point) in surface.positions.iter().enumerate() {
        if i % 256 == 0 {
            cancelled(progress, "volume-validate", i, surface.positions.len())?;
        }
        let mut agreeing = 0;
        for (mi, map) in maps.iter().enumerate() {
            let camera = sparse.cameras[map.image].as_ref().unwrap();
            let camera_point = camera.camera_point(point);
            let Some(pixel) = camera.project_camera_point(camera_point) else {
                continue;
            };
            let Some((depth, _, _)) = interpolate::<false>(
                map,
                &triangles[mi],
                &colors[mi],
                pixel.map(|v| v / map.step),
            ) else {
                continue;
            };
            let z = camera_point[2];
            if (depth - z).abs() <= depth_tolerance {
                agreeing += 1;
                // Further views cannot change the accepted support predicate.
                if agreeing >= options.min_support_views + 1 {
                    break;
                }
            }
        }
        valid[i] = agreeing >= options.min_support_views + 1;
    }
    // Validate the extracted surface, rather than eroding the whole narrow band.
    // Reindex exactly; no point movement or automatic hole filling occurs here.
    let mut output = Surface::default();
    let mut remap = vec![u32::MAX; surface.positions.len()];
    for t in &surface.triangles {
        if t.iter().any(|&i| !valid[i as usize]) {
            continue;
        }
        let t = t.map(|i| {
            if remap[i as usize] == u32::MAX {
                remap[i as usize] = output.positions.len() as u32;
                output.positions.push(surface.positions[i as usize]);
                output.colors.push(surface.colors[i as usize]);
            }
            remap[i as usize]
        });
        output.triangles.push(t);
    }
    Ok(output)
}

/// Perspective-correct plane interpolation only inside observed triangles.
/// Their construction already rejects unsupported pixels and depth jumps.
fn interpolate<const ATTRIBUTES: bool>(
    map: &DepthMap,
    triangles: &[u8],
    colors: &[[u8; 3]],
    uv: [f64; 2],
) -> Option<(f64, f64, V3)> {
    if let Some(value) = interpolate_observed::<ATTRIBUTES>(map, triangles, colors, uv) {
        return Some(value);
    }
    let [x, y] = uv;
    if !x.is_finite()
        || !y.is_finite()
        || x < 0.
        || y < 0.
        || x >= map.width.saturating_sub(1) as f64
        || y >= map.height.saturating_sub(1) as f64
    {
        return None;
    }
    let mut best = None;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let ix = x as i32 + dx;
            let iy = y as i32 + dy;
            if ix < 0 || iy < 0 || ix >= map.width as i32 - 1 || iy >= map.height as i32 - 1 {
                continue;
            }
            let i = iy as usize * map.width + ix as usize;
            let p = [x - ix as f64, y - iy as f64];
            for bit in [1, 2] {
                if triangles[i] & bit == 0 {
                    continue;
                }
                let (corners, ids, bary) = if bit == 1 {
                    (
                        [[0., 0.], [0., 1.], [1., 0.]],
                        [i, i + map.width, i + 1],
                        [1. - p[0] - p[1], p[1], p[0]],
                    )
                } else {
                    (
                        [[1., 0.], [0., 1.], [1., 1.]],
                        [i + 1, i + map.width, i + map.width + 1],
                        [1. - p[1], 1. - p[0], p[0] + p[1] - 1.],
                    )
                };
                let distance = if bary.iter().all(|&v| v >= 0.) {
                    0.
                } else {
                    [(0, 1), (1, 2), (2, 0)]
                        .into_iter()
                        .map(|(a, b)| {
                            let a = corners[a];
                            let b = corners[b];
                            let d = [b[0] - a[0], b[1] - a[1]];
                            let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1])
                                / (d[0] * d[0] + d[1] * d[1]))
                                .clamp(0., 1.);
                            (p[0] - a[0] - t * d[0]).powi(2) + (p[1] - a[1] - t * d[1]).powi(2)
                        })
                        .fold(f64::INFINITY, f64::min)
                };
                // Bounded continuation over a pixel footprint, not arbitrary filling.
                if distance > 0.75f64.powi(2) || best.is_some_and(|(d, _)| distance >= d) {
                    continue;
                }
                if let Some(value) = sample_triangle::<ATTRIBUTES>(map, colors, ids, bary) {
                    best = Some((distance, value));
                }
            }
        }
    }
    best.map(|(_, value)| value)
}

fn interpolate_observed<const ATTRIBUTES: bool>(
    map: &DepthMap,
    triangles: &[u8],
    colors: &[[u8; 3]],
    uv: [f64; 2],
) -> Option<(f64, f64, V3)> {
    let [x, y] = uv;
    if !x.is_finite()
        || !y.is_finite()
        || x < 0.
        || y < 0.
        || x >= map.width.saturating_sub(1) as f64
        || y >= map.height.saturating_sub(1) as f64
    {
        return None;
    }
    let i = y as usize * map.width + x as usize;
    let (u, v) = (x.fract(), y.fract());
    let (bit, ids, bary) = if u + v <= 1. {
        (1, [i, i + map.width, i + 1], [1. - u - v, v, u])
    } else {
        (
            2,
            [i + 1, i + map.width, i + map.width + 1],
            [1. - v, 1. - u, u + v - 1.],
        )
    };
    if triangles[i] & bit == 0 {
        return None;
    }
    sample_triangle::<ATTRIBUTES>(map, colors, ids, bary)
}
fn sample_triangle<const ATTRIBUTES: bool>(
    map: &DepthMap,
    colors: &[[u8; 3]],
    ids: [usize; 3],
    bary: [f64; 3],
) -> Option<(f64, f64, V3)> {
    let (mut inverse, mut weight, mut rgb) = (0., 0., [0.; 3]);
    let positive = bary.map(|v| v.max(0.));
    let total = positive.iter().sum::<f64>();
    for k in 0..3 {
        let z = map.depth[ids[k]];
        if z <= 0. || !z.is_finite() {
            return None;
        }
        inverse += bary[k] / z;
        // Validation only needs depth; monomorphization removes attribute loads.
        if ATTRIBUTES {
            weight += positive[k] / total * map.confidence[ids[k]] as f64;
            rgb = add(rgb, colors[ids[k]].map(|v| v as f64 * positive[k] / total));
        }
    }
    if !inverse.is_finite() || inverse <= 0. {
        return None;
    }
    Some((1. / inverse, weight, rgb))
}

fn position(key: Key, origin: V3, step: f64) -> V3 {
    add(origin, key.map(|v| v as f64 * step))
}

fn extract(
    keys: &[Key],
    field: &FastMap<Key, Node>,
    origin: V3,
    step: f64,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Surface> {
    let mut output = Surface::default();
    let mut edges = FastMap::<(Key, Key), u32>::default();
    let mut unique = FastSet::default();
    for (ci, &cell) in keys.iter().enumerate() {
        if ci % 512 == 0 {
            cancelled(progress, "volume-surface", ci, keys.len())?;
        }
        let corners = CORNERS.map(|c| std::array::from_fn(|k| cell[k] + c[k]));
        let nodes = corners.map(|key| field.get(&key).copied());
        for tet in TETS {
            if tet.iter().any(|&i| nodes[i].is_none()) {
                continue;
            }
            let (mut negatives, mut positives) = ([0; 4], [0; 4]);
            let (mut nn, mut np) = (0, 0);
            for i in tet {
                if nodes[i].unwrap().distance < 0. {
                    negatives[nn] = i;
                    nn += 1;
                } else {
                    positives[np] = i;
                    np += 1;
                }
            }
            if nn == 0 || np == 0 {
                continue;
            }
            let negative = &negatives[..nn];
            let positive = &positives[..np];
            let mut crossings = [0; 4];
            let mut nc = 0;
            for &a in negative {
                for &b in positive {
                    let (mut va, mut vb) = (nodes[a].unwrap(), nodes[b].unwrap());
                    let (mut a, mut b) = (corners[a], corners[b]);
                    if a > b {
                        std::mem::swap(&mut a, &mut b);
                        std::mem::swap(&mut va, &mut vb);
                    }
                    let edge = if va.distance == 0. {
                        (a, a)
                    } else if vb.distance == 0. {
                        (b, b)
                    } else {
                        (a, b)
                    };
                    let id = if let Some(&id) = edges.get(&edge) {
                        id
                    } else {
                        if output.positions.len() == MAX_SURFACE_VERTICES {
                            return Err("Volume vertex budget exceeded".into());
                        }
                        let t = va.distance / (va.distance - vb.distance);
                        let p = add(
                            scale(position(a, origin, step), 1. - t),
                            scale(position(b, origin, step), t),
                        );
                        let color = add(scale(va.color, 1. - t), scale(vb.color, t));
                        let id = output.positions.len() as u32;
                        output.positions.push(p);
                        output
                            .colors
                            .push(color.map(|v| v.round().clamp(0., 255.) as u8));
                        edges.insert(edge, id);
                        id
                    };
                    crossings[nc] = id;
                    nc += 1;
                }
            }
            let center = |ids: &[usize]| {
                scale(
                    ids.iter().fold([0.; 3], |sum, &i| {
                        add(sum, position(corners[i], origin, step))
                    }),
                    1. / ids.len() as f64,
                )
            };
            let direction = sub(center(positive), center(negative));
            let tris = [
                [crossings[0], crossings[1], crossings[2]],
                [crossings[1], crossings[3], crossings[2]],
            ];
            for mut tri in tris.into_iter().take(if nc == 3 { 1 } else { 2 }) {
                let [a, b, c] = tri.map(|i| output.positions[i as usize]);
                let normal = cross(sub(b, a), sub(c, a));
                if norm(normal) < step * step * 1e-10 {
                    continue;
                }
                if dot(normal, direction) < 0. {
                    tri.swap(1, 2);
                }
                let mut sorted = tri;
                sorted.sort_unstable();
                if unique.insert(sorted) {
                    if output.triangles.len() == MAX_SURFACE_TRIANGLES {
                        return Err("Volume triangle budget exceeded".into());
                    }
                    output.triangles.push(tri);
                }
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volume_capacity_is_included_in_upfront_planning() {
        let images = vec![crate::Image {
            width: 48,
            height: 48,
            rgb: vec![0; 48 * 48 * 3],
            focal: 100.,
        }];
        let sparse = Reconstruction {
            cameras: vec![Some(crate::camera::Camera::identity(100., 24., 24.))],
            points: vec![],
            input_images: 1,
            reprojection_rmse: 0.,
        };
        let old = super::super::estimated_working_bytes(&images, &sparse, &DenseOptions::default())
            .unwrap();
        let new = super::super::estimated_working_bytes(
            &images,
            &sparse,
            &DenseOptions {
                shared_volume: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(new - old, WORKING_BYTES);
    }
    #[test]
    fn inverse_depth_interpolation_respects_observed_triangle_domain() {
        let map = DepthMap {
            image: 0,
            width: 2,
            height: 2,
            step: 1.,
            depth: vec![4., 2., 4., 2.],
            confidence: vec![1.; 4],
            neighbors: vec![],
        };
        let colors = vec![[100; 3]; 4];
        for uv in [[0.25, 0.25], [0.75, 0.75]] {
            let expected = 1. / (0.25 + uv[0] * 0.25);
            assert!(
                (interpolate::<true>(&map, &[3, 0, 0, 0], &colors, uv).unwrap().0 - expected).abs() < 1e-12
            );
        }
        assert!(interpolate_observed::<true>(&map, &[1, 0, 0, 0], &colors, [0.75, 0.75]).is_none());
        assert!(
            (interpolate::<true>(&map, &[1, 0, 0, 0], &colors, [0.75, 0.75])
                .unwrap()
                .0
                - 1. / 0.4375)
                .abs()
                < 1e-12
        );
        assert!(interpolate::<true>(&map, &[0; 4], &colors, [0.25, 0.25]).is_none());
        assert!(interpolate::<true>(&map, &[3; 4], &colors, [f64::NAN, 0.]).is_none());
    }
    #[test]
    fn depth_only_interpolation_preserves_acceptance_and_exact_depth() {
        let mut map = DepthMap {
            image: 0, width: 2, height: 2, step: 1.,
            depth: vec![4., 2., 4., 2.],
            confidence: vec![0.2, 0.6, 0.9, 1.], neighbors: vec![],
        };
        let colors = [[10, 30, 70], [200, 20, 50], [0, 255, 80], [70, 80, 90]];
        for invalid_depth in [4., 0., -1., f64::NAN, f64::INFINITY] {
            map.depth[0] = invalid_depth;
            for bits in [0, 1, 2, 3] {
                for uv in [[0.25, 0.25], [0.75, 0.75], [0.99, 0.01],
                           [-0.01, 0.5], [1., 0.5], [f64::NAN, 0.]] {
                    let mask = [bits, 0, 0, 0];
                    let full = interpolate::<true>(&map, &mask, &colors, uv);
                    // Empty attribute storage ensures validation never reads it.
                    let depth = interpolate::<false>(&map, &mask, &[], uv);
                    assert_eq!(full.map(|v| v.0.to_bits()), depth.map(|v| v.0.to_bits()));
                }
            }
        }
    }
    #[test]
    fn plane_is_shared_oriented_and_open_at_unknown_boundary() {
        let mut field = FastMap::default();
        for z in -2..=2 {
            for y in -2..=2 {
                for x in -2..=2 {
                    field.insert(
                        [x, y, z],
                        Node {
                            distance: z as f64 - 0.25,
                            color: [20., 40., 60.],
                        },
                    );
                }
            }
        }
        let mut keys: Vec<_> = field.keys().copied().collect();
        keys.sort_unstable();
        let mesh = extract(&keys, &field, [0.; 3], 1., &mut |_, _, _| true).unwrap();
        assert!(!mesh.triangles.is_empty());
        assert!(mesh.positions.iter().all(|p| (p[2] - 0.25).abs() < 1e-12));
        let mut edges = HashMap::new();
        for t in mesh.triangles {
            let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
            assert!(cross(sub(b, a), sub(c, a))[2] > 0.);
            for [a, b] in [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]] {
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        assert!(edges.values().all(|&n| n <= 2));
        assert!(edges.values().any(|&n| n == 1));
    }
    #[test]
    fn unknown_nodes_and_cancellation_produce_no_partial_mesh() {
        let keys = vec![[0, 0, 0]];
        assert!(
            extract(&keys, &FastMap::default(), [0.; 3], 1., &mut |_, _, _| true)
                .unwrap()
                .triangles
                .is_empty()
        );
        assert_eq!(
            extract(&keys, &FastMap::default(), [0.; 3], 1., &mut |_, _, _| false).unwrap_err(),
            "Cancelled"
        );
    }
}

#[cfg(test)]
#[test]
fn retry_budget_and_cancellation_contract() {
    let mut diagnostics = super::DenseDiagnostics::default();
    let mut calls = 0;
    let result = bounded_attempts(&mut diagnostics, &mut |_,_,_| true, |_,_| {
        calls += 1;
        if calls == 1 { Err(AttemptError::NodeBudget) } else { Ok(Surface::default()) }
    });
    assert!(result.is_ok()); assert_eq!(calls,2);
    assert_eq!(diagnostics.volume_spacing_scale,Some(1.25));
    calls = 0;
    let result = bounded_attempts(&mut diagnostics, &mut |_,_,_| true, |_,_| {
        calls += 1; Err(AttemptError::NodeBudget)
    });
    assert!(matches!(result,Err(ref e) if e == "Volume node budget exceeded"));
    assert_eq!(calls,4); assert_eq!(diagnostics.volume_attempts,4);
    calls = 0;
    let result = bounded_attempts(&mut diagnostics, &mut |_,_,_| false, |_,_| {
        calls += 1; Err(AttemptError::NodeBudget)
    });
    assert!(matches!(result,Err(ref e) if e == "Cancelled")); assert_eq!(calls,1);
    calls = 0;
    let result = bounded_attempts(&mut diagnostics, &mut |_,_,_| true, |_,_| {
        calls += 1; Err(AttemptError::Other("invalid geometry".into()))
    });
    assert!(matches!(result,Err(ref e) if e == "invalid geometry")); assert_eq!(calls,1);
}
