    use super::*;
    fn parse(source: &str) -> Value {
        dispatch(json!({"source":source,"dpi":96})).unwrap()
    }
    fn contours(value: &Value) -> Rings {
        value["regions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|r| value_codec::from_value::<Rings>(r["contours"].clone()).unwrap())
            .collect()
    }
    fn area(value: &Value) -> f64 {
        value["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                let rings = value_codec::from_value::<Rings>(r["contours"].clone()).unwrap();
                let rule = if r["fillRule"].as_str() == Some("evenodd") {
                    FillRule::EvenOdd
                } else {
                    FillRule::NonZero
                };
                rings::normalize(&rings, rule)
                    .unwrap()
                    .iter()
                    .map(|r| rings::area(r))
                    .sum::<f64>()
            })
            .sum()
    }
    fn bounds(value: &Value) -> [f64; 4] {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in contours(value).iter().flatten() {
            b[0] = b[0].min(p[0]);
            b[1] = b[1].min(p[1]);
            b[2] = b[2].max(p[0]);
            b[3] = b[3].max(p[1]);
        }
        b
    }
    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
    }
    #[test]
    fn open_scad_dimensions_and_viewbox_are_binary64() {
        for (source, expected) in [
            (
                "<svg width=\"10mm\" height=\"20mm\" viewBox=\"0 0 10 20\"><rect x=\"1\" y=\"2\" width=\"3\" height=\"5\"/></svg>",
                [1., 13., 4., 18.],
            ),
            (
                "<svg width=\"96px\" height=\"96px\" viewBox=\"0 0 96 96\"><rect width=\"96\" height=\"48\"/></svg>",
                [0., 12.7, 25.4, 25.4],
            ),
        ] {
            let result = dispatch(json!({"source":source,"legacyDpi":true})).unwrap();
            for (a, b) in bounds(&result).into_iter().zip(expected) {
                close(a, b);
            }
        }
    }
    #[test]
    fn css_use_symbol_nested_viewports_and_transform() {
        let result = parse(
            r##"<svg width="100mm" height="100mm" viewBox="0 0 100 100"><style>.paint { fill: red } .hidden { display: none }</style><defs><symbol id="s" viewBox="0 0 10 10"><rect class="paint" width="10" height="10"/></symbol></defs><g transform="translate(5 7)"><use href="#s" x="10" y="10" width="20" height="20"/><svg x="50" y="10" width="20" height="20" viewBox="0 0 10 10"><circle class="hidden" r="10"/><rect width="5" height="10"/></svg></g></svg>"##,
        );
        close(area(&result), 600.);
        let text = result["normalizedSvg"].as_str().unwrap();
        assert!(!text.contains("<use"));
        assert!(!text.contains("<symbol"));
        assert!(!text.contains("<style"));
    }
    #[test]
    fn quadratic_cubic_smooth_and_arc_paths_preserve_endpoints() {
        let result = parse(
            r#"<svg width="100mm" height="100mm" viewBox="0 0 100 100"><path d="M5 10 Q10 0 15 10 T25 10 C30 0 35 0 40 10 S50 20 55 10 A10 5 30 0 1 75 10 L75 50 H5 V10 Z"/></svg>"#,
        );
        let b = bounds(&result);
        close(b[0], 5.);
        close(b[2], 75.);
        close(b[1], 50.);
        assert!(contours(&result)[0].len() > 40);
    }
    #[test]
    fn dashed_strokes_have_caps_and_nonuniform_transforms() {
        let result = parse(
            r#"<svg width="100mm" height="100mm" viewBox="0 0 100 100"><path d="M10 10 H30" fill="none" stroke="red" stroke-width="2" stroke-linecap="square" stroke-dasharray="5 5" transform="scale(2 1)"/></svg>"#,
        );
        close(area(&result), 56.);
        for (a, b) in bounds(&result).into_iter().zip([18., 89., 52., 91.]) {
            close(a, b);
        }
    }
    #[test]
    fn clipping_unions_children_and_intersects_nested_clips() {
        let result = parse(
            r##"<svg width="100mm" height="100mm" viewBox="0 0 100 100"><defs><clipPath id="a"><rect width="10" height="20"/><rect x="10" width="10" height="10"/></clipPath><clipPath id="b"><rect x="5" width="10" height="100"/></clipPath></defs><g transform="translate(10 10)" clip-path="url(#a)"><g clip-path="url(#b)"><rect width="30" height="30"/></g></g></svg>"##,
        );
        close(area(&result), 150.);
        for (a, b) in bounds(&result).into_iter().zip([15., 70., 25., 90.]) {
            close(a, b);
        }
    }
    #[test]
    fn object_bounding_box_clip_and_evenodd_hole() {
        let result = parse(
            r##"<svg width="100mm" height="100mm" viewBox="0 0 100 100"><defs><clipPath id="a" clipPathUnits="objectBoundingBox"><rect x=".25" y=".25" width=".5" height=".5"/></clipPath></defs><rect x="10" y="20" width="40" height="20" clip-path="url(#a)"/><path fill-rule="evenodd" d="M60 0H90V30H60Z M65 5H85V25H65Z"/></svg>"##,
        );
        close(area(&result), 700.);
    }
    #[test]
    fn markers_and_gradient_paints_are_real_geometry() {
        let result = parse(
            r##"<svg width="100mm" height="100mm" viewBox="0 0 100 100"><defs><marker id="m" markerWidth="10" markerHeight="10" refX="0" refY="5" orient="auto" markerUnits="userSpaceOnUse"><path d="M0 0L10 5L0 10Z"/></marker><linearGradient id="g" gradientUnits="userSpaceOnUse"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient></defs><path d="M10 20H50" fill="none" stroke="url(#g)" stroke-width="2" marker-end="url(#m)"/></svg>"##,
        );
        assert!(
            area(&result) > 100.,
            "area {} bounds {:?}",
            area(&result),
            bounds(&result)
        );
        assert!(bounds(&result)[2] >= 60. - 1e-6);
    }
    #[test]
    fn latin_cyrillic_text_outlines_preview_and_reimport() {
        let result = parse(
            r#"<svg width="100mm" height="40mm" viewBox="0 0 100 40"><text x="5" y="25" font-size="20">Svg Привет</text></svg>"#,
        );
        assert!(contours(&result).len() > 10);
        assert!(area(&result) > 10.);
        let normalized = result["normalizedSvg"].as_str().unwrap();
        assert!(!normalized.contains("<text"));
        assert!(normalized.contains("mm\""));
        let second = parse(normalized);
        assert!((area(&result) - area(&second)).abs() < 0.01);
        assert!(!result["warnings"].as_array().unwrap().is_empty());
    }
    #[test]
    fn opacity_and_visibility_remove_invisible_geometry() {
        let result = parse(
            r#"<svg width="10" height="10"><g opacity="0"><rect width="10" height="10"/></g><rect visibility="hidden" width="10" height="10"/><rect fill-opacity="0" width="10" height="10"/></svg>"#,
        );
        assert!(contours(&result).is_empty());
    }
    #[test]
    fn effects_preserved_in_preview_require_explicit_silhouette() {
        let source = r##"<svg width="20" height="20"><defs><filter id="f"><feGaussianBlur stdDeviation="1"/></filter></defs><rect x="5" y="5" width="10" height="10" filter="url(#f)"/></svg>"##;
        let error = dispatch(json!({"source":source})).unwrap_err();
        assert_eq!(error.code, "E_IMPORT_UNSUPPORTED_FEATURE");
        assert!(error.message.contains("silhouette"));
        let preview = dispatch(json!({"source":source,"action":"preview"})).unwrap();
        assert!(
            preview["normalizedSvg"]
                .as_str()
                .unwrap()
                .contains("feGaussianBlur")
        );
        let result =
            dispatch(json!({"source":source,"geometryMode":"silhouette","rasterSize":128}))
                .unwrap();
        assert!(!contours(&result).is_empty());
    }
    #[test]
    fn malformed_geometry_security_and_reference_budgets_fail_explicitly() {
        for source in [
            "<svg><path d='M0 0 Lbad'/></svg>",
            "<svg><polygon points='1 2 3'/></svg>",
            "<svg><g transform='rotate(no)'/></svg>",
            "<!DOCTYPE svg><svg/>",
            "<svg><image href='file:///etc/passwd'/></svg>",
            "<svg><style>@import 'https://example.com'</style></svg>",
            "<svg><use href='#missing'/></svg>",
            "<svg><defs><g id='a'><use href='#a'/></g></defs></svg>",
            "<svg><script/></svg>",
            "<svg><foreignObject/></svg>",
            "<svg><animate/></svg>",
        ] {
            assert!(
                dispatch(json!({"source":source,"action":"preview"})).is_err(),
                "{source}"
            );
        }
        let source = format!("<svg>{}{}</svg>", "<g>".repeat(130), "</g>".repeat(130));
        assert_eq!(
            dispatch(json!({"source":source})).unwrap_err().code,
            "E_IMPORT_LIMIT"
        );
        assert!(dispatch(json!({"source":"<svg/>","fonts":["not base64"]})).is_err());
    }
    #[test]
    fn compatibility_px_preserves_css_identifiers_strings_and_xml_quotes() {
        let source = r#"<svg width="100" height="100"><style>.icon16px { fill:none }</style><rect class="icon16px" width="10" height="10"/><path d="M20 20H40" style="fill:none;stroke:red;stroke-width:1px;font-family:&quot;Noto Sans&quot;"/></svg>"#;
        let value = dispatch(json!({"source":source,"legacyDpi":true})).unwrap();
        close(area(&value), 20. * (25.4 / 72.) * (25.4 / 96.));
    }
    #[test]
    fn embedded_images_and_partial_geometry_fail_before_any_silent_omission() {
        for source in [
            "<svg><rect width='5' height='5'/><rect width='oops' height='4'/></svg>",
            "<svg><image width='5' height='5' href='data:image/png;base64,invalid'/></svg>",
            "<svg><path d='M0 0H5' style='stroke:red;vector-effect:non-rotation'/></svg>",
        ] {
            assert!(dispatch(json!({"source":source,"action":"preview"})).is_err());
        }
        let inner = base64::engine::general_purpose::STANDARD
            .encode("<svg xmlns='http://www.w3.org/2000/svg'><script/></svg>");
        let source = format!(
            "<svg><rect width='5' height='5'/><image href='data:image/svg+xml;base64,{inner}' width='5' height='5'/></svg>"
        );
        assert_eq!(
            dispatch(json!({"source":source,"action":"preview"}))
                .unwrap_err()
                .code,
            "E_IMPORT_UNSUPPORTED_FEATURE"
        );
    }
    #[test]
    fn reference_expansion_counts_path_weight_and_text_before_normalization() {
        let path = format!("M0 0{}Z", "L1 1 ".repeat(2000));
        let source = format!(
            "<svg><defs><path id='a' d='{path}'/></defs>{}</svg>",
            "<use href='#a'/>".repeat(300)
        );
        assert_eq!(
            dispatch(json!({"source":source,"action":"preview"}))
                .unwrap_err()
                .code,
            "E_IMPORT_LIMIT"
        );
        let source = format!("<svg><text>{}</text></svg>", "A".repeat(6000));
        assert_eq!(
            dispatch(json!({"source":source,"action":"preview"}))
                .unwrap_err()
                .code,
            "E_IMPORT_LIMIT"
        );
    }
    #[test]
    fn marker_replication_is_rejected_before_normalization() {
        let heavy = format!("M0 0{}Z", "L1 1 ".repeat(1000));
        let definitions = format!(
            "<defs><path id='heavy' d='{heavy}'/><marker id='m'><use href='#heavy'/></marker><path id='line' d='{heavy}'/></defs>"
        );
        for content in [
            "<path d='M0 0H10' marker-end='url(#m)'/>".repeat(300),
            "<style>.marked { marker-mid:url(#m) }</style><use class='marked' href='#line'/>"
                .into(),
            "<g style='marker: url(&quot;#m&quot;)'><use href='#line'/></g>".into(),
        ] {
            let source = format!("<svg>{definitions}{content}</svg>");
            // Calling preflight directly proves that no usvg expansion is needed
            // to reject the expensive body × placements combination.
            let error = preflight(&source).unwrap_err();
            assert_eq!(error.code, "E_IMPORT_LIMIT");
            assert!(error.message.contains("marker"));
        }
        let source = format!(
            "<svg><defs><marker id='text'><text>{}</text></marker></defs><path marker-mid='url(#text)' d='M0 0{}'/></svg>",
            "A".repeat(100),
            "L1 1 ".repeat(100)
        );
        assert_eq!(preflight(&source).unwrap_err().code, "E_IMPORT_LIMIT");
    }

    #[test]
    fn nested_markers_count_expansion_and_source_inheritance() {
        let body = format!("M0 0{}", "L1 1 ".repeat(100));
        let source = format!(
            "<svg><defs><marker id='a'><path d='{body}' marker-mid='url(#b)'/></marker><g style='marker-mid:url(#c)'><marker id='b'><path d='{body}'/></marker></g><marker id='c'><path d='{body}'/></marker></defs><path d='M0 0H10' marker-end='url(#a)'/></svg>"
        );
        assert_eq!(preflight(&source).unwrap_err().code, "E_IMPORT_LIMIT");
        let source = format!(
            "<svg><defs><marker id='m'><path d='{body}'/></marker></defs><path d='M0 0A1e24 1e24 0 1 0 1 1' marker-mid='url(#m)'/></svg>"
        );
        assert_eq!(preflight(&source).unwrap_err().code, "E_IMPORT_LIMIT");
    }

    #[test]
    fn custom_fonts_admit_outlines_and_reject_nested_images_in_every_collection_face() {
        let outline = include_bytes!("../../../assets/NotoSans-Regular.ttf").to_vec();
        fn collection(fonts: &[Vec<u8>]) -> Vec<u8> {
            let header = 12 + fonts.len() * 4;
            let mut out = vec![0; header];
            out[..4].copy_from_slice(b"ttcf");
            out[4..8].copy_from_slice(&0x00010000u32.to_be_bytes());
            out[8..12].copy_from_slice(&(fonts.len() as u32).to_be_bytes());
            for (index, font) in fonts.iter().enumerate() {
                let offset = out.len();
                out[12 + index * 4..16 + index * 4].copy_from_slice(&(offset as u32).to_be_bytes());
                let mut face = font.clone();
                let tables = u16::from_be_bytes([face[4], face[5]]) as usize;
                for table in 0..tables {
                    let field = 12 + table * 16 + 8;
                    let original = u32::from_be_bytes(face[field..field + 4].try_into().unwrap());
                    face[field..field + 4]
                        .copy_from_slice(&(original + offset as u32).to_be_bytes());
                }
                out.extend_from_slice(&face);
                while !out.len().is_multiple_of(4) {
                    out.push(0);
                }
            }
            out
        }
        for font in [
            outline.clone(),
            collection(&[outline.clone(), outline.clone()]),
        ] {
            admit_outline_font(&font).unwrap();
            let encoded = base64::engine::general_purpose::STANDARD.encode(font);
            assert!(dispatch(json!({"source":"<svg width='100' height='100'><text y='20'>Hi</text></svg>","action":"preview","fonts":[encoded]})).is_ok());
        }
        for tag in [*b"SVG ", *b"COLR", *b"CBDT", *b"sbix"] {
            let mut color = outline.clone();
            color[12..16].copy_from_slice(&tag);
            for font in [color.clone(), collection(&[outline.clone(), color])] {
                assert_eq!(
                    admit_outline_font(&font).unwrap_err().code,
                    "E_IMPORT_UNSUPPORTED_FEATURE"
                );
            }
        }
        let mut oversized = b"ttcf\0\x01\0\0".to_vec();
        oversized.extend_from_slice(&129u32.to_be_bytes());
        assert_eq!(
            admit_outline_font(&oversized).unwrap_err().code,
            "E_IMPORT_LIMIT"
        );
    }

    #[test]
    fn marker_admission_counts_tref_copies_links_and_primitive_arcs() {
        let target = format!("<g id='words'>{}</g>", "A".repeat(4000));
        let source = format!(
            "<svg><defs>{target}</defs><text>{}</text></svg>",
            "<tref href='#words'/>".repeat(1000)
        );
        assert_eq!(preflight(&source).unwrap_err().code, "E_IMPORT_LIMIT");
        let source = format!(
            "<svg><defs>{target}<marker id='m'><text><tref href='#words'/></text></marker></defs><path d='M0 0H10' marker-end='url(#m)'/></svg>"
        );
        assert_eq!(preflight(&source).unwrap_err().code, "E_IMPORT_LIMIT");
        let source = format!("<svg><text><a>{}</a></text></svg>", "A".repeat(6000));
        assert_eq!(preflight(&source).unwrap_err().code, "E_IMPORT_LIMIT");
        let source = "<svg width='100' height='100'><defs><g id='words'>Hi</g></defs><text y='20'><tref href='#words'/><a> SVG</a></text></svg>";
        assert!(dispatch(json!({"source":source,"action":"preview"})).is_ok());
        let source = "<svg><defs><marker id='m'><path d='M0 0L3 1L0 2Z'/></marker></defs><circle r='1e35' marker-mid='url(#m)'/></svg>";
        assert_eq!(preflight(source).unwrap_err().code, "E_IMPORT_LIMIT");
        let source = "<svg><defs><marker id='m' markerWidth='1e35'><circle r='100%'/></marker></defs><path d='M0 0H10' marker-end='url(#m)'/></svg>";
        assert_eq!(preflight(source).unwrap_err().code, "E_IMPORT_LIMIT");
    }

    #[test]
    fn ordinary_css_markers_and_recursive_marker_suppression_remain_valid() {
        let dense = format!("M0 0{}", "l1 1l1 -1".repeat(5000));
        for declaration in [
            "marker-end:url(#m)",
            "marker-start:url(#m)",
            "marker-start:url(#m);marker-end:url(#m)",
        ] {
            let source = format!(
                "<svg><defs><marker id='m'><path d='M0 0L3 1L0 2Z'/></marker></defs><path style='{declaration}' d='{dense}'/></svg>"
            );
            assert!(preflight(&source).is_ok());
        }
        let mut definitions = String::new();
        let mut css = String::new();
        let mut lines = String::new();
        for i in 0..12 {
            definitions.push_str(&format!(
                "<marker id='a{i}'><path d='M0 0L3 1L0 2Z'/></marker>"
            ));
            css.push_str(&format!(".line{i} {{ marker-end:url(#a{i}) }}"));
            lines.push_str(&format!(
                "<path class='line{i}' d='M0 {i}H20' fill='none' stroke='black'/>"
            ));
        }
        let source = format!(
            "<svg width='100' height='100'><style>{css}</style><defs>{definitions}</defs>{lines}</svg>"
        );
        assert!(preflight(&source).is_ok());
        assert!(dispatch(json!({"source":source,"action":"preview"})).is_ok());
        // The renderer suppresses an already-active marker, so a universal rule
        // that also matches its own triangle must not create an infinite budget.
        let source = "<svg width='100' height='100'><style>path {marker-end:url(#m)}</style><defs><marker id='m'><path d='M0 0L3 1L0 2Z'/></marker></defs><path d='M0 0H20' stroke='black'/></svg>";
        assert!(dispatch(json!({"source":source,"action":"preview"})).is_ok());
    }

    #[test]
    fn all_gap_dashes_are_valid_empty_geometry_and_preserve_other_paints() {
        let gap = r#"<path d="M0 0H1" fill="none" stroke="black" stroke-width="1" stroke-dasharray="1 10" stroke-dashoffset="2"/>"#;
        let source = format!("<svg width='10mm' height='10mm' viewBox='0 0 10 10'>{gap}</svg>");
        assert!(contours(&parse(&source)).is_empty());
        assert!(dispatch(json!({"source":source,"action":"preview"})).is_ok());
        let source = format!(
            "<svg width='10mm' height='10mm' viewBox='0 0 10 10'><rect x='2' y='2' width='3' height='3'/>{gap}</svg>"
        );
        close(area(&parse(&source)), 9.);
        let source = r#"<svg><path d="M0 0H1000000" fill="none" stroke="black" stroke-dasharray=".001 .001"/></svg>"#;
        assert_eq!(
            dispatch(json!({"source":source})).unwrap_err().code,
            "E_IMPORT_LIMIT"
        );
    }

    #[test]
    fn non_scaling_dash_budget_uses_host_space_before_preview_expansion() {
        // One tiny local segment becomes a very long centerline in host space.
        // Budgeting its local length would admit billions of generated dashes.
        let source = r#"<svg width="100" height="100"><path d="M0 0H1" transform="scale(100000)" fill="none" stroke="black" stroke-width="1" stroke-dasharray=".00001 .00001" vector-effect="non-scaling-stroke"/></svg>"#;
        for mode in ["preview", "parse"] {
            assert_eq!(
                dispatch(json!({"source":source,"action":mode}))
                    .unwrap_err()
                    .code,
                "E_IMPORT_LIMIT"
            );
        }
    }

    #[test]
    fn vector_effect_rejection_checks_every_css_declaration_and_rule() {
        for css in [
            "vector-effect:none;vector-effect:non-rotation",
            "vector-effect:non-rotation;vector-effect:none",
            "vector-effect: none !important; vector-effect: non-rotation",
            "vector-effect/**/:/**/non-rotation;vector-effect:none",
        ] {
            let source = format!("<svg><path d='M0 0H10' stroke='black' style='{css}'/></svg>");
            assert_eq!(
                dispatch(json!({"source":source,"action":"preview"}))
                    .unwrap_err()
                    .code,
                "E_IMPORT_UNSUPPORTED_FEATURE"
            );
        }
        let source = "<svg><style>.a{vector-effect:none}.b{vector-effect:non-rotation}</style><path class='b' d='M0 0H10' stroke='black'/></svg>";
        assert_eq!(
            dispatch(json!({"source":source})).unwrap_err().code,
            "E_IMPORT_UNSUPPORTED_FEATURE"
        );
        for css in [
            "vector-effect : none !important",
            "vector-effect/**/:/**/none",
            "font-family:'sample;vector-effect:non-scaling-stroke';vector-effect:none",
            "/*vector-effect:non-scaling-stroke*/vector-effect:none",
        ] {
            assert!(check_urls(css).is_ok(), "{css}");
        }
    }
    #[test]
    fn audit_css_root_size_and_close_continuation() {
        let source = r#"<svg width="10mm" height="10mm" viewBox="0 0 10 10" style="width:20mm;height:30mm" preserveAspectRatio="none"><rect width="10" height="10"/></svg>"#;
        let result = parse(source);
        close(result["widthMm"].as_f64().unwrap(), 20.);
        close(result["heightMm"].as_f64().unwrap(), 30.);
        close(area(&result), 600.);
        // Artwork normalization serializes usvg's f32 transforms; the CAD
        // contour exporter has its separate exact-dimension roundtrip checks.
        assert!((area(&parse(result["normalizedSvg"].as_str().unwrap())) - 600.).abs() < 0.001);
        let v = parse(
            r#"<svg width="10mm" height="10mm" viewBox="0 0 10 10"><path d="M0 0H2V2Z L0 4L4 4Z"/></svg>"#,
        );
        close(area(&v), 10.);
    }

    #[test]
    fn raster_decode_errors_are_explicit_before_preview_or_silhouette() {
        let pixmap = resvg::tiny_skia::Pixmap::new(2, 2).unwrap();
        let bytes = pixmap.encode_png().unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes[..33]);
        let source = format!(
            "<svg width='2' height='2'><rect width='1' height='1'/><image width='2' height='2' href='data:image/png;base64,{encoded}'/></svg>"
        );
        for options in [
            json!({"source":source,"action":"preview"}),
            json!({"source":source,"geometryMode":"silhouette"}),
        ] {
            let error = dispatch(options).unwrap_err();
            assert_eq!(error.code, "E_IMPORT_INVALID_DATA");
            assert!(error.message.contains("PNG"));
        }
    }
    #[test]
    fn animated_gif_first_frame_preserves_canvas_and_matches_static_preview() {
        let mut bytes = Vec::new();
        {
            let mut encoder = gif::Encoder::new(&mut bytes, 4, 4, &[]).unwrap();
            encoder.set_repeat(gif::Repeat::Infinite).unwrap();
            let mut red = [255, 0, 0, 255].repeat(4);
            let mut first = gif::Frame::from_rgba(2, 2, &mut red);
            first.left = 1;
            first.top = 1;
            first.delay = 10;
            encoder.write_frame(&first).unwrap();
            let mut blue = [0, 0, 255, 255].repeat(16);
            encoder
                .write_frame(&gif::Frame::from_rgba(4, 4, &mut blue))
                .unwrap();
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let source = format!(
            "<svg width='4mm' height='4mm' viewBox='0 0 4 4'><image width='4' height='4' style='image-rendering:pixelated' href='data:image/gif;base64,{encoded}'/></svg>"
        );
        let result = dispatch(json!({"source":source,"action":"preview"})).unwrap();
        let normalized = result["normalizedSvg"].as_str().unwrap();
        assert!(normalized.contains("data:image/png"));
        assert!(!normalized.contains("data:image/gif"));
        assert!(
            result["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w.as_str().unwrap().contains("first frame"))
        );
        let original =
            dispatch(json!({"source":source,"geometryMode":"silhouette","rasterSize":128}))
                .unwrap();
        let imported =
            dispatch(json!({"source":normalized,"geometryMode":"silhouette","rasterSize":128}))
                .unwrap();
        close(area(&original), 4.);
        assert_eq!(contours(&original), contours(&imported));
        for (a, b) in bounds(&original).into_iter().zip([1., 1., 3., 3.]) {
            close(a, b);
        }
    }
    #[test]
    fn webp_normalization_preserves_transparency_and_static_pixels() {
        let mut bytes = Vec::new();
        image_webp::WebPEncoder::new(&mut bytes)
            .encode(
                &[255, 0, 0, 255, 0, 0, 0, 0],
                2,
                1,
                image_webp::ColorType::Rgba8,
            )
            .unwrap();
        let kind = normalize_raster_image("image/webp", std::sync::Arc::new(bytes)).unwrap();
        let usvg::ImageKind::PNG(png) = kind else {
            panic!("static PNG expected")
        };
        let decoded = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 1);
        assert_eq!(decoded.pixels()[0].alpha(), 255);
        assert_eq!(decoded.pixels()[1].alpha(), 0);
    }
    #[test]
    fn css_geometry_properties_and_non_applicable_declarations_are_admitted() {
        for css in [
            "rect { x:10px }",
            "rect { rx:4px!important }",
            ".shape { d:path('M0 0H2') }",
            "svg { height:100% }",
        ] {
            let source = format!("<svg><style>{css}</style><rect width='5' height='5'/></svg>");
            assert!(
                dispatch(json!({"source":source,"action":"preview"})).is_ok(),
                "{css}"
            );
        }
        assert!(dispatch(json!({"source":r#"<svg><rect x="1" y="1" width="5" height="5" rx="2" style="stroke-width:1px;font-size:12px;fill:red"/></svg>"#,"action":"preview"})).is_ok());
    }
    #[test]
    fn duplicate_ids_and_dual_href_cannot_bypass_reference_admission() {
        let source = "<svg><defs><path id='a' d='M0 0H5V5Z'/><path id='a' d='M0 0H1V1Z'/></defs><use href='#a'/></svg>";
        assert_eq!(
            dispatch(json!({"source":source,"action":"preview"}))
                .unwrap_err()
                .code,
            "E_IMPORT_INVALID_DATA"
        );
        let path = format!("M0 0{}Z", "L1 1 ".repeat(2000));
        let source = format!(
            "<svg xmlns:xlink='http://www.w3.org/1999/xlink'><defs><path id='large' d='{path}'/><path id='small' d='M0 0H1V1Z'/></defs>{}</svg>",
            "<use xlink:href='#small' href='#large'/>".repeat(300)
        );
        assert_eq!(
            dispatch(json!({"source":source,"action":"preview"}))
                .unwrap_err()
                .code,
            "E_IMPORT_LIMIT"
        );
    }
