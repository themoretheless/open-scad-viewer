//! Static SVG normalization and CAD vector outlines. usvg owns SVG semantics;
//! this adapter expands strokes, flattens curves in millimetres, and applies
//! clip paths with the planar kernel. Effects with no exact vector footprint
//! require the explicitly selected raster silhouette mode.
use crate::{Error, Result};
use base64::Engine;
use planar_geometry::{
    rings::{self, Rings},
    tessellation::FillRule,
};
use usvg::{
    Node, Paint,
    tiny_skia_path::{Path, PathSegment, Point, Transform},
};
use value_codec::{Value, json};

const MAX_SOURCE: usize = 4 * 1024 * 1024;
const MAX_NODES: usize = 50_000;
const MAX_POINTS: usize = 500_000;
const MAX_DEPTH: usize = 128;

#[cfg(test)]
#[path = "svg/non_scaling_tests.rs"]
mod non_scaling_tests;

fn invalid(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_INVALID_DATA", message)
}
fn limit(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_LIMIT", message)
}
fn unsupported(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_UNSUPPORTED_FEATURE", message)
}
fn effect(name: &str) -> Error {
    unsupported(format!(
        "SVG {name} requires rendered silhouette mode. Select silhouette to convert the rendered alpha channel to contours, or convert this effect to vector paths."
    ))
}
fn number(v: &Value, key: &str, default: f64, min: f64, max: f64) -> Result<f64> {
    let Some(value) = v.get(key) else {
        return Ok(default);
    };
    let x = value
        .as_f64()
        .ok_or_else(|| invalid(format!("SVG {key} must be a number")))?;
    if !x.is_finite() || x < min || x > max {
        return Err(invalid(format!(
            "SVG {key} must be between {min} and {max}"
        )));
    }
    Ok(x)
}

fn check_urls(value: &str) -> Result<()> {
    let lower = value.to_ascii_lowercase();
    // Inspect every declaration independently. A `none` declaration elsewhere
    // in a stylesheet must not hide an unsupported value on another element.
    // Delimiters inside quoted strings/comments are not CSS declarations.
    let check_declaration = |declaration: &str| -> Result<()> {
        if let Some((name, value)) = declaration.split_once(':') {
            if name.trim() == "vector-effect"
                && !matches!(
                    value.trim().trim_end_matches("!important").trim(),
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
        }
        Ok(())
    };
    let mut declaration = String::new();
    let mut chars = lower.chars().peekable();
    let mut quote = None;
    while let Some(c) = chars.next() {
        if let Some(delimiter) = quote {
            declaration.push(c);
            if c == delimiter {
                quote = None;
            }
        } else if matches!(c, '\'' | '"') {
            quote = Some(c);
            declaration.push(c);
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(next) = chars.next() {
                if next == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
            declaration.push(' ');
        } else if matches!(c, ';' | '{' | '}') {
            check_declaration(&declaration)?;
            declaration.clear();
        } else {
            declaration.push(c);
        }
    }
    check_declaration(&declaration)?;
    if lower.contains("@import") || lower.contains("@font-face") || lower.contains('\\') {
        return Err(unsupported(
            "SVG external stylesheets and CSS font loading are unsupported; supply font files explicitly",
        ));
    }
    let mut tail = lower.as_str();
    while let Some(start) = tail.find("url(") {
        tail = &tail[start + 4..];
        let end = tail
            .find(')')
            .ok_or_else(|| invalid("Unterminated SVG CSS url()"))?;
        let reference = tail[..end].trim().trim_matches(['\'', '"']).trim();
        if !reference.starts_with('#') {
            return Err(unsupported(
                "SVG external resources must be embedded before import",
            ));
        }
        tail = &tail[end + 1..];
    }
    Ok(())
}

/// Validate before usvg's deliberately forgiving parser can discard malformed
/// geometry. Neither file access nor network access is available to the parser.
// kurbo/usvg subdivides arcs at 0.1 user-unit error; a very large or
// eccentric arc can create far more than four cubics before our contour limit.
fn arc_units(radius: f64) -> usize {
    let count = (11.163 * radius.abs()).powf(1. / 6.).max(4.).ceil() + 1.;
    if !count.is_finite() || count > MAX_POINTS as f64 {
        MAX_POINTS + 1
    } else {
        count as usize
    }
}
fn path_units(data: &str) -> usize {
    use svgtypes::PathSegment::*;
    let mut current = [0., 0.];
    let mut start = current;
    let mut units = 0usize;
    for segment in svgtypes::PathParser::from(data).flatten() {
        let mut count = 2; // Includes an implicit move after close.
        let next = match segment {
            MoveTo { abs, x, y }
            | LineTo { abs, x, y }
            | CurveTo { abs, x, y, .. }
            | SmoothCurveTo { abs, x, y, .. }
            | Quadratic { abs, x, y, .. }
            | SmoothQuadratic { abs, x, y } => {
                if abs {
                    [x, y]
                } else {
                    [current[0] + x, current[1] + y]
                }
            }
            HorizontalLineTo { abs, x } => [if abs { x } else { current[0] + x }, current[1]],
            VerticalLineTo { abs, y } => [current[0], if abs { y } else { current[1] + y }],
            EllipticalArc {
                abs, rx, ry, x, y, ..
            } => {
                let next = if abs {
                    [x, y]
                } else {
                    [current[0] + x, current[1] + y]
                };
                if rx != 0. && ry != 0. && next != current {
                    // Rotation cannot increase this bound on SVG radius correction.
                    let half_chord = (next[0] - current[0]).hypot(next[1] - current[1]) * 0.5;
                    let radius =
                        rx.abs().max(ry.abs()) * (half_chord / rx.abs().min(ry.abs())).max(1.);
                    count = arc_units(radius) + 1;
                }
                next
            }
            ClosePath { .. } => start,
        };
        if matches!(segment, MoveTo { .. }) {
            start = next;
        }
        current = next;
        units = units.saturating_add(count);
        if units > MAX_POINTS {
            break;
        }
    }
    units
}

#[derive(Clone, Copy)]
struct MarkerLink<'a, 'input> {
    node: usvg::roxmltree::Node<'a, 'input>,
    positions: u8, // start = 1, mid = 2, end = 4
}
fn merge_markers<'a, 'input>(
    into: &mut Vec<MarkerLink<'a, 'input>>,
    from: &[MarkerLink<'a, 'input>],
) {
    for marker in from {
        if let Some(existing) = into.iter_mut().find(|m| m.node == marker.node) {
            existing.positions |= marker.positions;
        } else {
            into.push(*marker);
        }
    }
}

// Keep selector matching aligned with usvg's source XML matching, including
// selectors on referenced <use> targets. Dynamic pseudo-classes do not apply.
struct CssNode<'a, 'input>(usvg::roxmltree::Node<'a, 'input>);
impl simplecss::Element for CssNode<'_, '_> {
    fn parent_element(&self) -> Option<Self> {
        self.0.parent_element().map(Self)
    }
    fn prev_sibling_element(&self) -> Option<Self> {
        self.0.prev_sibling_element().map(Self)
    }
    fn has_local_name(&self, name: &str) -> bool {
        self.0.tag_name().name() == name
    }
    fn attribute_matches(&self, name: &str, op: simplecss::AttributeOperator<'_>) -> bool {
        self.0
            .attribute(name)
            .is_some_and(|value| op.matches(value))
    }
    fn pseudo_class_matches(&self, class: simplecss::PseudoClass<'_>) -> bool {
        matches!(class, simplecss::PseudoClass::FirstChild)
            && self.0.prev_sibling_element().is_none()
    }
}

fn preflight(source: &str) -> Result<usvg::roxmltree::Document<'_>> {
    preflight_inner(source, 0, true)
}
fn preflight_inner(
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
        if let Some(id) = node.attribute("id") {
            if ids.insert(id, node).is_some() {
                return Err(invalid(format!("Duplicate SVG id {id}")));
            }
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
                if declaration.name == "font" {
                    if let Ok(font) = svgtypes::FontShorthand::from_str(declaration.value) {
                        admit_length(font.font_size);
                    }
                }
            }
        }
    }
    for declaration in css.rules.iter().flat_map(|r| &r.declarations) {
        if declaration.name == "font-size" {
            admit_length(declaration.value);
        }
        if declaration.name == "font" {
            if let Ok(font) = svgtypes::FontShorthand::from_str(declaration.value) {
                admit_length(font.font_size);
            }
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
            if let Ok(iri) = svgtypes::FuncIRI::from_str(value) {
                if let Some(target) = ids.get(iri.0).filter(|n| n.has_tag_name("marker")) {
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

/// OpenSCAD distinguishes explicit CSS px (96/in) from unitless coordinates
/// (the import dpi). usvg correctly uses one user-unit scale, so translate the
/// explicit px lengths to physical points only for the compatibility entrypoint.
fn legacy_source(source: &str, doc: &usvg::roxmltree::Document<'_>, dpi: f64) -> String {
    fn px_to_pt(value: &str, css: bool) -> String {
        let bytes = value.as_bytes();
        let mut output = String::new();
        let mut i = 0;
        while i < bytes.len() {
            if css && matches!(bytes[i], b'\'' | b'"') {
                let start = i;
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                output.push_str(&value[start..i]);
                continue;
            }
            if bytes[i].is_ascii_alphabetic() || matches!(bytes[i], b'_' | b'#') {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'-'))
                {
                    i += 1;
                }
                output.push_str(&value[start..i]);
                continue;
            }
            if bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'+' || bytes[i] == b'-'
            {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_digit()
                        || matches!(bytes[i], b'.' | b'e' | b'E' | b'+' | b'-'))
                {
                    i += 1;
                }
                if value[i..].starts_with("px") {
                    if let Ok(n) = value[start..i].parse::<f64>() {
                        output.push_str(&format!("{}pt", n * 0.75));
                        i += 2;
                        continue;
                    }
                }
                output.push_str(&value[start..i]);
            } else {
                let c = value[i..].chars().next().unwrap();
                output.push(c);
                i += c.len_utf8();
            }
        }
        output
    }
    let mut edits = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes() {
            if matches!(
                attr.name(),
                "x" | "y"
                    | "x1"
                    | "x2"
                    | "y1"
                    | "y2"
                    | "cx"
                    | "cy"
                    | "r"
                    | "rx"
                    | "ry"
                    | "width"
                    | "height"
                    | "dx"
                    | "dy"
                    | "stroke-width"
                    | "stroke-dasharray"
                    | "stroke-dashoffset"
                    | "font-size"
                    | "letter-spacing"
                    | "word-spacing"
                    | "style"
            ) && attr.value().contains("px")
            {
                let converted = px_to_pt(attr.value(), attr.name() == "style")
                    .replace('&', "&amp;")
                    .replace('"', "&quot;")
                    .replace('\'', "&apos;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                edits.push((attr.range_value(), converted));
            }
        }
        if node.tag_name().name() == "style" {
            for child in node.children().filter(|n| n.is_text()) {
                if let Some(text) = child.text() {
                    edits.push((child.range(), px_to_pt(text, true)));
                }
            }
        }
    }
    let root = doc.root_element();
    let fallback: Vec<_> = root
        .attribute("viewBox")
        .map(|s| {
            svgtypes::NumberListParser::from(s)
                .filter_map(|n| n.ok())
                .collect()
        })
        .unwrap_or_default();
    let mut attrs = String::new();
    for (name, default, index) in [("width", 300., 2), ("height", 150., 3)] {
        if root.attribute(name).is_none() {
            attrs.push_str(&format!(
                " {name}=\"{}\"",
                fallback.get(index).copied().unwrap_or(default) * dpi / 96.
            ));
        }
    }
    if !attrs.is_empty() {
        let start = root.range().start;
        let end = source[start..]
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .map(|n| start + n)
            .unwrap_or(start + 4);
        edits.push((end..end, attrs));
    }
    edits.sort_by_key(|(r, _)| r.start);
    let mut out = source.to_owned();
    for (range, text) in edits.into_iter().rev() {
        out.replace_range(range, &text);
    }
    out
}

/// Validate complete raster data before resvg's renderer can silently omit a
/// failed decode. Animated containers become a portable first-frame PNG so
/// browser preview and CAD silhouette use the same static pixels.
fn normalize_raster_image(mime: &str, data: std::sync::Arc<Vec<u8>>) -> Result<usvg::ImageKind> {
    use resvg::tiny_skia::{ColorU8, Pixmap};
    use std::io::Cursor;
    let dimensions =
        imagesize::blob_size(&data).map_err(|_| invalid("SVG embedded image is invalid"))?;
    if dimensions.width.saturating_mul(dimensions.height) > 16 * 1024 * 1024 {
        return Err(limit(
            "Embedded SVG image exceeds 16 million decoded pixels",
        ));
    }
    let rgba_pixmap = |rgba: &[u8], width: u32, height: u32, alpha: bool| -> Result<Pixmap> {
        let stride = if alpha { 4 } else { 3 };
        if rgba.len()
            != (width as usize)
                .saturating_mul(height as usize)
                .saturating_mul(stride)
        {
            return Err(invalid(
                "SVG embedded image has inconsistent decoded dimensions",
            ));
        }
        let mut image =
            Pixmap::new(width, height).ok_or_else(|| limit("SVG image allocation failed"))?;
        for (pixel, source) in image.pixels_mut().iter_mut().zip(rgba.chunks_exact(stride)) {
            *pixel = ColorU8::from_rgba(
                source[0],
                source[1],
                source[2],
                if alpha { source[3] } else { 255 },
            )
            .premultiply();
        }
        Ok(image)
    };
    let image = match mime {
        "image/png" => {
            Pixmap::decode_png(&data).map_err(|e| invalid(format!("Invalid embedded PNG: {e}")))?;
            return Ok(usvg::ImageKind::PNG(data));
        }
        "image/jpg" | "image/jpeg" => {
            let mut decoder = zune_jpeg::JpegDecoder::new(Cursor::new(data.as_slice()));
            decoder
                .decode_headers()
                .map_err(|e| invalid(format!("Invalid embedded JPEG: {e}")))?;
            let info = decoder
                .info()
                .ok_or_else(|| invalid("Embedded JPEG has no dimensions"))?;
            if usize::from(info.width).saturating_mul(usize::from(info.height)) > 16 * 1024 * 1024 {
                return Err(limit(
                    "Embedded SVG image exceeds 16 million decoded pixels",
                ));
            }
            decoder
                .decode()
                .map_err(|e| invalid(format!("Invalid embedded JPEG: {e}")))?;
            return Ok(usvg::ImageKind::JPEG(data));
        }
        "image/gif" => {
            let mut options = gif::DecodeOptions::new();
            options.set_color_output(gif::ColorOutput::RGBA);
            options.set_memory_limit(gif::MemoryLimit::Bytes(
                (64 * 1024 * 1024).try_into().unwrap(),
            ));
            let mut decoder = options
                .read_info(data.as_slice())
                .map_err(|e| invalid(format!("Invalid embedded GIF: {e}")))?;
            let (width, height) = (u32::from(decoder.width()), u32::from(decoder.height()));
            let frame = decoder
                .next_frame_info()
                .map_err(|e| invalid(format!("Invalid embedded GIF: {e}")))?
                .ok_or_else(|| invalid("Embedded GIF has no image frame"))?
                .clone();
            let frame_pixels = usize::from(frame.width).saturating_mul(usize::from(frame.height));
            if frame_pixels > 16 * 1024 * 1024 {
                return Err(limit(
                    "Embedded SVG image frame exceeds 16 million decoded pixels",
                ));
            }
            let mut pixels = vec![0; frame_pixels * 4];
            decoder
                .read_into_buffer(&mut pixels)
                .map_err(|e| invalid(format!("Invalid embedded GIF: {e}")))?;
            let first = rgba_pixmap(
                &pixels,
                u32::from(frame.width),
                u32::from(frame.height),
                true,
            )?;
            let mut canvas = Pixmap::new(width, height)
                .ok_or_else(|| limit("SVG GIF canvas allocation failed"))?;
            canvas.draw_pixmap(
                i32::from(frame.left),
                i32::from(frame.top),
                first.as_ref(),
                &Default::default(),
                Transform::identity(),
                None,
            );
            canvas
        }
        "image/webp" => {
            let mut decoder = image_webp::WebPDecoder::new(Cursor::new(data.as_slice()))
                .map_err(|e| invalid(format!("Invalid embedded WebP: {e}")))?;
            let (width, height) = decoder.dimensions();
            if (width as usize).saturating_mul(height as usize) > 16 * 1024 * 1024 {
                return Err(limit(
                    "Embedded SVG image exceeds 16 million decoded pixels",
                ));
            }
            let size = decoder
                .output_buffer_size()
                .ok_or_else(|| invalid("Embedded WebP has invalid dimensions"))?;
            let mut pixels = vec![0; size];
            decoder
                .read_image(&mut pixels)
                .map_err(|e| invalid(format!("Invalid embedded WebP: {e}")))?;
            rgba_pixmap(&pixels, width, height, decoder.has_alpha())?
        }
        _ => {
            return Err(unsupported(
                "SVG embedded image format must be PNG, JPEG, GIF, WebP or SVG",
            ));
        }
    };
    let encoded = image
        .encode_png()
        .map_err(|e| invalid(format!("Cannot normalize embedded SVG image: {e}")))?;
    if encoded.len() > MAX_SOURCE {
        return Err(limit("Normalized embedded SVG image exceeds 4 MiB"));
    }
    Ok(usvg::ImageKind::PNG(std::sync::Arc::new(encoded)))
}

fn admit_outline_font(bytes: &[u8]) -> Result<()> {
    let faces = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
    if faces == 0 || faces > 128 {
        return Err(limit(
            "SVG custom font collection must contain 1 to 128 faces",
        ));
    }
    for index in 0..faces {
        let face = ttf_parser::RawFace::parse(bytes, index)
            .map_err(|_| invalid("SVG custom font contains an invalid TrueType/OpenType face"))?;
        for record in face.table_records {
            let tag = record.tag.to_bytes();
            if [
                *b"SVG ", *b"COLR", *b"CBDT", *b"CBLC", *b"sbix", *b"EBDT", *b"EBLC", *b"bdat",
                *b"bloc",
            ]
            .contains(&tag)
            {
                // usvg parses SVG/COLR glyphs with private default options and
                // bitmap glyphs outside our image resolver. Admit outline fonts
                // only until those nested resources can share our admission.
                return Err(unsupported(format!(
                    "SVG custom font face {index} contains {} color/bitmap glyphs; supply an outline-only TTF/OTF/TTC font or convert text to paths before import",
                    String::from_utf8_lossy(&tag).trim(),
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn parse_tree(source: &str, dpi: f64, fonts: Option<&Value>) -> Result<usvg::Tree> {
    let document = preflight(source)?;
    let source_with_namespace;
    let source = if document.root_element().tag_name().namespace().is_none() {
        let at = document.root_element().range().start + 4;
        source_with_namespace = format!(
            "{} xmlns=\"http://www.w3.org/2000/svg\"{}",
            &source[..at],
            &source[at..]
        );
        source_with_namespace.as_str()
    } else {
        source
    };
    let mut options = usvg::Options {
        dpi: dpi as f32,
        font_family: "Noto Sans".to_owned(),
        default_size: usvg::Size::from_wh(300., 150.).unwrap(),
        ..Default::default()
    };
    options
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/NotoSans-Regular.ttf").to_vec());
    options.fontdb_mut().set_sans_serif_family("Noto Sans");
    options.fontdb_mut().set_serif_family("Noto Sans");
    options.fontdb_mut().set_monospace_family("Noto Sans");
    if let Some(fonts) = fonts {
        let list = fonts
            .as_array()
            .ok_or_else(|| invalid("SVG fonts must be an array of base64 strings"))?;
        if list.len() > 16 {
            return Err(limit("SVG accepts at most 16 custom fonts"));
        }
        let mut total = 0;
        for font in list {
            let encoded = font
                .as_str()
                .ok_or_else(|| invalid("SVG font must be a base64 string"))?;
            if encoded.len() > 5_592_408 {
                return Err(limit("SVG custom font exceeds 4 MiB"));
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|_| invalid("SVG font is not valid base64"))?;
            total += bytes.len();
            if bytes.len() > 4 * 1024 * 1024 || total > 8 * 1024 * 1024 {
                return Err(limit("SVG custom font budget exceeded"));
            }
            admit_outline_font(&bytes)?;
            let before = options.fontdb.faces().count();
            options.fontdb_mut().load_font_data(bytes);
            if options.fontdb.faces().count() == before {
                return Err(invalid(
                    "SVG custom font is not a supported OpenType/TrueType font",
                ));
            }
        }
    }
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    let default_data = usvg::ImageHrefResolver::default_data_resolver();
    let resolver_error = std::sync::Arc::new(std::sync::Mutex::new(None));
    let pending_error = resolver_error.clone();
    let decoded_pixel_budget = std::sync::Mutex::new(0usize);
    options.image_href_resolver.resolve_data = Box::new(move |mime, data, opts| {
        let result = (|| -> Result<_> {
            if data.len() > MAX_SOURCE {
                return Err(limit("Embedded SVG image exceeds 4 MiB"));
            }
            if mime == "image/svg+xml" {
                let source = std::str::from_utf8(&data)
                    .map_err(|_| invalid("Embedded SVG image is not UTF-8"))?;
                preflight(source)?;
            }
            if mime == "image/svg+xml" {
                default_data(mime, data, opts)
                    .ok_or_else(|| invalid("Embedded SVG image cannot be decoded"))
            } else {
                let size = imagesize::blob_size(&data)
                    .map_err(|_| invalid("SVG embedded image is invalid"))?;
                let mut pixels = decoded_pixel_budget.lock().unwrap();
                *pixels = pixels.saturating_add(size.width.saturating_mul(size.height));
                if *pixels > 64 * 1024 * 1024 {
                    return Err(limit("SVG image decoding exceeds 64 million total pixels"));
                }
                drop(pixels);
                normalize_raster_image(mime, data)
            }
        })();
        match result {
            Ok(image) => Some(image),
            Err(e) => {
                *pending_error.lock().unwrap() = Some(e);
                None
            }
        }
    });
    let mut tree =
        usvg::Tree::from_str(source, &options).map_err(|e| invalid(format!("Invalid SVG: {e}")))?;
    if let Some(error) = resolver_error.lock().unwrap().take() {
        return Err(error);
    }
    tree.resolve_non_scaling_contexts(MAX_POINTS)
        .map_err(|error| match error {
            usvg::NonScalingContextError::LimitExceeded => {
                limit("SVG non-scaling resource expansion exceeds 500000 work units")
            }
            usvg::NonScalingContextError::InvalidTransform => {
                invalid("SVG non-scaling resource has an invalid transform")
            }
        })?;
    fn check_images(
        group: &usvg::Group,
        depth: usize,
        nodes: &mut usize,
        pixels: &mut f64,
        stroke_points: &mut usize,
    ) -> Result<()> {
        if depth > MAX_DEPTH {
            return Err(limit("SVG image tree nesting exceeds 128 levels"));
        }
        for node in group.children() {
            *nodes += 1;
            if *nodes > 100_000 {
                return Err(limit("SVG image tree exceeds 100000 nodes"));
            }
            match node {
                Node::Group(g) => check_images(g, depth + 1, nodes, pixels, stroke_points)?,
                Node::Image(i) => {
                    let size = i.size();
                    *pixels += size.width() as f64 * size.height() as f64;
                    if *pixels > 64. * 1024. * 1024. {
                        return Err(limit("SVG images exceed 64 million total decoded pixels"));
                    }
                }
                Node::Path(path) if path.stroke().is_some_and(|s| s.is_non_scaling()) => {
                    let centerline = path
                        .stroke_centerline()
                        .ok_or_else(|| invalid("SVG non-scaling stroke transform is invalid"))?;
                    if let Some(intervals) = path.stroke().and_then(|s| s.dasharray()) {
                        check_dash_budget(&centerline, intervals)?;
                    }
                    if let Some(outline) = path.stroke_outline(4.) {
                        *stroke_points += outline.len();
                        if *stroke_points > MAX_POINTS {
                            return Err(limit(
                                "SVG non-scaling stroke outline exceeds 500000 segments",
                            ));
                        }
                    }
                }
                _ => {}
            }
            let mut status = Ok(());
            node.subroots(|subroot| {
                if status.is_ok() {
                    status = check_images(subroot, depth + 1, nodes, pixels, stroke_points);
                }
            });
            status?;
        }
        Ok(())
    }
    check_images(tree.root(), 0, &mut 0, &mut 0., &mut 0)?;
    Ok(tree)
}

#[derive(Clone, Copy)]
struct Matrix {
    sx: f64,
    ky: f64,
    kx: f64,
    sy: f64,
    tx: f64,
    ty: f64,
}
impl Matrix {
    fn mul(self, b: Self) -> Self {
        Self {
            sx: self.sx * b.sx + self.kx * b.ky,
            ky: self.ky * b.sx + self.sy * b.ky,
            kx: self.sx * b.kx + self.kx * b.sy,
            sy: self.ky * b.kx + self.sy * b.sy,
            tx: self.sx * b.tx + self.kx * b.ty + self.tx,
            ty: self.ky * b.tx + self.sy * b.ty + self.ty,
        }
    }
    fn pre_concat(self, b: Transform) -> Self {
        self.mul(Self {
            sx: b.sx as f64,
            ky: b.ky as f64,
            kx: b.kx as f64,
            sy: b.sy as f64,
            tx: b.tx as f64,
            ty: b.ty as f64,
        })
    }
}
fn physical_dimension_value(value: Option<&str>, fallback: f64, dpi: f64, legacy: bool) -> f64 {
    use svgtypes::LengthUnit::*;
    let Some(value) = value.and_then(|s| s.parse::<svgtypes::Length>().ok()) else {
        return fallback;
    };
    let unit = match value.unit {
        None => 25.4 / dpi,
        Px => 25.4 / if legacy { 96. } else { dpi },
        In => 25.4,
        Cm => 10.,
        Mm => 1.,
        Pt => 25.4 / 72.,
        Pc => 25.4 / 6.,
        _ => return fallback,
    };
    value.number * unit
}
fn coordinate_transform(
    document: &usvg::roxmltree::Document<'_>,
    tree: &usvg::Tree,
    dpi: f64,
    _legacy: bool,
    width: f64,
    height: f64,
) -> Result<Matrix> {
    let mm = 25.4 / dpi;
    let base = Matrix {
        sx: mm,
        ky: 0.,
        kx: 0.,
        sy: -mm,
        tx: 0.,
        ty: height,
    };
    let root = document.root_element();
    let Some(viewbox) = root.attribute("viewBox") else {
        return Ok(base);
    };
    let vb = viewbox
        .parse::<svgtypes::ViewBox>()
        .map_err(|e| invalid(format!("Invalid SVG viewBox: {e}")))?;
    let aspect = root
        .attribute("preserveAspectRatio")
        .unwrap_or("xMidYMid meet")
        .parse::<svgtypes::AspectRatio>()
        .map_err(|e| invalid(format!("Invalid SVG preserveAspectRatio: {e}")))?;
    use svgtypes::Align::*;
    let (fx, fy) = match aspect.align {
        None | XMinYMin => (0., 0.),
        XMidYMin => (0.5, 0.),
        XMaxYMin => (1., 0.),
        XMinYMid => (0., 0.5),
        XMidYMid => (0.5, 0.5),
        XMaxYMid => (1., 0.5),
        XMinYMax => (0., 1.),
        XMidYMax => (0.5, 1.),
        XMaxYMax => (1., 1.),
    };
    let (mut sx, mut sy) = (width / vb.w, height / vb.h);
    if aspect.align != None {
        let scale = if aspect.slice { sx.max(sy) } else { sx.min(sy) };
        sx = scale;
        sy = scale;
    }
    let desired = Matrix {
        sx,
        ky: 0.,
        kx: 0.,
        sy: -sy,
        tx: -vb.x * sx + (width - vb.w * sx) * fx,
        ty: height + vb.y * sy - (height - vb.h * sy) * fy,
    };
    // Reproduce only usvg's f32 root viewport matrix so it can be removed.
    // All subsequent affine composition and CAD unit conversion stay f64.
    let (mut ax, mut ay) = (
        tree.size().width() / vb.w as f32,
        tree.size().height() / vb.h as f32,
    );
    if aspect.align != None {
        let scale = if aspect.slice { ax.max(ay) } else { ax.min(ay) };
        ax = scale;
        ay = scale;
    }
    let tx = -(vb.x as f32) * ax + (tree.size().width() - vb.w as f32 * ax) * fx as f32;
    let ty = -(vb.y as f32) * ay + (tree.size().height() - vb.h as f32 * ay) * fy as f32;
    Ok(desired.mul(Matrix {
        sx: 1. / ax as f64,
        ky: 0.,
        kx: 0.,
        sy: 1. / ay as f64,
        tx: -(tx as f64) / ax as f64,
        ty: -(ty as f64) / ay as f64,
    }))
}

#[derive(Clone)]
struct Region {
    contours: Rings,
    rule: FillRule,
}
struct Geometry {
    tolerance: f64,
    points: usize,
    nodes: usize,
}
pub(crate) fn check_dash_budget(path: &Path, intervals: &[f32]) -> Result<usize> {
    // The control polygon bounds each Bézier's arc length from above.
    // Admit bounded work before tiny-skia, whose None result conflates an
    // entirely unpainted dash pattern with its own million-dash limit.
    let mut length = 0.;
    let mut contours = 0;
    let mut current = Point::from_xy(0., 0.);
    let mut start = current;
    let distance = |a: Point, b: Point| {
        (f64::from(a.x) - f64::from(b.x)).hypot(f64::from(a.y) - f64::from(b.y))
    };
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(p) => {
                current = p;
                start = p;
                contours += 1;
            }
            PathSegment::LineTo(p) => {
                length += distance(current, p);
                current = p;
            }
            PathSegment::QuadTo(c, p) => {
                length += distance(current, c) + distance(c, p);
                current = p;
            }
            PathSegment::CubicTo(a, b, p) => {
                length += distance(current, a) + distance(a, b) + distance(b, p);
                current = p;
            }
            PathSegment::Close => {
                length += distance(current, start);
                current = start;
            }
        }
    }
    let period: f64 = intervals.iter().map(|&x| f64::from(x)).sum();
    if period == 0. {
        return Ok(0);
    }
    let dashes = (length / period + f64::from(contours)) * intervals.len() as f64 / 2.;
    if !dashes.is_finite() || dashes > MAX_POINTS as f64 / 4. {
        return Err(limit("SVG stroke dash expansion exceeds 125000 dashes"));
    }
    Ok(dashes.ceil() as usize)
}

impl Geometry {
    fn append(&mut self, contour: &mut Vec<[f64; 2]>, p: [f64; 2]) -> Result<()> {
        if !p.iter().all(|v| v.is_finite()) {
            return Err(invalid("Non-finite SVG transformed coordinate"));
        }
        if contour.last() != Some(&p) {
            self.points += 1;
            if self.points > MAX_POINTS {
                return Err(limit(
                    "SVG geometry exceeds 500000 vertices; increase curve tolerance",
                ));
            }
            contour.push(p);
        }
        Ok(())
    }
    fn curve(&mut self, p: &[[f64; 2]], depth: u8, out: &mut Vec<[f64; 2]>) -> Result<()> {
        let a = p[0];
        let b = p[p.len() - 1];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let length2 = dx * dx + dy * dy;
        // Distance to the finite chord (not its infinite line) also detects
        // collinear reversals and cusp loops.
        let flat = p[1..p.len() - 1].iter().all(|q| {
            let t = if length2 == 0. {
                0.
            } else {
                ((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / length2
            }
            .clamp(0., 1.);
            (q[0] - a[0] - t * dx).hypot(q[1] - a[1] - t * dy) <= self.tolerance
        });
        if flat {
            return self.append(out, b);
        }
        if depth >= 28 {
            return Err(limit("SVG curve subdivision budget exceeded"));
        }
        let mut levels = vec![p.to_vec()];
        while levels.last().unwrap().len() > 1 {
            levels.push(
                levels
                    .last()
                    .unwrap()
                    .windows(2)
                    .map(|w| [(w[0][0] + w[1][0]) * 0.5, (w[0][1] + w[1][1]) * 0.5])
                    .collect(),
            );
        }
        let left: Vec<_> = levels.iter().map(|l| l[0]).collect();
        let right: Vec<_> = levels.iter().rev().map(|l| *l.last().unwrap()).collect();
        self.curve(&left, depth + 1, out)?;
        self.curve(&right, depth + 1, out)
    }
    fn flatten(&mut self, path: &Path, transform: Matrix) -> Result<Rings> {
        let map = |p: Point| -> [f64; 2] {
            [
                transform.sx as f64 * p.x as f64
                    + transform.kx as f64 * p.y as f64
                    + transform.tx as f64,
                transform.ky as f64 * p.x as f64
                    + transform.sy as f64 * p.y as f64
                    + transform.ty as f64,
            ]
        };
        let mut contours = Vec::new();
        let mut current = Vec::new();
        let mut at = [0.; 2];
        let finish = |c: &mut Vec<[f64; 2]>, result: &mut Rings| {
            if c.len() > 1 && c.first() == c.last() {
                c.pop();
            }
            if c.len() >= 3 {
                result.push(std::mem::take(c));
            } else {
                c.clear();
            }
        };
        for segment in path.segments() {
            match segment {
                PathSegment::MoveTo(p) => {
                    finish(&mut current, &mut contours);
                    at = map(p);
                    self.append(&mut current, at)?;
                }
                PathSegment::LineTo(p) => {
                    at = map(p);
                    self.append(&mut current, at)?;
                }
                PathSegment::QuadTo(c, p) => {
                    let next = map(p);
                    self.curve(&[at, map(c), next], 0, &mut current)?;
                    at = next;
                }
                PathSegment::CubicTo(a, b, p) => {
                    let next = map(p);
                    self.curve(&[at, map(a), map(b), next], 0, &mut current)?;
                    at = next;
                }
                PathSegment::Close => {
                    if let Some(p) = current.first() {
                        at = *p;
                    }
                    finish(&mut current, &mut contours);
                }
            }
        }
        finish(&mut current, &mut contours);
        Ok(contours)
    }
    fn painted(paint: &Paint) -> Result<bool> {
        Ok(match paint {
            Paint::Color(_) => true,
            Paint::LinearGradient(g) => Self::gradient(g.stops())?,
            Paint::RadialGradient(g) => Self::gradient(g.stops())?,
            Paint::Pattern(_) => return Err(effect("pattern paint")),
        })
    }
    fn gradient(stops: &[usvg::Stop]) -> Result<bool> {
        let painted = stops.iter().any(|s| s.opacity().get() > 0.);
        if painted && stops.iter().any(|s| s.opacity().get() == 0.) {
            return Err(effect("gradient transparency"));
        }
        Ok(painted)
    }
    fn path(&mut self, path: &usvg::Path, ts: Matrix, clipping: bool) -> Result<Vec<Region>> {
        if !path.is_visible() {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        if let Some(fill) = path.fill() {
            if clipping || fill.opacity().get() > 0. && Self::painted(fill.paint())? {
                let contours = self.flatten(path.data(), ts)?;
                if !contours.is_empty() {
                    out.push(Region {
                        contours,
                        rule: match fill.rule() {
                            usvg::FillRule::NonZero => FillRule::NonZero,
                            usvg::FillRule::EvenOdd => FillRule::EvenOdd,
                        },
                    });
                }
            }
        }
        if !clipping {
            if let Some(stroke) = path.stroke() {
                if stroke.opacity().get() > 0. && Self::painted(stroke.paint())? {
                    if stroke.is_non_scaling() {
                        // Width, dashes and joins are defined in the outer SVG
                        // viewport; the result returns to local coordinates so
                        // the same clips and CAD millimeter transform still apply.
                        let centerline = path.stroke_centerline().ok_or_else(|| {
                            invalid("SVG non-scaling stroke transform is invalid")
                        })?;
                        if let Some(intervals) = stroke.dasharray() {
                            check_dash_budget(&centerline, intervals)?;
                        }
                        if let Some(outline) =
                            path.stroke_outline((1. / self.tolerance).clamp(1., 100_000.) as f32)
                        {
                            let contours = self.flatten(&outline, ts)?;
                            if !contours.is_empty() {
                                out.push(Region {
                                    contours,
                                    rule: FillRule::NonZero,
                                });
                            }
                        }
                        return Ok(out);
                    }
                    let style = stroke.to_tiny_skia();
                    let scale = ((ts.sx as f64).hypot(ts.ky as f64)
                        + (ts.kx as f64).hypot(ts.sy as f64))
                        / self.tolerance;
                    let resolution = scale.clamp(1., 100_000.) as f32;
                    let dashed;
                    let data = if let Some(dash) = &style.dash {
                        check_dash_budget(path.data(), stroke.dasharray().unwrap())?;
                        let Some(result) = path.data().dash(dash, resolution) else {
                            return Ok(out);
                        };
                        dashed = result;
                        &dashed
                    } else {
                        path.data()
                    };
                    let outline = data
                        .stroke(&style, resolution)
                        .ok_or_else(|| invalid("SVG stroke expansion failed"))?;
                    let contours = self.flatten(&outline, ts)?;
                    if !contours.is_empty() {
                        out.push(Region {
                            contours,
                            rule: FillRule::NonZero,
                        });
                    }
                }
            }
        }
        Ok(out)
    }
    fn union(regions: Vec<Region>) -> Result<Rings> {
        let mut out = Vec::new();
        for region in regions {
            out = rings::planar_with_rules(
                &out,
                &region.contours,
                "union",
                FillRule::NonZero,
                region.rule,
            )?;
        }
        Ok(out)
    }
    fn clip(&mut self, clip: &usvg::ClipPath, parent: Matrix, depth: usize) -> Result<Rings> {
        if depth > MAX_DEPTH {
            return Err(limit("SVG clip nesting budget exceeded"));
        }
        let ts = parent.pre_concat(clip.transform());
        let mut out = Self::union(self.group(clip.root(), ts, true, depth + 1)?)?;
        if let Some(nested) = clip.clip_path() {
            let other = self.clip(nested, parent, depth + 1)?;
            out = rings::planar_with_rules(
                &out,
                &other,
                "intersection",
                FillRule::NonZero,
                FillRule::NonZero,
            )?;
        }
        Ok(out)
    }
    fn group(
        &mut self,
        group: &usvg::Group,
        parent: Matrix,
        clipping: bool,
        depth: usize,
    ) -> Result<Vec<Region>> {
        if depth > MAX_DEPTH {
            return Err(limit("Normalized SVG nesting budget exceeded"));
        }
        self.nodes += 1;
        if self.nodes > 100_000 {
            return Err(limit("Normalized SVG exceeds 100000 nodes"));
        }
        if !clipping && group.opacity().get() == 0. {
            return Ok(vec![]);
        }
        if !clipping && group.mask().is_some() {
            return Err(effect("mask"));
        }
        if !clipping && !group.filters().is_empty() {
            return Err(effect("filter"));
        }
        let ts = parent.pre_concat(group.transform());
        let mut out = Vec::new();
        for node in group.children() {
            match node {
                Node::Group(g) => out.extend(self.group(g, ts, clipping, depth + 1)?),
                Node::Path(p) => out.extend(self.path(p, ts, clipping)?),
                Node::Text(t) => out.extend(self.group(t.flattened(), ts, clipping, depth + 1)?),
                Node::Image(_) => return Err(effect("embedded image")),
            }
        }
        if let Some(clip) = group.clip_path() {
            let mask = self.clip(clip, ts, depth + 1)?;
            for region in &mut out {
                region.contours = rings::planar_with_rules(
                    &region.contours,
                    &mask,
                    "intersection",
                    region.rule,
                    FillRule::NonZero,
                )?;
                region.rule = FillRule::NonZero;
            }
            out.retain(|r| !r.contours.is_empty());
        }
        Ok(out)
    }
}

pub(crate) fn dispatch(v: Value) -> Result<Value> {
    let source = v["source"]
        .as_str()
        .ok_or_else(|| invalid("SVG source must be a string"))?;
    let action = v["action"].as_str().unwrap_or("parse");
    if !matches!(action, "parse" | "preview") {
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
    Ok(
        json!({"regions":regions.into_iter().map(|r|json!({"contours":r.contours,"fillRule":match r.rule{FillRule::NonZero=>"nonzero",FillRule::EvenOdd=>"evenodd"}})).collect::<Vec<_>>(),"widthMm":width,"heightMm":height,"normalizedSvg":normalized,"warnings":warnings}),
    )
}

#[cfg(test)]
mod tests {
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
        let outline = include_bytes!("../assets/NotoSans-Regular.ttf").to_vec();
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
                while out.len() % 4 != 0 {
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
            let mut red = vec![255, 0, 0, 255].repeat(4);
            let mut first = gif::Frame::from_rgba(2, 2, &mut red);
            first.left = 1;
            first.top = 1;
            first.delay = 10;
            encoder.write_frame(&first).unwrap();
            let mut blue = vec![0, 0, 255, 255].repeat(16);
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
}
