//! Explicit pixel approximation of an SVG's rendered alpha silhouette.
//!
//! This path is opt-in: it must never serve as an implicit vector import
//! fallback. A returned contour follows pixel cell boundaries, so callers must
//! report the raster resolution and alpha threshold with the result.
use crate::{Error, Result};
use planar_geometry::rings::Rings;
use resvg::{tiny_skia, usvg};

const MAX_RASTER_SIZE: u32 = 2048;
const MAX_PIXELS: usize = 4 * 1024 * 1024;
const MAX_POINTS: usize = 500_000;
const MAX_LAYER_PIXELS: f64 = 16. * 1024. * 1024.;
const MAX_RENDER_WORK: f64 = 64. * 1024. * 1024.;

fn invalid(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_INVALID_DATA", message)
}

fn limit(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_LIMIT", message)
}

/// Render into a bounded transparent bitmap and trace its thresholded alpha.
///
/// `raster_size` is the longest viewport edge in pixels. Contours use CAD
/// millimetres with Y pointing up, positive outer winding and negative holes.
/// Fully opaque pixels are included when `alpha_threshold` is exactly one.
pub(crate) fn contours(
    tree: &usvg::Tree,
    width_mm: f64,
    height_mm: f64,
    raster_size: u32,
    alpha_threshold: f64,
) -> Result<Rings> {
    if !width_mm.is_finite() || !height_mm.is_finite() || width_mm <= 0. || height_mm <= 0. {
        return Err(invalid(
            "SVG silhouette dimensions must be positive and finite.",
        ));
    }
    if !(1..=MAX_RASTER_SIZE).contains(&raster_size) {
        return Err(limit(
            "SVG silhouette raster size must be between 1 and 2048 pixels.",
        ));
    }
    if !alpha_threshold.is_finite() || !(0.01..=1.).contains(&alpha_threshold) {
        return Err(invalid(
            "SVG silhouette alpha threshold must be between 0.01 and 1.",
        ));
    }
    let size = tree.size();
    let longest = f64::from(size.width().max(size.height()));
    let scale = f64::from(raster_size) / longest;
    let width = (f64::from(size.width()) * scale).ceil().max(1.) as u32;
    let height = (f64::from(size.height()) * scale).ceil().max(1.) as u32;
    if width as usize * height as usize > MAX_PIXELS {
        return Err(limit("SVG silhouette exceeds 4 million raster pixels."));
    }
    let sx = f64::from(width) / f64::from(size.width());
    let sy = f64::from(height) / f64::from(size.height());
    if !(sx as f32).is_finite() || !(sy as f32).is_finite() {
        return Err(limit(
            "SVG silhouette viewport scale exceeds the renderer's numeric range.",
        ));
    }
    let mut budget = RenderBudget::default();
    budget.group(tree.root(), sx, sy, 0)?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| limit("SVG silhouette bitmap allocation failed."))?;
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(sx as f32, sy as f32),
        &mut pixmap.as_mut(),
    );
    let minimum_alpha = (alpha_threshold * 255.).ceil() as u8;
    let pixels: Vec<bool> = pixmap
        .pixels()
        .iter()
        .map(|p| p.alpha() >= minimum_alpha)
        .collect();
    trace(
        &pixels,
        width as usize,
        height as usize,
        width_mm,
        height_mm,
        MAX_POINTS,
    )
}

/// resvg creates temporary layers for filters, masks and group opacity. Bound
/// those too: a small output bitmap alone does not bound a large filter region.
#[derive(Default)]
struct RenderBudget {
    nodes: usize,
    work: f64,
    geometry_work: usize,
}

impl RenderBudget {
    fn group(&mut self, group: &usvg::Group, sx: f64, sy: f64, depth: usize) -> Result<()> {
        if depth > 64 {
            return Err(limit("SVG silhouette rendering exceeds depth 64."));
        }
        if group.should_isolate() {
            let bbox = group.abs_layer_bounding_box();
            let pixels = (f64::from(bbox.width()) * sx + 4.).ceil()
                * (f64::from(bbox.height()) * sy + 4.).ceil();
            if !pixels.is_finite() || pixels > MAX_LAYER_PIXELS {
                return Err(limit(
                    "SVG silhouette effect layer exceeds 16 million pixels.",
                ));
            }
            let primitives: usize = group
                .filters()
                .iter()
                .map(|filter| filter.primitives().len())
                .sum();
            // Each filter primitive may retain its result and allocate scratch
            // images. This is a conservative work estimate, not a memory claim.
            self.work += pixels * (1. + primitives as f64 * 4.);
            if self.work > MAX_RENDER_WORK {
                return Err(limit(
                    "SVG silhouette effects exceed the rendering work budget.",
                ));
            }
        }
        for node in group.children() {
            self.nodes += 1;
            if self.nodes > 20_000 {
                return Err(limit("SVG silhouette rendering exceeds 20000 nodes."));
            }
            if let usvg::Node::Group(child) = node {
                self.group(child, sx, sy, depth + 1)?;
            }
            if let usvg::Node::Image(image) = node
                && !matches!(image.kind(), usvg::ImageKind::SVG(_)) {
                    let size = image.size();
                    let pixels = f64::from(size.width()) * f64::from(size.height());
                    if pixels > MAX_LAYER_PIXELS {
                        return Err(limit(
                            "SVG silhouette embedded image exceeds 16 million decoded pixels.",
                        ));
                    }
                    self.work += pixels;
                }
            if let usvg::Node::Path(path) = node {
                self.geometry_work += path.data().segments().count();
                if let Some(intervals) = path.stroke().and_then(|stroke| stroke.dasharray()) {
                    let centerline = path.stroke_centerline().ok_or_else(|| {
                        crate::Error::new(
                            "E_IMPORT_INVALID_DATA",
                            "SVG stroke transform is invalid",
                        )
                    })?;
                    self.geometry_work += crate::svg::check_dash_budget(&centerline, intervals)?;
                }
                if self.geometry_work > MAX_POINTS {
                    return Err(limit(
                        "SVG silhouette geometry exceeds 500000 segments and dashes.",
                    ));
                }
                if path.is_visible() {
                    let bounds = path.abs_stroke_bounding_box();
                    let paints =
                        usize::from(path.fill().is_some()) + usize::from(path.stroke().is_some());
                    self.work += (f64::from(bounds.width()) * sx).ceil()
                        * (f64::from(bounds.height()) * sy).ceil()
                        * paints as f64;
                }
                for paint in path
                    .fill()
                    .map(|fill| fill.paint())
                    .into_iter()
                    .chain(path.stroke().map(|stroke| stroke.paint()))
                {
                    if let usvg::Paint::Pattern(pattern) = paint {
                        let transform = tiny_skia::Transform::from_scale(sx as f32, sy as f32)
                            .pre_concat(path.abs_transform())
                            .pre_concat(pattern.transform());
                        let (px, py) = transform.get_scale();
                        let pixels = (f64::from(pattern.rect().width()) * f64::from(px)).round()
                            * (f64::from(pattern.rect().height()) * f64::from(py)).round();
                        if !pixels.is_finite() || pixels > MAX_LAYER_PIXELS {
                            return Err(limit(
                                "SVG silhouette pattern tile exceeds 16 million pixels.",
                            ));
                        }
                        self.work += pixels;
                        // A pattern subroot is rendered using the tile scale,
                        // not the outer document's viewport scale.
                        self.group(pattern.root(), f64::from(px), f64::from(py), depth + 1)?;
                    }
                }
            }
            if self.work > MAX_RENDER_WORK {
                return Err(limit(
                    "SVG silhouette effects exceed the rendering work budget.",
                ));
            }
            if !matches!(node, usvg::Node::Path(_)) {
                // Embedded SVG images and mask/filter subroots have their own
                // coordinate systems. Carry the referencing node's scale into
                // them; otherwise an enlarged image can evade tile limits.
                let transform = tiny_skia::Transform::from_scale(sx as f32, sy as f32)
                    .pre_concat(node.abs_transform());
                let (inner_sx, inner_sy) = transform.get_scale();
                let mut result = Ok(());
                node.subroots(|root| {
                    if result.is_ok() {
                        result =
                            self.group(root, f64::from(inner_sx), f64::from(inner_sy), depth + 1);
                    }
                });
                result?;
            }
        }
        Ok(())
    }
}

/// Directed cell edges keep the filled cell to their right in image space.
/// Right turns at diagonal contacts keep four-connected islands separate.
fn trace(
    pixels: &[bool],
    width: usize,
    height: usize,
    width_mm: f64,
    height_mm: f64,
    max_points: usize,
) -> Result<Rings> {
    if width == 0 || height == 0 || pixels.len() != width * height || pixels.len() > MAX_PIXELS {
        return Err(invalid("SVG silhouette pixel grid is invalid."));
    }
    let stride = width + 1;
    let mut edges = vec![0u8; stride * (height + 1)];
    let mut remaining = 0usize;
    let mut add = |x: usize, y: usize, direction: u8| {
        edges[y * stride + x] |= 1 << direction;
        remaining += 1;
    };
    for y in 0..height {
        for x in 0..width {
            if !pixels[y * width + x] {
                continue;
            }
            if y == 0 || !pixels[(y - 1) * width + x] {
                add(x, y, 0);
            }
            if x + 1 == width || !pixels[y * width + x + 1] {
                add(x + 1, y, 1);
            }
            if y + 1 == height || !pixels[(y + 1) * width + x] {
                add(x + 1, y + 1, 2);
            }
            if x == 0 || !pixels[y * width + x - 1] {
                add(x, y + 1, 3);
            }
        }
    }
    let mut rings = Vec::new();
    let mut point_count = 0usize;
    for start in 0..edges.len() {
        while edges[start] != 0 {
            let mut vertex = start;
            let mut direction = edges[start].trailing_zeros() as u8;
            let first_direction = direction;
            let mut corners = vec![start];
            loop {
                if edges[vertex] & (1 << direction) == 0 || remaining == 0 {
                    return Err(invalid("SVG silhouette boundary is inconsistent."));
                }
                edges[vertex] &= !(1 << direction);
                remaining -= 1;
                vertex = match direction {
                    0 => vertex + 1,
                    1 => vertex + stride,
                    2 => vertex - 1,
                    _ => vertex - stride,
                };
                if vertex == start {
                    // If start was on a straight edge, remove the unnecessary
                    // initial vertex as well as all intermediate collinear ones.
                    if direction == first_direction {
                        corners.remove(0);
                    }
                    break;
                }
                let next_direction = [(direction + 1) % 4, direction, (direction + 3) % 4]
                    .into_iter()
                    .find(|next| edges[vertex] & (1 << next) != 0)
                    .ok_or_else(|| invalid("SVG silhouette boundary is open."))?;
                if next_direction != direction {
                    corners.push(vertex);
                    if point_count + corners.len() > max_points {
                        return Err(limit("SVG silhouette exceeds 500000 contour points."));
                    }
                }
                direction = next_direction;
            }
            if corners.len() < 4 {
                return Err(invalid("SVG silhouette boundary is degenerate."));
            }
            point_count += corners.len();
            if point_count > max_points {
                return Err(limit("SVG silhouette exceeds 500000 contour points."));
            }
            let ring = corners
                .into_iter()
                .rev()
                .map(|v| {
                    [
                        (v % stride) as f64 * width_mm / width as f64,
                        height_mm - (v / stride) as f64 * height_mm / height as f64,
                    ]
                })
                .collect();
            rings.push(ring);
        }
    }
    if remaining != 0 {
        return Err(invalid("SVG silhouette boundary was not fully traced."));
    }
    Ok(rings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use planar_geometry::rings::{area, inside};

    fn tree(contents: &str) -> usvg::Tree {
        usvg::Tree::from_str(
            &format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16">{contents}</svg>"#),
            &usvg::Options::default(),
        ).unwrap()
    }

    #[test]
    fn rectangle_has_exact_mm_bounds_and_only_four_corners() {
        let rings = contours(
            &tree(r#"<rect x="2" y="3" width="10" height="8"/>"#),
            32.,
            48.,
            16,
            1.,
        )
        .unwrap();
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 4);
        assert_eq!(area(&rings[0]), 480.);
        assert!(rings[0].contains(&[4., 39.]));
        assert!(rings[0].contains(&[24., 15.]));
    }

    #[test]
    fn compound_fill_preserves_hole_and_island_winding() {
        let rings = contours(
            &tree(r#"<path fill-rule="evenodd" d="M0 0H16V16H0Z M2 2V14H14V2Z M6 6H10V10H6Z"/>"#),
            16.,
            16.,
            16,
            1.,
        )
        .unwrap();
        assert_eq!(rings.len(), 3);
        assert_eq!(rings.iter().map(|r| area(r)).sum::<f64>(), 128.);
        assert_eq!(rings.iter().filter(|r| area(r) < 0.).count(), 1);
        assert!(inside([1., 1.], &rings));
        assert!(!inside([4., 4.], &rings));
        assert!(inside([8., 8.], &rings));
    }

    #[test]
    fn diagonal_pixels_remain_separate_closed_islands() {
        let rings = trace(&[true, false, false, true], 2, 2, 2., 2., MAX_POINTS).unwrap();
        assert_eq!(rings.len(), 2);
        assert!(rings.iter().all(|r| r.len() == 4 && area(r) == 1.));
    }

    #[test]
    fn every_three_by_three_grid_preserves_area_and_pixel_membership() {
        for mask in 0..512 {
            let pixels: Vec<bool> = (0..9).map(|i| mask & (1 << i) != 0).collect();
            let rings = trace(&pixels, 3, 3, 3., 3., MAX_POINTS).unwrap();
            assert_eq!(
                rings.iter().map(|r| area(r)).sum::<f64>(),
                pixels.iter().filter(|&&p| p).count() as f64
            );
            for (index, &filled) in pixels.iter().enumerate() {
                assert_eq!(
                    inside([(index % 3) as f64 + 0.5, 2.5 - (index / 3) as f64], &rings),
                    filled
                );
            }
        }
    }

    #[test]
    fn alpha_threshold_and_luminance_mask_change_the_silhouette() {
        let masked = tree(
            r##"<defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="16" height="16"><rect width="8" height="16" fill="white"/><rect x="8" width="8" height="16" fill="black"/></mask></defs><rect width="16" height="16" mask="url(#m)"/>"##,
        );
        let rings = contours(&masked, 16., 16., 16, 0.5).unwrap();
        assert_eq!(rings.iter().map(|r| area(r)).sum::<f64>(), 128.);
        let translucent = tree(r#"<rect width="16" height="16" opacity="0.25"/>"#);
        assert!(
            contours(&translucent, 16., 16., 16, 0.5)
                .unwrap()
                .is_empty()
        );
        assert_eq!(contours(&translucent, 16., 16., 16, 0.2).unwrap().len(), 1);
    }

    #[test]
    fn filter_output_is_rendered_before_thresholding() {
        let filtered = tree(
            r##"<defs><filter id="f" x="0" y="0" width="100%" height="100%"><feOffset dx="4" dy="0"/></filter></defs><rect x="2" y="2" width="8" height="8" filter="url(#f)"/>"##,
        );
        let rings = contours(&filtered, 16., 16., 16, 1.).unwrap();
        assert_eq!(rings.iter().map(|r| area(r)).sum::<f64>(), 32.);
        assert!(inside([8., 10.], &rings));
        assert!(!inside([4., 10.], &rings));
    }

    #[test]
    fn pattern_opacity_produces_disjoint_geometry() {
        let patterned = tree(
            r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="4" height="4"/></pattern></defs><rect width="16" height="16" fill="url(#p)"/>"##,
        );
        let rings = contours(&patterned, 16., 16., 16, 1.).unwrap();
        assert_eq!(rings.len(), 4);
        assert_eq!(rings.iter().map(|r| area(r)).sum::<f64>(), 64.);
    }

    #[test]
    fn embedded_png_alpha_controls_image_silhouette() {
        // A 2x1 RGBA PNG: opaque black followed by a transparent pixel.
        let image = tree(
            r#"<image x="0" y="0" width="16" height="16" preserveAspectRatio="none" image-rendering="pixelated" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAAD0lEQVR4nGNgYGD4D8QMAAUEAQCwBUiSAAAAAElFTkSuQmCC"/>"#,
        );
        let rings = contours(&image, 16., 16., 16, 0.5).unwrap();
        assert_eq!(rings.iter().map(|r| area(r)).sum::<f64>(), 128.);
        assert!(inside([4., 8.], &rings));
        assert!(!inside([12., 8.], &rings));
    }

    #[test]
    fn invalid_options_and_excessive_effect_work_fail_closed() {
        let plain = tree(r#"<rect width="16" height="16"/>"#);
        for threshold in [0., f64::NAN, 1.1] {
            assert!(contours(&plain, 16., 16., 16, threshold).is_err());
        }
        assert_eq!(
            contours(&plain, 16., 16., 2049, 0.5).unwrap_err().code,
            "E_IMPORT_LIMIT"
        );
        assert!(contours(&plain, f64::INFINITY, 16., 16, 0.5).is_err());
        assert_eq!(
            trace(&[true, false, false, true], 2, 2, 2., 2., 4)
                .unwrap_err()
                .code,
            "E_IMPORT_LIMIT"
        );
        let large = tree(
            r##"<defs><filter id="f" x="-1000%" y="-1000%" width="2100%" height="2100%"><feGaussianBlur stdDeviation="1"/></filter></defs><rect width="16" height="16" filter="url(#f)"/>"##,
        );
        assert_eq!(
            contours(&large, 16., 16., 512, 0.5).unwrap_err().code,
            "E_IMPORT_LIMIT"
        );
    }

    #[test]
    fn enlarged_nested_svg_cannot_bypass_pattern_tile_budget() {
        use base64::Engine;
        let nested = base64::engine::general_purpose::STANDARD.encode(r##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><defs><pattern id="p" patternUnits="userSpaceOnUse" width="20" height="20"><rect width="1" height="1"/></pattern></defs><rect width="1" height="1" fill="url(#p)"/></svg>"##);
        let image = tree(&format!(
            r#"<image width="16" height="16" href="data:image/svg+xml;base64,{nested}"/>"#
        ));
        // Inspect admission only: the rejected renderer tile would otherwise
        // be 10240x10240 pixels despite the 512x512 final bitmap.
        let error = RenderBudget::default()
            .group(image.root(), 32., 32., 0)
            .unwrap_err();
        assert_eq!(error.code, "E_IMPORT_LIMIT");
        assert!(error.message.contains("pattern tile"));
    }

    #[test]
    fn svg_admission_review_regressions() {
        use base64::Engine;
        use value_codec::json;
        let wrap = |contents: &str| {
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="40mm" viewBox="0 0 40 40">{contents}</svg>"#
            )
        };
        for contents in [
            r#"<path d="M0 0H10" stroke="black" style="vector-effect:non-rotation"/>"#,
            r#"<rect width="10" height="10"/><rect width="oops" height="10"/>"#,
        ] {
            assert!(crate::svg::dispatch(json!({"source":wrap(contents)})).is_err());
        }
        for nested in [
            r#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script><rect width="10" height="10"/></svg>"#,
            r#"<svg xmlns="urn:other"><rect width="10" height="10"/></svg>"#,
            r#"<svg xmlns="http://www.w3.org/2000/svg"><image href="https://example.test/image.png"/></svg>"#,
        ] {
            let encoded = base64::engine::general_purpose::STANDARD.encode(nested);
            let source = wrap(&format!(
                r#"<rect width="10" height="10"/><image width="10" height="10" href="data:image/svg+xml;base64,{encoded}"/>"#
            ));
            assert!(crate::svg::dispatch(json!({"source":source,"action":"preview"})).is_err());
        }
        let heavy = format!(
            r##"<defs><path id="p" d="M0 0{}Z"/></defs>{}"##,
            "L1 1".repeat(2000),
            r##"<use href="#p"/>"##.repeat(260)
        );
        assert_eq!(
            crate::svg::dispatch(json!({"source":wrap(&heavy),"action":"preview"}))
                .unwrap_err()
                .code,
            "E_IMPORT_LIMIT"
        );
        let css = wrap(
            r#"<style>.icon16px{fill:none}</style><rect class="icon16px" width="10" height="10"/><rect x="20" width="2" height="2"/>"#,
        );
        let normalized = crate::svg::dispatch(json!({"source":css,"legacyDpi":true})).unwrap();
        assert_eq!(normalized["regions"].as_array().unwrap().len(), 1);
        let quoted = wrap(
            r#"<rect width="10" height="10" style="stroke-width:1px;font-family:&quot;Noto Sans&quot;"/>"#,
        );
        assert!(crate::svg::dispatch(json!({"source":quoted,"legacyDpi":true})).is_ok());
        let prefixed = r#"<s:svg xmlns:s="http://www.w3.org/2000/svg" width="10mm" height="10mm" viewBox="0 0 10 10"><s:rect width="5" height="5"/></s:svg>"#;
        assert!(
            !crate::svg::dispatch(json!({"source":prefixed})).unwrap()["regions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn dense_dashes_and_accumulated_paint_work_fail_before_rendering() {
        let tree = usvg::Tree::from_str(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><path d="M0 50H100" fill="none" stroke="black" stroke-dasharray=".00001 .00001"/></svg>"#, &usvg::Options::default(),
        ).unwrap();
        assert_eq!(
            contours(&tree, 100., 100., 512, 0.5).unwrap_err().code,
            "E_IMPORT_LIMIT"
        );
        let source = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='512' height='512'>{}</svg>",
            "<rect width='512' height='512'/>".repeat(300)
        );
        let tree = usvg::Tree::from_str(&source, &usvg::Options::default()).unwrap();
        assert_eq!(
            contours(&tree, 512., 512., 512, 0.5).unwrap_err().code,
            "E_IMPORT_LIMIT"
        );
    }
}
