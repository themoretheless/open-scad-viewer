//! Operation dispatch: preserves the original arithmetic and proof budgets.
use super::*;

pub(crate) fn dispatch(v: Value) -> Result<Value> {
    let source = v["source"]
        .as_str()
        .ok_or_else(|| invalid("SVG source must be a string"))?;
    let action = v["action"].as_str().unwrap_or("parse");
    if !matches!(action, "parse" | "preview" | "profile") {
        return Err(invalid("Unknown SVG action"));
    }
    let dpi = number(&v, "dpi", 72., 0.001, 1_000_000.)?;
    let tolerance = number(&v, "tolerance", 0.02, 0.000001, 1000.)?;
    let document = preflight(source)?;
    let has_text = document
        .descendants()
        .any(|n| n.is_element() && n.tag_name().name() == "text");
    let legacy = v["legacyDpi"].as_bool().unwrap_or(false);
    let adjusted = if legacy {
        legacy_source(source, &document, dpi)
    } else {
        source.to_owned()
    };
    let tree = parse_tree(&adjusted, dpi, v.get("fonts"))?;
    let css = crate::svg_css::analyze(source)?;
    let mm = 25.4 / dpi;
    let width = physical_dimension_value(
        css.root_width
            .as_deref()
            .or_else(|| document.root_element().attribute("width")),
        tree.size().width() as f64 * mm,
        dpi,
        legacy,
    );
    let height = physical_dimension_value(
        css.root_height
            .as_deref()
            .or_else(|| document.root_element().attribute("height")),
        tree.size().height() as f64 * mm,
        dpi,
        legacy,
    );
    let mut warnings = Vec::<String>::new();
    if document
        .descendants()
        .flat_map(|n| n.attributes())
        .any(|a| {
            a.name() == "href"
                && (a.value().starts_with("data:image/gif")
                    || a.value().starts_with("data:image/webp"))
        })
    {
        warnings.push("Embedded GIF and WebP images are normalized to their first frame for a consistent static preview and silhouette.".to_owned());
    }
    if has_text {
        warnings.push("SVG text is converted to outlines using supplied fonts with bundled Noto Sans as fallback; unavailable fonts may change the layout.".to_owned());
    }
    let mode = v["geometryMode"].as_str().unwrap_or("vector");
    if !matches!(mode, "vector" | "silhouette") {
        return Err(invalid("SVG geometryMode must be vector or silhouette"));
    }
    let regions = if action == "preview" {
        vec![]
    } else if mode == "silhouette" {
        let size = number(&v, "rasterSize", 512., 128., 2048.)?;
        if size.fract() != 0. {
            return Err(invalid("SVG rasterSize must be an integer"));
        }
        let alpha = number(&v, "alphaThreshold", 0.5, 0.01, 1.)?;
        warnings.push(format!("Rendered silhouette uses a {size}-pixel maximum dimension and alpha threshold {alpha}; its contours approximate raster pixels."));
        let contours = crate::svg_silhouette::contours(&tree, width, height, size as u32, alpha)?;
        if contours.is_empty() {
            vec![]
        } else {
            vec![Region {
                contours,
                rule: FillRule::NonZero,
            }]
        }
    } else {
        let transform = coordinate_transform(&document, &tree, dpi, legacy, width, height)?;
        Geometry {
            tolerance,
            points: 0,
            nodes: 0,
        }
        .group(tree.root(), transform, false, 0)?
    };
    let normalized = tree.to_string(&usvg::WriteOptions {
        preserve_text: false,
        ..Default::default()
    });
    let normalized_doc =
        usvg::roxmltree::Document::parse(&normalized).map_err(|e| invalid(e.to_string()))?;
    let mut edits = Vec::new();
    for (name, value) in [("width", width), ("height", height)] {
        if let Some(attr) = normalized_doc
            .root_element()
            .attributes()
            .find(|a| a.name() == name)
        {
            edits.push((attr.range_value(), format!("{value}mm")));
        }
    }
    let root_start = normalized_doc.root_element().range().start + 4;
    edits.push((
        root_start..root_start,
        format!(
            " viewBox=\"0 0 {} {}\"",
            tree.size().width(),
            tree.size().height()
        ),
    ));
    edits.sort_by_key(|(r, _)| r.start);
    let mut normalized = normalized;
    for (range, text) in edits.into_iter().rev() {
        normalized.replace_range(range, &text);
    }
    if normalized.len() > MAX_SOURCE {
        return Err(limit("Normalized SVG exceeds 4 MiB"));
    }
    if action == "profile" {
        let mut sections = Vec::with_capacity(regions.len());
        for region in regions {
            let mut rings = region.contours;
            rings.retain(|r| r.len() >= 3);
            if rings.is_empty() {
                sections.push(Vec::new());
                continue;
            }
            if matches!(region.rule, FillRule::EvenOdd) {
                let copy = rings.clone();
                for (i, r) in rings.iter_mut().enumerate() {
                    let p = r[0];
                    let mut depth = 0;
                    for (j, other) in copy.iter().enumerate() {
                        if i == j {
                            continue;
                        }
                        let mut yes = false;
                        for k in 0..other.len() {
                            let a = other[k];
                            let b = other[(k + 1) % other.len()];
                            if (a[1] > p[1]) != (b[1] > p[1])
                                && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
                            {
                                yes = !yes;
                            }
                        }
                        if yes {
                            depth += 1;
                        }
                    }
                    if (rings::area(r) > 0.) != (depth % 2 == 0) {
                        r.reverse();
                    }
                }
                sections.push(rings::planar(&rings, &vec![], "union")?);
            } else {
                sections.push(rings::nonzero(&rings)?);
            }
        }
        let merged = match sections.split_first() {
            None => Vec::new(),
            Some((first, rest)) => {
                let mut acc = first.clone();
                for next in rest {
                    acc = rings::planar(&acc, next, "union")?;
                }
                acc
            }
        };
        let contours = mesh_section::bounded_svg_contours(merged)?;
        return Ok(json!({
            "contours": contours,
            "widthMm": width,
            "heightMm": height,
            "warnings": warnings,
        }));
    }
    Ok(
        json!({"regions":regions.into_iter().map(|r|json!({"contours":r.contours,"fillRule":match r.rule{FillRule::NonZero=>"nonzero",FillRule::EvenOdd=>"evenodd"}})).collect::<Vec<_>>(),"widthMm":width,"heightMm":height,"normalizedSvg":normalized,"warnings":warnings}),
    )
}
