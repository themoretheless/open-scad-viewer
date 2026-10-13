use super::*;

pub(super) fn preflight_inner(
    source: &str,
    embedded_depth: usize,
    validate_css: bool,
) -> Result<usvg::roxmltree::Document<'_>> {
    if embedded_depth > 8 {
        return Err(limit("Embedded SVG image nesting exceeds 8 levels"));
    }
    if source.len() > MAX_SOURCE {
        return Err(limit("SVG source exceeds 4 MiB"));
    }
    if source.to_ascii_lowercase().contains("<!doctype")
        || source.to_ascii_lowercase().contains("<!entity")
    {
        return Err(invalid("SVG document types and entities are forbidden"));
    }
    // Bound lexical element depth before the XML parser allocates/recurses.
    let mut cursor = 0;
    let mut depth = 0_usize;
    while let Some(offset) = source[cursor..].find('<') {
        let start = cursor + offset;
        let tail = &source[start..];
        if let Some((prefix, ending)) = [("<!--", "-->"), ("<![CDATA[", "]]>"), ("<?", "?>")]
            .into_iter()
            .find(|(p, _)| tail.starts_with(p))
        {
            let end = tail[prefix.len()..]
                .find(ending)
                .ok_or_else(|| invalid("Unterminated SVG XML section"))?;
            cursor = start + prefix.len() + end + ending.len();
            continue;
        }
        let mut quote = None;
        let mut end = None;
        for (i, c) in tail.char_indices().skip(1) {
            if quote == Some(c) {
                quote = None;
            } else if quote.is_none() && matches!(c, '\'' | '"') {
                quote = Some(c);
            } else if quote.is_none() && c == '>' {
                end = Some(i);
                break;
            }
        }
        let end = end.ok_or_else(|| invalid("Unterminated SVG XML tag"))?;
        if tail.starts_with("</") {
            depth = depth.saturating_sub(1);
        } else if !tail[..end].trim_end().ends_with('/') && !tail.starts_with("<!") {
            depth += 1;
            if depth > 64 {
                return Err(limit("SVG nesting exceeds 64 levels"));
            }
        }
        cursor = start + end + 1;
    }
    let doc = usvg::roxmltree::Document::parse(source)
        .map_err(|e| invalid(format!("Malformed SVG XML: {e}")))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg"
        || root
            .tag_name()
            .namespace()
            .is_some_and(|ns| ns != "http://www.w3.org/2000/svg")
    {
        return Err(invalid("SVG root element must be <svg>"));
    }
    let mut nodes = 0;
    let mut segments = 0;
    for node in doc.descendants().filter(|n| n.is_element()) {
        nodes += 1;
        if nodes > MAX_NODES {
            return Err(limit("SVG exceeds 50000 elements"));
        }
        if node.ancestors().filter(|n| n.is_element()).count() > 64 {
            return Err(limit("SVG nesting exceeds 64 levels"));
        }
        let tag = node.tag_name().name();
        if matches!(
            tag,
            "script"
                | "foreignObject"
                | "animate"
                | "animateMotion"
                | "animateTransform"
                | "set"
                | "discard"
        ) {
            return Err(unsupported(format!(
                "SVG <{tag}> is outside the static SVG import profile"
            )));
        }
        for attr in node.attributes() {
            let name = attr.name();
            if name.starts_with("on") {
                return Err(unsupported("SVG event handlers are forbidden"));
            }
            if name == "href" && tag != "a" {
                let href = attr.value().trim();
                if !href.starts_with('#')
                    && !(matches!(tag, "image" | "feImage") && href.starts_with("data:image/"))
                {
                    return Err(unsupported(
                        "SVG external references must be embedded or resolved to local fragments before import",
                    ));
                }
                if href.starts_with("data:") {
                    let url = data_url::DataUrl::process(href)
                        .map_err(|_| invalid("Malformed SVG image data URL"))?;
                    let (data, _) = url
                        .decode_to_vec()
                        .map_err(|_| invalid("Malformed SVG image data"))?;
                    if data.len() > MAX_SOURCE {
                        return Err(limit("Embedded SVG image exceeds 4 MiB"));
                    }
                    if url.mime_type().matches("image", "svg+xml") && validate_css {
                        preflight_inner(
                            std::str::from_utf8(&data)
                                .map_err(|_| invalid("Embedded SVG is not UTF-8"))?,
                            embedded_depth + 1,
                            true,
                        )?;
                    } else if !url.mime_type().matches("image", "svg+xml") {
                        if !["png", "jpg", "jpeg", "gif", "webp"]
                            .contains(&url.mime_type().subtype.as_str())
                        {
                            return Err(unsupported(
                                "SVG embedded image format must be PNG, JPEG, GIF, WebP or SVG",
                            ));
                        }
                        let size = imagesize::blob_size(&data)
                            .map_err(|_| invalid("SVG embedded image is invalid"))?;
                        if size.width.saturating_mul(size.height) > 16 * 1024 * 1024 {
                            return Err(limit(
                                "Embedded SVG image exceeds 16 million decoded pixels",
                            ));
                        }
                    }
                }
            }
            if name == "style"
                || matches!(
                    name,
                    "fill"
                        | "stroke"
                        | "filter"
                        | "mask"
                        | "clip-path"
                        | "marker"
                        | "marker-start"
                        | "marker-mid"
                        | "marker-end"
                )
            {
                check_urls(attr.value())?;
            }
            if name == "vector-effect"
                && !matches!(
                    attr.value().trim(),
                    "none"
                        | "non-scaling-stroke"
                        | "inherit"
                        | "initial"
                        | "unset"
                        | "revert"
                        | "revert-layer"
                )
            {
                return Err(unsupported(
                    "SVG vector-effect supports none and non-scaling-stroke",
                ));
            }
            if name == "d" && tag == "path" {
                for item in svgtypes::PathParser::from(attr.value()) {
                    item.map_err(|e| invalid(format!("Malformed SVG path: {e}")))?;
                    segments += 1;
                    if segments > MAX_POINTS {
                        return Err(limit("SVG path command budget exceeded"));
                    }
                }
            }
            if name == "transform" || name == "gradientTransform" || name == "patternTransform" {
                for item in svgtypes::TransformListParser::from(attr.value()) {
                    item.map_err(|e| invalid(format!("Malformed SVG transform: {e}")))?;
                }
            }
            if name == "points" && matches!(tag, "polygon" | "polyline") {
                let mut count = 0;
                for item in svgtypes::NumberListParser::from(attr.value()) {
                    let n = item.map_err(|e| invalid(format!("Malformed SVG points: {e}")))?;
                    if !n.is_finite() {
                        return Err(invalid("Non-finite SVG point"));
                    }
                    count += 1;
                }
                if count % 2 != 0 {
                    return Err(invalid("SVG points must contain coordinate pairs"));
                }
                segments += count / 2;
                if segments > MAX_POINTS {
                    return Err(limit("SVG path command budget exceeded"));
                }
            }
            if matches!(name, "width" | "height" | "r" | "rx" | "ry")
                && matches!(
                    tag,
                    "svg"
                        | "rect"
                        | "circle"
                        | "ellipse"
                        | "image"
                        | "use"
                        | "symbol"
                        | "pattern"
                        | "mask"
                        | "filter"
                )
            {
                let length = attr
                    .value()
                    .parse::<svgtypes::Length>()
                    .map_err(|_| invalid(format!("Invalid SVG {name} length")))?;
                if !length.number.is_finite() || length.number < 0. {
                    return Err(invalid(format!(
                        "SVG {name} must be nonnegative and finite"
                    )));
                }
            }
        }
        if tag == "style" {
            check_urls(node.text().unwrap_or(""))?;
            crate::svg_css::validate_stylesheet(node.text().unwrap_or(""))?;
        }
    }
    // Reject cyclic and exponential use trees before normalization expands them.
    let mut ids = std::collections::HashMap::new();
    for node in doc.descendants() {
        if let Some(id) = node.attribute("id")
            && ids.insert(id, node).is_some() {
                return Err(invalid(format!("Duplicate SVG id {id}")));
            }
    }
    // Match marker declarations with the same selector implementation as usvg.
    // Union all possible declarations and inherited values instead of resolving
    // the cascade: overrides can overestimate work, but cannot hide replication.
    let mut css = simplecss::StyleSheet::new();
    for node in doc.descendants().filter(|n| n.has_tag_name("style")) {
        if matches!(node.attribute("type"), None | Some("text/css")) {
            css.parse_more(node.text().unwrap_or(""));
        }
    }
    // Primitive radii also pass through kurbo. Bound relative lengths against
    // every possible source viewport/font value and the allowed reference depth.
    // Physical lengths use the API's maximum dpi; numeric radii stay unscaled.
    let mut absolute: f64 = 16.;
    let mut relative: f64 = 1.;
    let mut admit_length = |value: &str| {
        if let Ok(length) = value.parse::<svgtypes::Length>() {
            use svgtypes::LengthUnit::*;
            let n = length.number.abs();
            match length.unit {
                Em | Ex => relative = relative.max(n),
                Percent => relative = relative.max(n / 100.),
                None | Px => absolute = absolute.max(n),
                _ => absolute = absolute.max(n * 1_000_000.),
            }
        }
    };
    for node in doc.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes().filter(|a| {
            matches!(
                a.name(),
                "width"
                    | "height"
                    | "markerWidth"
                    | "markerHeight"
                    | "r"
                    | "rx"
                    | "ry"
                    | "font-size"
            )
        }) {
            admit_length(attr.value());
        }
        if let Some(viewbox) = node.attribute("viewBox") {
            for number in svgtypes::NumberListParser::from(viewbox).flatten() {
                admit_length(&number.to_string());
            }
        }
        if let Some(style) = node.attribute("style") {
            for declaration in simplecss::DeclarationTokenizer::from(style) {
                if declaration.name == "font-size" {
                    admit_length(declaration.value);
                }
                if declaration.name == "font"
                    && let Ok(font) = svgtypes::FontShorthand::from_str(declaration.value) {
                        admit_length(font.font_size);
                    }
            }
        }
    }
    for declaration in css.rules.iter().flat_map(|r| &r.declarations) {
        if declaration.name == "font-size" {
            admit_length(declaration.value);
        }
        if declaration.name == "font"
            && let Ok(font) = svgtypes::FontShorthand::from_str(declaration.value) {
                admit_length(font.font_size);
            }
    }
    let relative_bound = absolute * relative.powi(MAX_DEPTH as i32 + 1);
    let mut primitive_units = std::collections::HashMap::new();
    for node in doc
        .descendants()
        .filter(|n| matches!(n.tag_name().name(), "rect" | "circle" | "ellipse"))
    {
        let mut radius: f64 = 0.;
        for name in ["r", "rx", "ry"] {
            if let Some(length) = node
                .attribute(name)
                .and_then(|v| v.parse::<svgtypes::Length>().ok())
            {
                use svgtypes::LengthUnit::*;
                let n = length.number.abs();
                let value = match length.unit {
                    None | Px => n,
                    Percent => n / 100. * relative_bound,
                    Em | Ex => n * relative_bound,
                    _ => n * 1_000_000.,
                };
                radius = radius.max(value);
            }
        }
        if radius > 0. {
            primitive_units.insert(node.id(), arc_units(radius).saturating_mul(4));
        }
    }
    let marker_property = |name: &str| {
        matches!(
            name,
            "marker" | "marker-start" | "marker-mid" | "marker-end"
        )
    };
    for rule in &mut css.rules {
        rule.declarations.retain(|d| marker_property(d.name));
    }
    css.rules.retain(|r| !r.declarations.is_empty());
    let mut local_markers = std::collections::HashMap::new();
    let mut selector_checks = 0usize;
    let mut reference_count = 0usize;
    for node in doc.descendants().filter(|n| n.is_element()) {
        let mut refs = std::collections::HashMap::new();
        let mut add = |property: &str, value: &str| {
            if let Ok(iri) = svgtypes::FuncIRI::from_str(value)
                && let Some(target) = ids.get(iri.0).filter(|n| n.has_tag_name("marker")) {
                    let positions = match property {
                        "marker-start" => 1,
                        "marker-mid" => 2,
                        "marker-end" => 4,
                        _ => 7,
                    };
                    refs.entry(target.id())
                        .or_insert(MarkerLink {
                            node: *target,
                            positions: 0,
                        })
                        .positions |= positions;
                }
        };
        for attr in node.attributes().filter(|a| marker_property(a.name())) {
            add(attr.name(), attr.value());
        }
        for rule in &css.rules {
            selector_checks += 1;
            if selector_checks > 2_000_000 {
                return Err(limit(
                    "SVG marker CSS admission exceeds 2000000 selector checks",
                ));
            }
            if rule.selector.matches(&CssNode(node)) {
                for declaration in &rule.declarations {
                    add(declaration.name, declaration.value);
                }
            }
        }
        if let Some(style) = node.attribute("style") {
            for declaration in simplecss::DeclarationTokenizer::from(style) {
                if marker_property(declaration.name) {
                    add(declaration.name, declaration.value);
                }
            }
        }
        reference_count += refs.len();
        if reference_count > MAX_POINTS {
            return Err(limit("SVG marker reference budget exceeded"));
        }
        if !refs.is_empty() {
            local_markers.insert(node.id(), refs.into_values().collect::<Vec<_>>());
        }
    }
    #[expect(clippy::too_many_arguments, reason = "SVG traversal carries explicit inherited rendering state")]
    fn cost<'a, 'input>(
        node: usvg::roxmltree::Node<'a, 'input>,
        ids: &std::collections::HashMap<&str, usvg::roxmltree::Node<'a, 'input>>,
        local_markers: &std::collections::HashMap<
            usvg::roxmltree::NodeId,
            Vec<MarkerLink<'a, 'input>>,
        >,
        primitive_units: &std::collections::HashMap<usvg::roxmltree::NodeId, usize>,
        inherited: &[MarkerLink<'a, 'input>],
        stack: &mut Vec<usvg::roxmltree::NodeId>,
        marker_stack: &mut Vec<usvg::roxmltree::NodeId>,
        copies: usize,
        budget: &mut usize,
    ) -> Result<()> {
        if stack.contains(&node.id()) {
            return Err(invalid("Cyclic SVG use reference"));
        }
        if stack.len() + marker_stack.len() > MAX_DEPTH {
            return Err(limit("SVG reference depth exceeds 128 levels"));
        }
        let commands = node.attribute("d").map_or(0, path_units);
        let point_units = node.attribute("points").map_or(0, |p| p.len() / 2);
        let primitive = primitive_units.get(&node.id()).copied().unwrap_or(0);
        let mut units = 1 + commands + point_units + primitive;
        if matches!(node.tag_name().name(), "text" | "tspan" | "textPath" | "a") {
            units += node
                .children()
                .filter(|n| n.is_text())
                .filter_map(|n| n.text())
                .map(|t| t.chars().count() * 100)
                .sum::<usize>();
        }
        if node.has_tag_name("tref") {
            // usvg copies all descendant character data from the source target,
            // including text outside <text>. Count both href spellings as for use.
            for href in node
                .attributes()
                .filter(|a| a.name() == "href")
                .map(|a| a.value())
            {
                if let Some(target) = svgtypes::IRI::from_str(href)
                    .ok()
                    .and_then(|iri| ids.get(iri.0))
                {
                    units = units.saturating_add(
                        target
                            .descendants()
                            .filter(|n| n.is_text())
                            .filter_map(|n| n.text())
                            .map(|text| text.chars().count().saturating_mul(100))
                            .sum::<usize>(),
                    );
                } else {
                    return Err(invalid(format!("Unresolved SVG tref reference {href}")));
                }
            }
        }
        *budget = budget.saturating_add(units.saturating_mul(copies));
        if *budget > MAX_POINTS {
            return Err(limit(
                "Expanded SVG use/marker content exceeds 500000 geometry units",
            ));
        }
        let mut markers = inherited.to_vec();
        if let Some(local) = local_markers.get(&node.id()) {
            merge_markers(&mut markers, local);
        }
        // Only mid markers multiply with vertex count. Start/end each render
        // once in usvg, including on dense plots and compound paths.
        // Arc subdivision is included in path_units before usvg allocates it.
        let vertices = match node.tag_name().name() {
            "path" => commands + 1,
            "polygon" | "polyline" => point_units + 1,
            "rect" | "circle" | "ellipse" => primitive + 8,
            "line" => 2,
            _ => 0,
        };
        if vertices > 0 {
            for marker in &markers {
                // usvg suppresses a marker already active in its expansion
                // chain. Different markers can still multiply one another.
                if marker_stack.contains(&marker.node.id()) {
                    continue;
                }
                let mut source_inherited = Vec::new();
                for ancestor in marker.node.ancestors().skip(1) {
                    if let Some(refs) = local_markers.get(&ancestor.id()) {
                        merge_markers(&mut source_inherited, refs);
                    }
                }
                marker_stack.push(marker.node.id());
                let placements = usize::from(marker.positions & 1 != 0)
                    + usize::from(marker.positions & 4 != 0)
                    + if marker.positions & 2 != 0 {
                        vertices
                    } else {
                        0
                    };
                cost(
                    marker.node,
                    ids,
                    local_markers,
                    primitive_units,
                    &source_inherited,
                    &mut Vec::new(),
                    marker_stack,
                    copies.saturating_mul(placements),
                    budget,
                )?;
                marker_stack.pop();
            }
        }
        stack.push(node.id());
        if node.tag_name().name() == "use" {
            for href in node
                .attributes()
                .filter(|a| a.name() == "href")
                .map(|a| a.value())
            {
                if let Some(target) = href.strip_prefix('#').and_then(|id| ids.get(id)) {
                    cost(
                        *target,
                        ids,
                        local_markers,
                        primitive_units,
                        &markers,
                        stack,
                        marker_stack,
                        copies,
                        budget,
                    )?;
                } else {
                    return Err(invalid(format!("Unresolved SVG use reference {href}")));
                }
            }
        }
        for child in node.children().filter(|n| n.is_element()) {
            cost(
                child,
                ids,
                local_markers,
                primitive_units,
                &markers,
                stack,
                marker_stack,
                copies,
                budget,
            )?;
        }
        stack.pop();
        Ok(())
    }
    cost(
        root,
        &ids,
        &local_markers,
        &primitive_units,
        &[],
        &mut Vec::new(),
        &mut Vec::new(),
        1,
        &mut 0,
    )?;
    if validate_css {
        let css = crate::svg_css::analyze(source)?;
        if css.admission_source != source {
            // Source selectors remain authoritative for rendering. This copy
            // conservatively budgets the resolved geometry and marker links.
            preflight_inner(&css.admission_source, embedded_depth, false)?;
        }
    }
    Ok(doc)
}
