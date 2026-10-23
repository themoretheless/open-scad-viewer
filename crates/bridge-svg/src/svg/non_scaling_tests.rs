//! Acceptance oracles for SVG non-scaling strokes in the outermost SVG viewport.
//!
//! The dimensions below are independently derived in millimetres. Geometry output
//! uses CAD Y-up coordinates; SVG and the raster assertions use Y-down coordinates.

use super::*;

const STROKE: &str =
    r#"fill="none" stroke="black" stroke-width="2mm" vector-effect="non-scaling-stroke""#;

fn document(body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">{body}</svg>"#
    )
}

fn parse_at(source: &str, dpi: f64) -> Value {
    dispatch(json!({"source":source,"dpi":dpi,"tolerance":0.001})).unwrap()
}

fn parse_body(body: &str) -> Value {
    parse_at(&document(body), 96.)
}

// Fill and stroke, as well as markers, can be separate overlapping regions.
// Measuring their union prevents an implementation from passing by double-counting.
fn solid(value: &Value) -> Rings {
    let mut result = Rings::new();
    for region in value["regions"].as_array().unwrap() {
        let contours = value_codec::from_value::<Rings>(region["contours"].clone()).unwrap();
        let rule = if region["fillRule"].as_str() == Some("evenodd") {
            FillRule::EvenOdd
        } else {
            FillRule::NonZero
        };
        result =
            rings::planar_with_rules(&result, &contours, "union", FillRule::NonZero, rule).unwrap();
    }
    result
}

fn area_of(contours: &Rings) -> f64 {
    contours
        .iter()
        .map(|ring| rings::area(ring))
        .sum::<f64>()
        .abs()
}

fn bounds_of(contours: &Rings) -> [f64; 4] {
    assert!(!contours.is_empty(), "expected visible geometry");
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for point in contours.iter().flatten() {
        bounds[0] = bounds[0].min(point[0]);
        bounds[1] = bounds[1].min(point[1]);
        bounds[2] = bounds[2].max(point[0]);
        bounds[3] = bounds[3].max(point[1]);
    }
    bounds
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() <= tolerance,
        "actual {actual}, expected {expected}, tolerance {tolerance}"
    );
}

fn assert_geometry(value: &Value, area: f64, svg_bounds: [f64; 4], tolerance: f64) {
    let contours = solid(value);
    close(area_of(&contours), area, tolerance);
    let height = value["heightMm"].as_f64().unwrap();
    let expected = [
        svg_bounds[0],
        height - svg_bounds[3],
        svg_bounds[2],
        height - svg_bounds[1],
    ];
    for (actual, expected) in bounds_of(&contours).into_iter().zip(expected) {
        close(actual, expected, tolerance);
    }
}

fn assert_same_geometry(left: &Value, right: &Value, tolerance: f64) {
    let left = solid(left);
    let right = solid(right);
    close(area_of(&left), area_of(&right), tolerance);
    for (left, right) in bounds_of(&left).into_iter().zip(bounds_of(&right)) {
        close(left, right, tolerance);
    }
    let difference =
        rings::planar_with_rules(&left, &right, "xor", FillRule::NonZero, FillRule::NonZero)
            .unwrap();
    close(area_of(&difference), 0., tolerance);
}

fn render(source: &str) -> resvg::tiny_skia::Pixmap {
    let tree = parse_tree(source, 96., None).unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(1000, 1000).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            1000. / tree.size().width(),
            1000. / tree.size().height(),
        ),
        &mut pixmap.as_mut(),
    );
    pixmap
}

fn pixel(pixmap: &resvg::tiny_skia::Pixmap, x_mm: f64, y_mm: f64) -> [u8; 4] {
    let pixel = pixmap
        .pixel((x_mm * 10.) as u32, (y_mm * 10.) as u32)
        .unwrap();
    [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
}

#[test]
fn non_scaling_affine_transform_preserves_physical_width() {
    let cases = [
        ("", "M10 20H20", 20., [10., 19., 20., 21.]),
        ("scale(3)", "M10 20H20", 60., [30., 59., 60., 61.]),
        ("scale(3 .5)", "M10 20H20", 60., [30., 9., 60., 11.]),
        (
            "translate(60 0) rotate(90)",
            "M10 20H20",
            20.,
            [39., 10., 41., 20.],
        ),
        (
            "matrix(-2 0 0 1 80 0)",
            "M10 20H20",
            40.,
            [40., 19., 60., 21.],
        ),
        (
            "matrix(1 0 1 1 0 0)",
            "M10 10V20",
            20. * 2_f64.sqrt(),
            [
                20. - 0.5_f64.sqrt(),
                10. - 0.5_f64.sqrt(),
                30. + 0.5_f64.sqrt(),
                20. + 0.5_f64.sqrt(),
            ],
        ),
    ];
    for (transform, path, area, bounds) in cases {
        let value = parse_body(&format!(
            r#"<path d="{path}" transform="{transform}" {STROKE}/>"#
        ));
        assert_geometry(&value, area, bounds, 0.005);
    }
}

#[test]
fn non_scaling_square_and_round_caps_are_built_after_transform() {
    for (cap, area) in [("square", 64.), ("round", 60. + std::f64::consts::PI)] {
        let value = parse_body(&format!(
            r#"<path d="M10 20H20" transform="scale(3)" stroke-linecap="{cap}" {STROKE}/>"#
        ));
        assert_geometry(&value, area, [29., 59., 61., 61.], 0.02);
    }
}

#[test]
fn non_scaling_closed_stroke_preserves_hole_and_join_geometry() {
    for (join, expected_area) in [
        ("miter", 168.),
        ("round", 168. - (4. - std::f64::consts::PI)),
    ] {
        let value = parse_body(&format!(
            r#"<rect x="10" y="20" width="10" height="6" transform="scale(3 2)" stroke-linejoin="{join}" {STROKE}/>"#
        ));
        assert_geometry(&value, expected_area, [29., 39., 61., 53.], 0.03);
        let contours = solid(&value);
        assert_eq!(contours.len(), 2, "the interior must remain a hole");
        assert!(rings::area(&contours[0]) * rings::area(&contours[1]) < 0.);
        close(
            contours
                .iter()
                .map(|ring| rings::area(ring).abs())
                .fold(f64::INFINITY, f64::min),
            280.,
            0.01,
        );
    }
}

#[test]
fn non_scaling_host_is_outermost_viewport_even_with_nested_svg() {
    let nested = parse_body(&format!(
        r#"<g transform="scale(2)"><svg x="5" y="5" width="20" height="20" viewBox="0 0 10 10"><path d="M2 5H8" {STROKE}/></svg></g>"#
    ));
    assert_geometry(&nested, 48., [18., 29., 42., 31.], 0.005);

    let anisotropic_root = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 100" preserveAspectRatio="none"><path d="M10 20H20" {STROKE}/></svg>"#
    );
    assert_geometry(
        &parse_at(&anisotropic_root, 96.),
        20.,
        [10., 9., 20., 11.],
        0.005,
    );
}

#[test]
fn non_scaling_width_resolves_physical_pixel_and_percentage_units() {
    for dpi in [96., 192.] {
        for (width, expected_width) in [
            ("1mm", 1.),
            ("2", 2. * 25.4 / dpi),
            ("2px", 2. * 25.4 / dpi),
            ("10%", 10. * 25.4 / dpi),
        ] {
            let source = document(&format!(
                r#"<path d="M10 20H20" fill="none" stroke="black" stroke-width="{width}" vector-effect="non-scaling-stroke"/>"#
            ));
            assert_geometry(
                &parse_at(&source, dpi),
                10. * expected_width,
                [
                    10.,
                    20. - expected_width / 2.,
                    20.,
                    20. + expected_width / 2.,
                ],
                0.005,
            );
        }
    }
}

#[test]
fn non_scaling_dash_lengths_and_offsets_are_measured_in_host_space() {
    for (offset, spans) in [
        ("0", vec![[20., 25.], [30., 35.], [40., 45.], [50., 55.]]),
        (
            "2mm",
            vec![[20., 23.], [28., 33.], [38., 43.], [48., 53.], [58., 60.]],
        ),
    ] {
        let value = parse_body(&format!(
            r#"<path d="M10 20H30" transform="scale(2)" stroke-dasharray="5mm 5mm" stroke-dashoffset="{offset}" {STROKE}/>"#
        ));
        let contours = solid(&value);
        assert_eq!(
            contours.len(),
            spans.len(),
            "dash topology, offset {offset}"
        );
        close(area_of(&contours), 40., 0.01);
        let mut actual: Vec<_> = contours
            .iter()
            .map(|ring| bounds_of(&vec![ring.clone()]))
            .collect();
        actual.sort_by(|a, b| a[0].total_cmp(&b[0]));
        for (bounds, span) in actual.into_iter().zip(spans) {
            for (actual, expected) in bounds.into_iter().zip([span[0], 59., span[1], 61.]) {
                close(actual, expected, 0.005);
            }
        }
    }
}

#[test]
fn non_scaling_css_cascade_and_explicit_inheritance_match_attributes() {
    let non_scaling = parse_body(&format!(
        r#"<path d="M10 20H20" transform="scale(3)" {STROKE}/>"#
    ));
    let ordinary = parse_body(
        r#"<path d="M10 20H20" transform="scale(3)" fill="none" stroke="black" stroke-width="2mm"/>"#,
    );
    let path = r#"<path class="n" d="M10 20H20" transform="scale(3)" fill="none" stroke="black" stroke-width="2mm"/>"#;
    for css in [
        ".n { vector-effect: non-scaling-stroke }",
        "path { vector-effect: none } .n { vector-effect: non-scaling-stroke }",
        ".n { vector-effect: non-scaling-stroke !important; vector-effect: none }",
        ".n { vector-effect: none } .n { vector-effect: non-scaling-stroke }",
    ] {
        assert_same_geometry(
            &parse_body(&format!("<style>{css}</style>{path}")),
            &non_scaling,
            0.01,
        );
    }
    assert_same_geometry(
        &parse_body(&format!(
            r#"<g vector-effect="non-scaling-stroke">{path}</g>"#
        )),
        &ordinary,
        0.01,
    );
    let inherited = path.replace("class=\"n\"", "vector-effect=\"inherit\"");
    assert_same_geometry(
        &parse_body(&format!(
            r#"<g vector-effect="non-scaling-stroke">{inherited}</g>"#
        )),
        &non_scaling,
        0.01,
    );
    let inline = path.replace("class=\"n\"", "class=\"n\" style=\"vector-effect:none\"");
    assert_same_geometry(
        &parse_body(&format!(
            "<style>.n {{ vector-effect: non-scaling-stroke !important }}</style>{inline}"
        )),
        &non_scaling,
        0.01,
    );
}

#[test]
fn non_scaling_use_resolves_inherit_per_instance_without_leaking_from_defs() {
    let definitions = r#"<defs><g vector-effect="non-scaling-stroke"><path id="p" vector-effect="inherit" d="M2 5H8" fill="none" stroke="black" stroke-width="2mm"/></g></defs>"#;
    let actual = parse_body(&format!(
        r##"{definitions}<use href="#p" transform="scale(2)" vector-effect="non-scaling-stroke"/><use href="#p" x="30" transform="scale(2)" vector-effect="none"/>"##
    ));
    let expected = parse_body(&format!(
        r#"<path d="M2 5H8" transform="scale(2)" {STROKE}/><path d="M32 5H38" transform="scale(2)" fill="none" stroke="black" stroke-width="2mm"/>"#
    ));
    assert_eq!(solid(&actual).len(), 2);
    assert_same_geometry(&actual, &expected, 0.01);

    let no_inherit = definitions.replace(r#" vector-effect="inherit""#, "");
    let actual = parse_body(&format!(
        r##"{no_inherit}<use href="#p" transform="scale(2)" vector-effect="non-scaling-stroke"/>"##
    ));
    let expected = parse_body(
        r#"<path d="M2 5H8" transform="scale(2)" fill="none" stroke="black" stroke-width="2mm"/>"#,
    );
    assert_same_geometry(&actual, &expected, 0.01);
}

#[test]
fn non_scaling_text_matches_equivalent_glyphs_at_double_font_size() {
    let style = r#"font-family="Noto Sans" fill="none" stroke="black" stroke-width="1mm" vector-effect="non-scaling-stroke""#;
    let transformed = parse_body(&format!(
        r#"<text x="10" y="20" font-size="10" transform="scale(2)" {style}>HI</text>"#
    ));
    let explicit = parse_body(&format!(
        r#"<text x="20" y="40" font-size="20" {style}>HI</text>"#
    ));
    assert!(area_of(&solid(&transformed)) > 1.);
    assert_same_geometry(&transformed, &explicit, 0.03);
}

#[test]
fn non_scaling_marker_units_distinguish_stroke_width_from_user_space() {
    for (units, area, bounds) in [
        ("strokeWidth", 84., [30., 9., 66., 14.]),
        ("userSpaceOnUse", 69., [30., 9., 69., 11.]),
    ] {
        let source = document(&format!(
            r##"<defs><marker id="m" markerUnits="{units}" markerWidth="10" markerHeight="10" refX="0" refY="0" overflow="visible"><rect width="3" height="2" fill="red"/></marker></defs><path d="M10 20H20" transform="scale(3 .5)" marker-end="url(#m)" {STROKE}/>"##
        ));
        assert_geometry(&parse_at(&source, 96.), area, bounds, 0.01);
        let pixmap = render(&source);
        assert_eq!(pixel(&pixmap, 62., 10.5), [255, 0, 0, 255]);
        assert_eq!(pixel(&pixmap, 31., 9.5), [0, 0, 0, 255]);
    }
}

#[test]
fn non_scaling_paint_order_and_element_opacity_survive_outline_rendering() {
    for (order, inside_color) in [
        ("normal", [0, 0, 255, 255]),
        ("stroke fill", [255, 0, 0, 255]),
    ] {
        let body = format!(
            r#"<rect x="10" y="20" width="10" height="6" transform="scale(3 2)" fill="red" stroke="blue" stroke-width="2mm" vector-effect="non-scaling-stroke" paint-order="{order}"/>"#
        );
        let source = document(&body);
        let pixmap = render(&source);
        assert_eq!(pixel(&pixmap, 30.5, 42.), inside_color);
        assert_eq!(pixel(&pixmap, 29.5, 42.), [0, 0, 255, 255]);
        assert_geometry(&parse_at(&source, 96.), 448., [29., 39., 61., 53.], 0.01);

        let translucent = render(&document(&body.replace("<rect ", "<rect opacity=\"0.5\" ")));
        let overlap = pixel(&translucent, 30.5, 42.);
        assert!(
            (overlap[3] as i16 - 128).abs() <= 1,
            "opacity applied once: {overlap:?}"
        );
    }
}

#[test]
fn non_scaling_gradient_stroke_retains_original_paint_coordinates() {
    let source = document(
        r##"<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="10" y1="20" x2="20" y2="20"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient></defs><rect x="10" y="20" width="10" height="6" transform="scale(3 2)" fill="none" stroke="url(#g)" stroke-width="2mm" vector-effect="non-scaling-stroke"/>"##,
    );
    let pixmap = render(&source);
    assert_eq!(pixel(&pixmap, 29.5, 46.), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixmap, 60.5, 46.), [0, 0, 255, 255]);
    let middle = pixel(&pixmap, 45., 39.5);
    assert!(middle[0] > 115 && middle[0] < 140, "{middle:?}");
    assert!(middle[2] > 115 && middle[2] < 140, "{middle:?}");
    assert_eq!(middle[3], 255);
    assert_geometry(&parse_at(&source, 96.), 168., [29., 39., 61., 53.], 0.01);
}

#[test]
fn non_scaling_clips_use_the_declared_space_and_unstroked_object_bbox() {
    let root_clip = parse_body(&format!(
        r##"<defs><clipPath id="c" clipPathUnits="userSpaceOnUse"><rect x="40" width="10" height="100"/></clipPath></defs><g clip-path="url(#c)"><path d="M10 20H20" transform="scale(3)" {STROKE}/></g>"##
    ));
    assert_geometry(&root_clip, 20., [40., 59., 50., 61.], 0.005);

    let object_clip = document(&format!(
        r##"<defs><clipPath id="c" clipPathUnits="objectBoundingBox"><rect width=".5" height="1"/></clipPath></defs><rect x="10" y="20" width="10" height="6" transform="scale(3 2)" clip-path="url(#c)" {STROKE}/>"##
    ));
    assert_geometry(
        &parse_at(&object_clip, 96.),
        40.,
        [30., 40., 45., 52.],
        0.01,
    );
    let pixmap = render(&object_clip);
    assert_eq!(pixel(&pixmap, 30.5, 46.), [0, 0, 0, 255]);
    assert_eq!(pixel(&pixmap, 29.5, 46.)[3], 0);
    assert_eq!(pixel(&pixmap, 45.5, 40.5)[3], 0);
}

#[test]
fn non_scaling_normalized_svg_roundtrip_preserves_physical_geometry_and_paint() {
    let source = document(
        r##"<defs><clipPath id="c"><rect x="25" y="35" width="50" height="30"/></clipPath></defs><g clip-path="url(#c)"><rect x="10" y="20" width="10" height="6" transform="scale(3 2)" fill="red" stroke="blue" stroke-width="2mm" vector-effect="non-scaling-stroke" paint-order="stroke fill" opacity=".5"/></g>"##,
    );
    for dpi in [96., 192.] {
        let original = parse_at(&source, dpi);
        let normalized = original["normalizedSvg"].as_str().unwrap();
        let reparsed = parse_at(normalized, 96.);
        close(reparsed["widthMm"].as_f64().unwrap(), 100., 0.0001);
        close(reparsed["heightMm"].as_f64().unwrap(), 100., 0.0001);
        assert_same_geometry(&original, &reparsed, 0.03);
        let pixmap = render(normalized);
        let inside = pixel(&pixmap, 30.5, 42.);
        let outside = pixel(&pixmap, 29.5, 42.);
        assert!(
            inside[0] >= 126 && inside[2] == 0 && (127..=128).contains(&inside[3]),
            "{inside:?}"
        );
        assert!(
            outside[2] >= 126 && outside[0] == 0 && (127..=128).contains(&outside[3]),
            "{outside:?}"
        );
    }
}

#[test]
fn non_scaling_silhouette_matches_vector_contours_and_preserves_hole() {
    let source = document(&format!(
        r#"<rect x="10" y="20" width="10" height="6" transform="scale(3 2)" {STROKE}/>"#
    ));
    let vector = parse_at(&source, 96.);
    for source in [&source, vector["normalizedSvg"].as_str().unwrap()] {
        let silhouette = dispatch(json!({
            "source":source,"dpi":96,"geometryMode":"silhouette",
            "rasterSize":1000,"alphaThreshold":0.5
        }))
        .unwrap();
        assert_geometry(&silhouette, 168., [29., 39., 61., 53.], 0.3);
        assert_eq!(solid(&silhouette).len(), 2);
        assert_same_geometry(&vector, &silhouette, 0.3);
    }
}

#[test]
fn non_scaling_pattern_content_uses_each_painted_instances_host_transform() {
    // The pattern has a single horizontal stripe. Its period scales with the
    // painted rectangle, but its physical stroke width remains one millimetre.
    for scale in [1., 2.] {
        let source = document(&format!(
            r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="10" height="10"><path d="M0 5H10" fill="none" stroke="black" stroke-width="1mm" vector-effect="non-scaling-stroke"/></pattern></defs><rect x="10" y="10" width="10" height="10" transform="scale({scale})" fill="url(#p)"/>"##
        ));
        let silhouette = dispatch(json!({
            "source":source,"dpi":96,"geometryMode":"silhouette",
            "rasterSize":1000,"alphaThreshold":0.5
        }))
        .unwrap();
        assert_geometry(
            &silhouette,
            10. * scale,
            [
                10. * scale,
                15. * scale - 0.5,
                20. * scale,
                15. * scale + 0.5,
            ],
            0.3,
        );
    }

    let source = document(
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="10" height="10"><path id="stripe" d="M0 5H10" fill="none" stroke="black" stroke-width="1mm" vector-effect="non-scaling-stroke"/></pattern></defs><rect x="10" y="10" width="10" height="10" fill="url(#p)"/><rect x="20" y="10" width="10" height="10" transform="scale(2)" fill="url(#p)"/>"##,
    );
    let preview = dispatch(json!({"source":source,"action":"preview","dpi":96})).unwrap();
    let normalized = preview["normalizedSvg"].as_str().unwrap();
    let xml = usvg::roxmltree::Document::parse(normalized).unwrap();
    let ids: Vec<_> = xml
        .descendants()
        .filter_map(|node| node.attribute("id"))
        .collect();
    let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
    assert_eq!(
        ids.len(),
        unique.len(),
        "resource clones must not duplicate IDs"
    );
    assert_eq!(
        xml.descendants()
            .filter(|node| node.has_tag_name("pattern"))
            .count(),
        2
    );
    for source in [source.as_str(), normalized] {
        let silhouette = dispatch(json!({
            "source":source,"dpi":96,"geometryMode":"silhouette",
            "rasterSize":1000,"alphaThreshold":0.5
        }))
        .unwrap();
        assert_geometry(&silhouette, 30., [10., 14.5, 60., 30.5], 0.3);
        assert_eq!(solid(&silhouette).len(), 2);
    }
}

#[test]
fn non_scaling_mask_content_resolves_user_space_and_object_bbox_instances() {
    let cases = [
        (
            r##"<defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="100"><path d="M10 20H20" fill="none" stroke="white" stroke-width="2mm" vector-effect="non-scaling-stroke"/></mask></defs><g transform="scale(3 .5)" mask="url(#m)"><rect width="30" height="40" fill="red"/></g>"##,
            60.,
            [30., 9., 60., 11.],
        ),
        (
            r##"<defs><mask id="m" maskContentUnits="objectBoundingBox"><path d="M.1 .5H.9" fill="none" stroke="white" stroke-width="1mm" vector-effect="non-scaling-stroke"/></mask></defs><rect x="10" y="10" width="20" height="20" transform="scale(2)" fill="red" mask="url(#m)"/>"##,
            32.,
            [24., 39.5, 56., 40.5],
        ),
    ];
    for (body, area, bounds) in cases {
        let source = document(body);
        let preview = dispatch(json!({"source":source,"action":"preview","dpi":96})).unwrap();
        for source in [source.as_str(), preview["normalizedSvg"].as_str().unwrap()] {
            let silhouette = dispatch(json!({
                "source":source,"dpi":96,"geometryMode":"silhouette",
                "rasterSize":1000,"alphaThreshold":0.5
            }))
            .unwrap();
            assert_geometry(&silhouette, area, bounds, 0.3);
        }
    }
}

#[test]
fn non_scaling_resource_budget_is_checked_before_any_clone_or_mutation() {
    let source = document(
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="10" height="10"><path id="stripe" d="M0 5H10" fill="none" stroke="black" stroke-width="1mm" vector-effect="non-scaling-stroke"/></pattern></defs><rect x="10" y="10" width="10" height="10" fill="url(#p)"/><rect x="20" y="10" width="10" height="10" transform="scale(2)" fill="url(#p)"/>"##,
    );
    let mut tree = usvg::Tree::from_str(&source, &usvg::Options::default()).unwrap();
    let before = tree.to_string(&usvg::WriteOptions::default());
    assert_eq!(
        tree.resolve_non_scaling_contexts(5),
        Err(usvg::NonScalingContextError::LimitExceeded)
    );
    assert_eq!(tree.to_string(&usvg::WriteOptions::default()), before);
    tree.resolve_non_scaling_contexts(5000).unwrap();
    assert_eq!(tree.patterns().len(), 2);
    assert_ne!(tree.patterns()[0].id(), tree.patterns()[1].id());

    let many_points = format!("M0 5{}", "L1 5 ".repeat(300));
    let source = document(&format!(
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="10" height="10"><path d="{many_points}" fill="none" stroke="black" stroke-width="1mm" vector-effect="non-scaling-stroke"/></pattern></defs>{}"##,
        r##"<rect width="10" height="10" fill="url(#p)"/>"##.repeat(20)
    ));
    let mut tree = usvg::Tree::from_str(&source, &usvg::Options::default()).unwrap();
    let before = tree.to_string(&usvg::WriteOptions::default());
    assert_eq!(
        tree.resolve_non_scaling_contexts(1000),
        Err(usvg::NonScalingContextError::LimitExceeded)
    );
    assert_eq!(tree.to_string(&usvg::WriteOptions::default()), before);
}
