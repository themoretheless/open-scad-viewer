//! Reciprocal depth/reprojection checks define which observations may enter fusion.
use super::{DenseDiagnostics, DenseOptions, DepthMap, cancelled, world_points};
use crate::{Image, Reconstruction, Result, camera::Camera, math::*};

#[derive(Clone, Debug)]
pub(super) struct Sample {
    pub position: V3,
    /// Points toward the observing camera; opposing sheet sides cannot fuse.
    pub normal: Option<V3>,
    pub color: [u8; 3],
    pub footprint: f64,
    /// NCC confidence times sqrt(number of supporting views, including self).
    pub weight: f64,
    pub source: usize,
}
#[derive(Default)]
pub(super) struct ObservedPatch {
    pub samples: Vec<Sample>,
    pub triangles: Vec<[u32; 3]>,
}

// Reciprocal depths per pixel, 0 where invalid; 1./z of the same z is the same bit.
fn inverse_depths(map: &DepthMap) -> Vec<f64> {
    map.depth
        .iter()
        .map(|&z| if z.is_finite() && z > 0. { 1. / z } else { 0. })
        .collect()
}

fn normal(map: &DepthMap, points: &[V3], camera: &Camera, i: usize, tolerance: f64) -> Option<V3> {
    let (x, y) = (i % map.width, i / map.width);
    let z = map.depth[i];
    if z <= 0. {
        return None;
    }
    let p = points[i];
    let continuous = |j: usize| map.depth[j] > 0. && (map.depth[j] - z).abs() < tolerance * z;
    // Prefer central differences; one-sided derivatives keep valid boundary samples.
    let direction = |negative: Option<usize>, positive: Option<usize>| {
        let negative = negative.filter(|&j| continuous(j));
        let positive = positive.filter(|&j| continuous(j));
        match (negative, positive) {
            (Some(a), Some(b)) => Some(sub(points[b], points[a])),
            (None, Some(b)) => Some(sub(points[b], p)),
            (Some(a), None) => Some(sub(p, points[a])),
            _ => None,
        }
    };
    let dx = direction(
        x.checked_sub(1).map(|_| i - 1),
        (x + 1 < map.width).then_some(i + 1),
    )?;
    let dy = direction(
        y.checked_sub(1).map(|_| i - map.width),
        (y + 1 < map.height).then_some(i + map.width),
    )?;
    let n = cross(dx, dy);
    if norm(n) < 1e-15 {
        return None;
    }
    let n = unit(n);
    Some(if dot(n, sub(camera.center(), p)) < 0. {
        scale(n, -1.)
    } else {
        n
    })
}

// Both predicates use the same bounds, validity and planar-continuity rules.
fn planar_edge(
    inv: &[f64],
    width: usize,
    height: usize,
    a: usize,
    b: usize,
    tolerance: f64,
) -> bool {
    continuous_edge(inv, width, height, a, b, tolerance, false)
}
fn curved_edge(
    inv: &[f64],
    width: usize,
    height: usize,
    a: usize,
    b: usize,
    tolerance: f64,
) -> bool {
    continuous_edge(inv, width, height, a, b, tolerance, true)
}
fn continuous_edge(
    inv: &[f64],
    width: usize,
    height: usize,
    a: usize,
    b: usize,
    tolerance: f64,
    allow_curvature: bool,
) -> bool {
    let ax = (a % width) as isize;
    let ay = (a / width) as isize;
    let bx = (b % width) as isize;
    let by = (b / width) as isize;
    let sample = |x: isize, y: isize| -> Option<f64> {
        if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
            return None;
        }
        let iz = inv[y as usize * width + x as usize];
        (iz != 0.).then_some(iz)
    };
    let left = sample(2 * ax - bx, 2 * ay - by);
    let right = sample(2 * bx - ax, 2 * by - ay);
    let Some(u) = sample(ax, ay) else {
        return false;
    };
    let Some(v) = sample(bx, by) else {
        return false;
    };
    let limit = tolerance * 0.1 * u.min(v);
    if let (Some(left), Some(right)) = (left, right)
        && (left - 2. * u + v).abs() <= limit
        && (u - 2. * v + right).abs() <= limit
    {
        return true;
    }
    if !allow_curvature {
        return false;
    }
    let slope = v - u;
    let continuous = |neighbor: f64| {
        neighbor * slope > 0.
            && neighbor.abs() >= 0.5 * slope.abs()
            && neighbor.abs() <= 2. * slope.abs()
    };
    // Missing support may use one measured slope; contradictory support may not.
    match (left, right) {
        (Some(left), Some(right)) => continuous(u - left) && continuous(right - v),
        (Some(left), None) => continuous(u - left),
        (None, Some(right)) => continuous(right - v),
        (None, None) => false,
    }
}

pub(super) fn filter(
    images: &[Image],
    sparse: &Reconstruction,
    maps: &[DepthMap],
    options: &DenseOptions,
    diagnostics: &mut DenseDiagnostics,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Vec<ObservedPatch>> {
    // Compute local geometry once; reciprocal checks reuse it for every source.
    let total_rows: usize = maps.iter().map(|map| map.height).sum();
    let mut points = Vec::with_capacity(maps.len());
    let mut inverse = Vec::with_capacity(maps.len());
    for map in maps {
        let camera = sparse.cameras[map.image].as_ref().unwrap();
        points.push(world_points(map, camera));
        inverse.push(inverse_depths(map));
    }
    let mut normals = Vec::with_capacity(maps.len());
    let mut normal_rows = 0;
    for (map_index, map) in maps.iter().enumerate() {
        let camera = sparse.cameras[map.image].as_ref().unwrap();
        let mut values = Vec::with_capacity(map.depth.len());
        for y in 0..map.height {
            cancelled(progress, "consistency", normal_rows + y, total_rows * 2)?;
            for x in 0..map.width {
                values.push(normal(
                    map,
                    &points[map_index],
                    camera,
                    y * map.width + x,
                    options.relative_depth_tolerance,
                ));
            }
        }
        normal_rows += map.height;
        normals.push(values);
    }
    let mut patches = Vec::new();
    let mut completed_rows = 0;
    for (map_index, map) in maps.iter().enumerate() {
        let camera = sparse.cameras[map.image].as_ref().unwrap();
        let mut patch = ObservedPatch::default();
        let mut ids = vec![u32::MAX; map.depth.len()];
        for y in 0..map.height {
            cancelled(
                progress,
                "consistency",
                total_rows + completed_rows + y,
                total_rows * 2,
            )?;
            for x in 0..map.width {
                let i = y * map.width + x;
                let z = map.depth[i];
                if z <= 0. {
                    continue;
                }
                let p = points[map_index][i];
                let n = normals[map_index][i];
                let mut support = 0;
                for (other_index, other) in maps.iter().enumerate() {
                    if other.image == map.image {
                        continue;
                    }
                    // Opt-in COLMAP-style check: only the views selected for
                    // depth estimation may vote; the default polls every map.
                    if options.selected_sources_consistency && !map.neighbors.contains(&other.image)
                    {
                        continue;
                    }
                    let nc = sparse.cameras[other.image].as_ref().unwrap();
                    let cp = nc.camera_point(p);
                    let Some(uv) = nc.project_camera_point(cp) else {
                        continue;
                    };
                    if uv.iter().any(|x| !x.is_finite()) {
                        continue;
                    }
                    let (ix, iy) = (
                        (uv[0] / other.step).round() as isize,
                        (uv[1] / other.step).round() as isize,
                    );
                    if ix < 0 || iy < 0 || ix >= other.width as isize || iy >= other.height as isize
                    {
                        continue;
                    }
                    let j = iy as usize * other.width + ix as usize;
                    let dz = other.depth[j];
                    let expected = cp[2];
                    if dz <= 0.
                        || (dz - expected).abs() > options.relative_depth_tolerance * expected
                    {
                        continue;
                    }
                    if let (Some(a), Some(b)) = (n, normals[other_index][j])
                        && dot(a, b) < 0.5
                    {
                        continue;
                    }
                    let Some(back) = camera.project(points[other_index][j]) else {
                        continue;
                    };
                    let error = ((back[0] / map.step - x as f64).powi(2)
                        + (back[1] / map.step - y as f64).powi(2))
                    .sqrt();
                    if error <= options.reprojection_tolerance {
                        support += 1;
                    }
                }
                if support < options.min_support_views {
                    continue;
                }
                let color_index = ((y as f64 * map.step) as usize * images[map.image].width
                    + (x as f64 * map.step) as usize)
                    * 3;
                ids[i] = patch.samples.len() as u32;
                patch.samples.push(Sample {
                    position: p,
                    normal: n,
                    color: images[map.image].rgb[color_index..color_index + 3]
                        .try_into()
                        .unwrap(),
                    footprint: z * map.step / camera.focal,
                    weight: (map.confidence[i] as f64).max(0.01) * (1. + support as f64).sqrt(),
                    source: map.image,
                });
            }
        }
        for y in 0..map.height.saturating_sub(1) {
            for x in 0..map.width.saturating_sub(1) {
                let a = y * map.width + x;
                for triangle in [
                    [a, a + map.width, a + 1],
                    [a + 1, a + map.width, a + map.width + 1],
                ] {
                    if triangle.iter().any(|&i| ids[i] == u32::MAX) {
                        continue;
                    }
                    let low = triangle
                        .iter()
                        .map(|&i| map.depth[i])
                        .fold(f64::INFINITY, f64::min);
                    let high = triangle.iter().map(|&i| map.depth[i]).fold(0., f64::max);
                    if high - low > options.relative_depth_tolerance * low
                        && ![
                            (triangle[0], triangle[1]),
                            (triangle[1], triangle[2]),
                            (triangle[2], triangle[0]),
                        ]
                        .iter()
                        .all(|&(a, b)| {
                            planar_edge(
                                &inverse[map_index],
                                map.width,
                                map.height,
                                a,
                                b,
                                options.relative_depth_tolerance,
                            )
                        })
                    {
                        if !options.dual_scale {
                            continue;
                        }
                        if ![
                            (triangle[0], triangle[1]),
                            (triangle[1], triangle[2]),
                            (triangle[2], triangle[0]),
                        ]
                        .iter()
                        .all(|&(a, b)| {
                            curved_edge(
                                &inverse[map_index],
                                map.width,
                                map.height,
                                a,
                                b,
                                options.relative_depth_tolerance,
                            )
                        }) {
                            continue;
                        }
                        let world = triangle.map(|i| points[map_index][i]);
                        let center = scale(add(add(world[0], world[1]), world[2]), 1. / 3.);
                        let mut support = 0;
                        for other in maps {
                            if other.image == map.image {
                                continue;
                            }
                            if options.selected_sources_consistency
                                && !map.neighbors.contains(&other.image)
                            {
                                continue;
                            }
                            let nc = sparse.cameras[other.image].as_ref().unwrap();
                            let cp = nc.camera_point(center);
                            let Some(uv) = nc.project_camera_point(cp) else {
                                continue;
                            };
                            if uv.iter().any(|v| !v.is_finite()) {
                                continue;
                            }
                            let x = (uv[0] / other.step).round() as isize;
                            let y = (uv[1] / other.step).round() as isize;
                            if x < 0
                                || y < 0
                                || x >= other.width as isize
                                || y >= other.height as isize
                            {
                                continue;
                            }
                            let z = other.depth[y as usize * other.width + x as usize];
                            let expected = cp[2];
                            if z > 0.
                                && (z - expected).abs()
                                    <= options.relative_depth_tolerance * expected
                            {
                                support += 1;
                            }
                        }
                        if support < options.min_support_views {
                            continue;
                        }
                    }
                    patch.triangles.push(triangle.map(|i| ids[i]));
                }
            }
        }
        diagnostics.view_reports[map_index].consistent_samples = patch.samples.len();
        diagnostics.consistent_samples += patch.samples.len();
        completed_rows += map.height;
        patches.push(patch);
    }
    diagnostics.rejected_inconsistent_samples = diagnostics
        .photometric_samples
        .saturating_sub(diagnostics.consistent_samples);
    Ok(patches)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn map() -> DepthMap {
        DepthMap {
            image: 0,
            width: 4,
            height: 4,
            step: 1.,
            depth: vec![4.; 16],
            confidence: vec![1.; 16],
            neighbors: vec![1],
        }
    }
    #[test]
    fn selected_sources_option_limits_support_to_chosen_views() {
        let first = Camera::identity(100., 6., 6.);
        let mut second = first.clone();
        second.translation = [-0.04, 0., 0.];
        let mut third = first.clone();
        third.translation = [-0.08, 0., 0.];
        let maps: Vec<_> = (0..3)
            .map(|image| DepthMap {
                image,
                width: 12,
                height: 12,
                step: 1.,
                depth: vec![4.; 144],
                confidence: vec![1.; 144],
                // View 0 selected only view 1 as its depth-estimation source.
                neighbors: match image {
                    0 => vec![1],
                    1 => vec![0, 2],
                    _ => vec![0, 1],
                },
            })
            .collect();
        let image = Image {
            width: 48,
            height: 48,
            rgb: vec![128; 48 * 48 * 3],
            focal: 100.,
        };
        let sparse = Reconstruction {
            cameras: vec![Some(first), Some(second), Some(third)],
            points: vec![],
            input_images: 3,
            reprojection_rmse: 0.,
        };
        let options = DenseOptions {
            min_support_views: 2,
            max_source_views: 3,
            ..Default::default()
        };
        let run = |options: &DenseOptions| {
            let mut diagnostics = DenseDiagnostics {
                photometric_samples: 432,
                view_reports: vec![Default::default(); 3],
                ..Default::default()
            };
            filter(
                &[image.clone(), image.clone(), image.clone()],
                &sparse,
                &maps,
                options,
                &mut diagnostics,
                &mut |_, _, _| true,
            )
            .unwrap()
        };
        // Polling every map gives view 0 two supporters; restricting to the
        // selected sources leaves one, below the required support.
        let all = run(&options);
        assert!(!all[0].samples.is_empty());
        let selected_options = DenseOptions {
            selected_sources_consistency: true,
            ..options.clone()
        };
        let selected = run(&selected_options);
        assert!(selected[0].samples.is_empty());
        assert!(!selected[1].samples.is_empty());
        let again = run(&selected_options);
        let weights: Vec<f64> = selected[1].samples.iter().map(|s| s.weight).collect();
        let weights_again: Vec<f64> = again[1].samples.iter().map(|s| s.weight).collect();
        assert_eq!(weights, weights_again);
    }

    #[test]
    fn curved_boundary_requires_measured_support_without_contradiction() {
        let mut depths = map();
        let (curved, planar) = (
            |d: &DepthMap, a, b| curved_edge(&inverse_depths(d), d.width, d.height, a, b, 0.025),
            |d: &DepthMap, a, b| planar_edge(&inverse_depths(d), d.width, d.height, a, b, 0.025),
        );
        depths.depth.fill(0.);
        depths.depth[4] = 1. / 0.25;
        depths.depth[5] = 1. / 0.26;
        depths.depth[6] = 1. / 0.27;
        assert!(curved(&depths, 5, 6));
        assert!(!planar(&depths, 5, 6));
        depths.depth[7] = 1. / 0.20;
        assert!(!curved(&depths, 5, 6));
        depths.depth[7] = 1. / 0.28;
        assert!(planar(&depths, 5, 6));
        depths.depth[4] = 0.;
        assert!(curved(&depths, 5, 6));
        depths.depth[7] = 0.;
        assert!(!curved(&depths, 5, 6));
        depths.depth[4..8].copy_from_slice(&[4., 4., 2., 2.]);
        assert!(!curved(&depths, 5, 6));
    }

    #[test]
    fn normals_face_camera_and_do_not_bridge_depth_discontinuities() {
        let camera = Camera::identity(100., 2., 2.);
        let mut map = map();
        let points = world_points(&map, &camera);
        assert_eq!(
            normal(&map, &points, &camera, 5, 0.025).unwrap(),
            [0., 0., -1.]
        );
        map.depth[4] = 2.;
        map.depth[6] = 2.;
        let points = world_points(&map, &camera);
        assert!(normal(&map, &points, &camera, 5, 0.025).is_none());
    }
    #[test]
    fn overlapping_registered_depth_maps_become_one_observed_grid() {
        let first = Camera::identity(100., 6., 6.);
        let mut second = first.clone();
        // One exact depth pixel of camera translation gives a known overlap.
        second.translation = [-0.04, 0., 0.];
        let maps: Vec<_> = (0..2)
            .map(|image| DepthMap {
                image,
                width: 12,
                height: 12,
                step: 1.,
                depth: vec![4.; 144],
                confidence: vec![1.; 144],
                neighbors: vec![1 - image],
            })
            .collect();
        let image = Image {
            width: 48,
            height: 48,
            rgb: vec![128; 48 * 48 * 3],
            focal: 100.,
        };
        let sparse = Reconstruction {
            cameras: vec![Some(first), Some(second)],
            points: vec![],
            input_images: 2,
            reprojection_rmse: 0.,
        };
        let mut diagnostics = DenseDiagnostics {
            photometric_samples: 288,
            view_reports: vec![Default::default(); 2],
            ..Default::default()
        };
        let patches = filter(
            &[image.clone(), image],
            &sparse,
            &maps,
            &DenseOptions::default(),
            &mut diagnostics,
            &mut |_, _, _| true,
        )
        .unwrap();
        assert_eq!(diagnostics.consistent_samples, 264);
        let surface =
            super::super::fusion::fuse(&patches, true, &mut diagnostics, &mut |_, _, _| true)
                .unwrap();
        assert_eq!(surface.positions.len(), 132);
        assert_eq!(surface.triangles.len(), 220);
        assert_eq!(diagnostics.fused_samples, 132);
        assert!(surface.positions.iter().all(|p| (p[2] - 4.).abs() < 1e-12));
    }

    #[test]
    fn opposite_thin_sheet_sides_do_not_supply_false_depth_support() {
        let first = Camera::identity(100., 6., 6.);
        let mut second = first.clone();
        second.rotation = [[-1., 0., 0.], [0., 1., 0.], [0., 0., -1.]];
        second.translation = [0., 0., 8.];
        let maps: Vec<_> = (0..2)
            .map(|image| DepthMap {
                image,
                width: 12,
                height: 12,
                step: 1.,
                depth: vec![if image == 0 { 4. } else { 3.999 }; 144],
                confidence: vec![1.; 144],
                neighbors: vec![1 - image],
            })
            .collect();
        let image = Image {
            width: 48,
            height: 48,
            rgb: vec![128; 48 * 48 * 3],
            focal: 100.,
        };
        let sparse = Reconstruction {
            cameras: vec![Some(first), Some(second)],
            points: vec![],
            input_images: 2,
            reprojection_rmse: 0.,
        };
        let mut diagnostics = DenseDiagnostics {
            photometric_samples: 288,
            view_reports: vec![Default::default(); 2],
            ..Default::default()
        };
        let patches = filter(
            &[image.clone(), image],
            &sparse,
            &maps,
            &DenseOptions::default(),
            &mut diagnostics,
            &mut |_, _, _| true,
        )
        .unwrap();
        assert_eq!(diagnostics.consistent_samples, 0);
        assert!(patches.iter().all(|p| p.samples.is_empty()));
    }

    #[test]
    fn observed_triangles_never_cross_depth_jump() {
        let camera = Camera::identity(100., 2., 2.);
        let mut a = map();
        for y in 0..4 {
            a.depth[y * 4 + 2] = 5.;
            a.depth[y * 4 + 3] = 5.;
        }
        let b = DepthMap {
            image: 1,
            width: 4,
            height: 4,
            step: 1.,
            depth: a.depth.clone(),
            confidence: vec![1.; 16],
            neighbors: vec![0],
        };
        let image = Image {
            width: 48,
            height: 48,
            rgb: vec![128; 48 * 48 * 3],
            focal: 100.,
        };
        let sparse = Reconstruction {
            cameras: vec![Some(camera); 2],
            points: vec![],
            input_images: 2,
            reprojection_rmse: 0.,
        };
        let mut diagnostics = DenseDiagnostics {
            photometric_samples: 32,
            view_reports: vec![Default::default(); 2],
            ..Default::default()
        };
        let patches = filter(
            &[image.clone(), image],
            &sparse,
            &[a, b],
            &DenseOptions::default(),
            &mut diagnostics,
            &mut |_, _, _| true,
        )
        .unwrap();
        assert_eq!(diagnostics.consistent_samples, 32);
        for patch in patches {
            assert_eq!(patch.triangles.len(), 12);
            for triangle in patch.triangles {
                let z = triangle.map(|i| patch.samples[i as usize].position[2]);
                assert_eq!(z[0], z[1]);
                assert_eq!(z[1], z[2]);
            }
        }
    }
}

#[cfg(test)]
#[test]
fn planar_edge_accepts_tilt_and_rejects_depth_step() {
    let mut map = DepthMap {
        image: 0,
        width: 8,
        height: 8,
        step: 1.,
        depth: vec![0.; 64],
        confidence: vec![1.; 64],
        neighbors: vec![],
    };
    for y in 0..8 {
        for x in 0..8 {
            map.depth[y * 8 + x] = 1. / (0.2 + 0.015 * x as f64 + 0.01 * y as f64);
        }
    }
    assert!(planar_edge(&inverse_depths(&map), 8, 8, 27, 28, 0.03));
    assert!(planar_edge(&inverse_depths(&map), 8, 8, 27, 36, 0.03));
    for y in 0..8 {
        for x in 0..8 {
            map.depth[y * 8 + x] = if x < 4 { 3. } else { 4. };
        }
    }
    assert!(!planar_edge(&inverse_depths(&map), 8, 8, 27, 28, 0.03));
    assert!(!planar_edge(&inverse_depths(&map), 8, 8, 0, 1, 0.03));
}
