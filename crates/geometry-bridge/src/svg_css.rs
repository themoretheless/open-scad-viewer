//! CSS geometry admission. The original XML is rendered by patched usvg; this
//! conservative copy is only inspected by the existing geometry/resource budget.
//! Rewriting the rendered XML would change attribute-selector matching.
use crate::{Error, Result};
use std::collections::{HashMap, HashSet};
use usvg::roxmltree::{Document, Node, NodeId};

const MAX_SOURCE: usize = 4 * 1024 * 1024;
const MAX_CHECKS: usize = 2_000_000;
const PROPERTIES: [&str; 10] = [
    "cx", "cy", "r", "rx", "ry", "x", "y", "width", "height", "d",
];

#[derive(Debug)]
pub(crate) struct Analysis {
    pub admission_source: String,
    pub root_width: Option<String>,
    pub root_height: Option<String>,
}
fn limit(message: &str) -> Error {
    Error::new("E_IMPORT_LIMIT", message)
}
fn unsupported(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_UNSUPPORTED_FEATURE", message)
}
fn invalid(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_INVALID_DATA", message)
}

struct CssNode<'a, 'input>(Node<'a, 'input>);
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
        self.0.attribute(name).is_some_and(|v| op.matches(v))
    }
    fn pseudo_class_matches(&self, class: simplecss::PseudoClass<'_>) -> bool {
        matches!(class, simplecss::PseudoClass::FirstChild)
            && self.0.prev_sibling_element().is_none()
    }
}
#[derive(Clone)]
struct Winner {
    value: String,
    important: bool,
}
type Values = HashMap<&'static str, Winner>;

fn put(values: &mut Values, name: &str, value: &str, important: bool) -> Result<()> {
    let Some(&property) = PROPERTIES.iter().find(|p| p.eq_ignore_ascii_case(name)) else {
        return Ok(());
    };
    // Valid modern functions must not be mistaken for invalid declarations and
    // silently discarded by the deliberately small SVG CSS value parser.
    let lower = value.to_ascii_lowercase();
    if (lower.contains('(') && property != "d")
        || lower.contains("var(")
        || lower.contains("env(")
        || lower == "revert-layer"
    {
        return Err(unsupported(format!(
            "SVG CSS {property} value {value:?} requires unsupported CSS math, variables or cascade layers; supply a resolved length/path"
        )));
    }
    if property != "d"
        && [
            "rem", "vw", "vh", "vmin", "vmax", "svw", "svh", "lvw", "lvh", "dvw", "dvh", "ch",
            "ic", "lh", "rlh", "cap",
        ]
        .iter()
        .any(|unit| {
            lower
                .strip_suffix(unit)
                .is_some_and(|n| n.trim().parse::<f64>().is_ok())
        })
    {
        return Err(unsupported(format!(
            "SVG CSS {property} uses an unsupported viewport/font-relative unit; supply px, physical units, em/ex or percent"
        )));
    }
    if property != "d" {
        let unit_start = lower
            .trim_end()
            .rfind(|c: char| !c.is_ascii_alphabetic())
            .map_or(0, |i| i + 1);
        let unit = lower[unit_start..].trim();
        if !unit.is_empty()
            && lower[..unit_start].trim().parse::<f64>().is_ok()
            && !["px", "em", "ex", "in", "cm", "mm", "pt", "pc", "q"].contains(&unit)
        {
            return Err(unsupported(format!(
                "SVG CSS {property} unit {unit:?} is not supported; resolve it to px, physical units, em/ex or percent"
            )));
        }
    }
    if let Some(normalized) = usvg::normalize_geometry_property(property, value)
        && values
            .get(property)
            .is_none_or(|old| important || !old.important)
        {
            values.insert(
                property,
                Winner {
                    value: normalized,
                    important,
                },
            );
        }
    Ok(())
}
fn append_declaration(
    styles: &mut HashMap<NodeId, String>,
    node: NodeId,
    declaration: &simplecss::Declaration<'_>,
) -> Result<()> {
    let value = styles.entry(node).or_default();
    let extra = declaration
        .name
        .len()
        .saturating_add(declaration.value.len())
        .saturating_add(12);
    if value.len().saturating_add(extra) > MAX_SOURCE {
        return Err(limit("SVG CSS declaration expansion exceeds 4 MiB"));
    }
    value.push(';');
    value.push_str(&declaration.name.to_ascii_lowercase());
    value.push(':');
    value.push_str(declaration.value);
    if declaration.important {
        value.push_str("!important");
    }
    Ok(())
}
fn initial(property: &str) -> &'static str {
    match property {
        "width" | "height" | "rx" | "ry" => "auto",
        "d" => "",
        _ => "0",
    }
}
fn xml_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;")
}

/// Analyze source after its XML/resource security preflight, before any usvg
/// normalization. The returned admission copy must never be rendered/exported.
pub(crate) fn analyze(source: &str) -> Result<Analysis> {
    if source.len() > MAX_SOURCE {
        return Err(limit("SVG CSS input exceeds 4 MiB"));
    }
    let lower = source.to_ascii_lowercase();
    if lower.contains("<!doctype") || lower.contains("<!entity") {
        return Err(invalid("SVG DTDs and entities are forbidden"));
    }
    let document =
        Document::parse(source).map_err(|e| invalid(format!("Malformed SVG XML: {e}")))?;
    let mut sheet = simplecss::StyleSheet::new();
    let mut sheets = Vec::new();
    let mut nodes = 0;
    for node in document.descendants().filter(|n| n.is_element()) {
        nodes += 1;
        if nodes > 50_000 || node.ancestors().count() > 65 {
            return Err(limit("SVG CSS source tree budget exceeded"));
        }
        if node.has_tag_name("style") && matches!(node.attribute("type"), None | Some("text/css")) {
            let css = node.text().unwrap_or("");
            validate_stylesheet(css)?;
            sheets.push(usvg::normalize_css_important(css));
        }
    }
    for css in &sheets {
        sheet.parse_more(css);
        if sheet.rules.len() > 50_000 {
            return Err(limit("SVG CSS exceeds 50000 rules"));
        }
    }
    // usvg matches source selectors again for every expanded <use> node.
    if !sheet.rules.is_empty() {
        let ids: HashMap<_, _> = document
            .descendants()
            .filter_map(|n| n.attribute("id").map(|id| (id, n)))
            .collect();
        fn work<'a, 'input>(
            node: Node<'a, 'input>,
            ids: &HashMap<&str, Node<'a, 'input>>,
            rule_count: usize,
            stack: &mut Vec<NodeId>,
            total: &mut usize,
        ) -> Result<()> {
            if stack.len() > 128 {
                return Err(limit("SVG CSS reference depth exceeded"));
            }
            if stack.contains(&node.id()) {
                return Err(invalid("Cyclic SVG CSS use reference"));
            }
            *total = total.saturating_add(rule_count);
            if *total > MAX_CHECKS {
                return Err(limit("Expanded SVG CSS exceeds 2000000 selector checks"));
            }
            stack.push(node.id());
            if node.has_tag_name("use") {
                for attr in node.attributes().filter(|a| a.name() == "href") {
                    if let Some(target) = attr.value().strip_prefix('#').and_then(|id| ids.get(id))
                    {
                        work(*target, ids, rule_count, stack, total)?;
                    }
                }
            }
            for child in node.children().filter(|n| n.is_element()) {
                work(child, ids, rule_count, stack, total)?;
            }
            stack.pop();
            Ok(())
        }
        work(
            document.root_element(),
            &ids,
            sheet.rules.len(),
            &mut Vec::new(),
            &mut 0,
        )?;
    }
    // Keep font/paint declarations in the original source; geometry admission
    // and the renderer match selectors against the same unmodified attributes.
    let mut selectors = 0usize;
    let mut values_by_node: HashMap<NodeId, Values> = HashMap::new();
    let mut paths = HashSet::<String>::new();
    let mut lengths = HashSet::<String>::new();
    let mut css_styles = HashMap::<NodeId, String>::new();
    let mut storage_bytes = 0usize;
    for node in document.descendants().filter(|n| n.is_element()) {
        let mut values = Values::new();
        for property in PROPERTIES {
            if usvg::geometry_property_applies(property, node.tag_name().name())
                && let Some(value) = node.attribute(property) {
                    if property == "d" {
                        values.insert(
                            property,
                            Winner {
                                value: value.into(),
                                important: false,
                            },
                        );
                    } else {
                        put(&mut values, property, value, false)?;
                    }
                }
        }
        for rule in &sheet.rules {
            selectors += 1;
            if selectors > MAX_CHECKS {
                return Err(limit("SVG CSS exceeds 2000000 selector checks"));
            }
            if rule.selector.matches(&CssNode(node)) {
                for declaration in &rule.declarations {
                    append_declaration(&mut css_styles, node.id(), declaration)?;
                    put(
                        &mut values,
                        declaration.name,
                        declaration.value,
                        declaration.important,
                    )?;
                }
            }
        }
        if let Some(style) = node.attribute("style") {
            validate_declarations(style)?;
            let normalized = usvg::normalize_css_important(style);
            for declaration in simplecss::DeclarationTokenizer::from(normalized.as_str()) {
                append_declaration(&mut css_styles, node.id(), &declaration)?;
                put(
                    &mut values,
                    declaration.name,
                    declaration.value,
                    declaration.important,
                )?;
            }
        }
        storage_bytes = storage_bytes
            .saturating_add(values.values().map(|v| v.value.len()).sum::<usize>())
            .saturating_add(css_styles.get(&node.id()).map_or(0, String::len));
        if storage_bytes > MAX_SOURCE {
            return Err(limit("SVG CSS computed-value storage exceeds 4 MiB"));
        }
        for (property, winner) in &values {
            if *property == "d" {
                if !["inherit", "initial", "unset", "revert", "none", ""]
                    .contains(&winner.value.as_str())
                {
                    paths.insert(winner.value.clone());
                }
            } else if winner.value.parse::<svgtypes::Length>().is_ok() {
                lengths.insert(winner.value.clone());
            }
        }
        if !values.is_empty() {
            values_by_node.insert(node.id(), values);
        }
    }
    // A referenced root inherits from each <use> instance. Charge every possible
    // path as an upper bound, without materializing the shadow tree.
    let inherited_path = paths.into_iter().collect::<Vec<_>>().join(" M0 0 ");
    if inherited_path.len() > MAX_SOURCE {
        return Err(limit("SVG CSS inherited path budget exceeded"));
    }
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    let root = document.root_element();
    let root_value = |property: &str| -> Option<String> {
        values_by_node
            .get(&root.id())
            .and_then(|v| v.get(property))
            .map(|winner| {
                if ["inherit", "initial", "unset", "revert"].contains(&winner.value.as_str()) {
                    initial(property).into()
                } else {
                    winner.value.clone()
                }
            })
    };
    let root_width = root_value("width");
    let root_height = root_value("height");
    let mut has_inherited_length = false;
    for node in document.descendants().filter(|n| n.is_element()) {
        let mut add = String::new();
        if let Some(markers) = css_styles.get(&node.id()) {
            if let Some(style) = node.attribute_node("style") {
                edits.push((
                    style.range_value(),
                    xml_attr(&format!("{}{}", style.value(), markers)),
                ));
            } else {
                add.push_str(&format!(" style=\"{}\"", xml_attr(markers)));
            }
        }
        for (property, winner) in values_by_node.get(&node.id()).into_iter().flatten() {
            let mut value = winner.value.clone();
            if value == "inherit" {
                if *property == "d" {
                    value = inherited_path.clone();
                } else {
                    value = "100%".into();
                    has_inherited_length = true;
                }
            } else if ["initial", "unset", "revert"].contains(&value.as_str()) {
                value = initial(property).into();
            }
            if value == "auto" {
                value = "0".into();
            }
            if !usvg::geometry_property_applies(property, node.tag_name().name())
                && *property != "d"
            {
                continue;
            }
            if let Some(attr) = node.attribute_node(*property) {
                edits.push((attr.range_value(), xml_attr(&value)));
            } else {
                add.push_str(&format!(" {property}=\"{}\"", xml_attr(&value)));
            }
        }
        if !add.is_empty() {
            let range = node.range();
            let tail = &source[range.start + 1..];
            let name_end = range.start
                + 1
                + tail
                    .find(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/')
                    .ok_or_else(|| invalid("Malformed SVG element"))?;
            edits.push((name_end..name_end, add));
        }
    }
    if has_inherited_length {
        // Existing SVG preflight derives a conservative viewport/font bound from
        // all authored length values. Include CSS lengths in that same estimate.
        let mut extra = String::new();
        for length in lengths {
            extra.push_str(&format!("<g font-size=\"{}\"/>", xml_attr(&length)));
        }
        let range = root.range();
        if !source[range.clone()].trim_end().ends_with("/>") {
            let pos = source[..range.end]
                .rfind("</")
                .ok_or_else(|| invalid("Malformed SVG closing tag"))?;
            edits.push((pos..pos, extra));
        }
    }
    edits.sort_by_key(|e| e.0.start);
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    for (range, replacement) in edits {
        if range.start < cursor {
            return Err(invalid("Overlapping SVG CSS admission edits"));
        }
        if output
            .len()
            .saturating_add(range.start - cursor)
            .saturating_add(replacement.len())
            > MAX_SOURCE
        {
            return Err(limit("SVG CSS expansion exceeds 4 MiB"));
        }
        output.push_str(&source[cursor..range.start]);
        output.push_str(&replacement);
        cursor = range.end;
    }
    output.push_str(&source[cursor..]);
    if output.len() > MAX_SOURCE {
        return Err(limit("SVG CSS expansion exceeds 4 MiB"));
    }
    Ok(Analysis {
        admission_source: output,
        root_width,
        root_height,
    })
}

fn validate_declarations(css: &str) -> Result<()> {
    for declaration in simplecss::DeclarationTokenizer::from(css) {
        if ["all", "min-width", "max-width", "min-height", "max-height"]
            .iter()
            .any(|p| p.eq_ignore_ascii_case(declaration.name))
        {
            return Err(unsupported(format!(
                "SVG CSS {} is not yet supported; resolve sizing/reset constraints explicitly",
                declaration.name
            )));
        }
    }
    Ok(())
}
pub(crate) fn validate_stylesheet(css: &str) -> Result<()> {
    // The renderer's selector implementation is intentionally SVG-sized. Reject
    // valid but unsupported conditional/layered rules instead of losing geometry.
    // Resource/style preflight has already rejected loading and CSS escapes.
    let mut start = 0;
    let mut depth = 0usize;
    let mut quote = None;
    let mut body_start = 0;
    let mut rule_count = 0usize;
    let mut expanded_declarations = 0usize;
    let bytes = css.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = quote {
            if c == b'\\' {
                i += 1;
            } else if c == q {
                quote = None;
            }
        } else if c == b'/' && bytes.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < bytes.len() && &bytes[i..i + 2] != b"*/" {
                i += 1;
            }
            i += 1;
        } else if c == b'\'' || c == b'"' {
            quote = Some(c);
        } else if c == b'{' {
            if depth == 0 {
                body_start = i + 1;
            }
            depth += 1;
        } else if c == b'}' {
            if depth == 0 {
                return Err(invalid("Unbalanced SVG CSS rule"));
            }
            depth -= 1;
            if depth == 0 {
                rule_count += 1;
                if rule_count > 50_000 {
                    return Err(limit("SVG CSS exceeds 50000 rules"));
                }
                let selector = css[start..body_start - 1].trim();
                let body = &css[body_start..i];
                let selectors = selector.split(',').count();
                let declarations = simplecss::DeclarationTokenizer::from(body).count();
                expanded_declarations =
                    expanded_declarations.saturating_add(selectors.saturating_mul(declarations));
                if expanded_declarations > 500_000 {
                    return Err(limit(
                        "SVG CSS selector-list expansion exceeds 500000 declarations",
                    ));
                }
                for part in selector.split(',') {
                    if part.len() > 16_384
                        || part
                            .bytes()
                            .filter(|c| matches!(c, b'.' | b'#' | b'[' | b':'))
                            .count()
                            > 200
                    {
                        return Err(limit(
                            "SVG CSS selector complexity exceeds the static profile",
                        ));
                    }
                }
                validate_declarations(body)?;
                let geometry = simplecss::DeclarationTokenizer::from(body)
                    .any(|d| PROPERTIES.iter().any(|p| p.eq_ignore_ascii_case(d.name)));
                if selector.starts_with('@') {
                    return Err(unsupported(
                        "SVG conditional CSS and cascade layers must be resolved before import",
                    ));
                }
                if geometry {
                    for part in selector.split(',') {
                        if part.contains(":link") || part.contains(":lang(") {
                            return Err(unsupported(
                                "SVG geometry CSS :link/:lang selectors require a link/language style context that is not available in the static renderer",
                            ));
                        }
                        if simplecss::Selector::parse(part.trim()).is_none() {
                            return Err(unsupported(format!(
                                "SVG CSS geometry selector {part:?} is not supported"
                            )));
                        }
                    }
                }
                start = i + 1;
            }
        }
        i += 1;
    }
    if depth != 0 || quote.is_some() {
        return Err(invalid("Unterminated SVG CSS rule/string"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tree(body: &str) -> usvg::Tree {
        usvg::Tree::from_str(
            &format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='200' height='100'>{body}</svg>"
            ),
            &usvg::Options::default(),
        )
        .unwrap()
    }
    fn paths(tree: &usvg::Tree) -> Vec<&usvg::Path> {
        fn walk<'a>(group: &'a usvg::Group, out: &mut Vec<&'a usvg::Path>) {
            for node in group.children() {
                match node {
                    usvg::Node::Path(path) => out.push(path),
                    usvg::Node::Group(group) => walk(group, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(tree.root(), &mut out);
        out
    }
    fn close(a: f32, b: f32) {
        assert!((a - b).abs() < 0.001, "{a} != {b}");
    }
    #[test]
    fn geometry_cascade_applies_specificity_source_order_and_inline_important() {
        let tree = tree(
            "<style>rect{width:10px!important} .r{width:20px!important;width:30px!important} #r{width:40px!important}</style><rect id='r' class='r' width='2' height='10' style='width:50px!important;width:99px'/>",
        );
        close(paths(&tree)[0].data().bounds().width(), 50.);
        let tree =
            self::tree("<style>.r{WIDTH:20px}</style><rect class='r' width='2' height='10'/>");
        close(paths(&tree)[0].data().bounds().width(), 20.);
    }
    #[test]
    fn case_insensitive_important_keeps_quoted_data_unchanged() {
        let source = "<svg width='100' height='100'><style>.r{WIDTH:20px !/**/IMPORTANT}</style><rect class='r' width='2' height='10' style='width:30px !ImPoRtAnT;width:99px'/></svg>";
        let admitted = analyze(source).unwrap();
        let document = Document::parse(&admitted.admission_source).unwrap();
        assert_eq!(
            document
                .descendants()
                .find(|n| n.has_tag_name("rect"))
                .unwrap()
                .attribute("width"),
            Some("30px")
        );
        let rendered = usvg::Tree::from_str(source, &usvg::Options::default()).unwrap();
        close(paths(&rendered)[0].data().bounds().width(), 30.);
        assert_eq!(
            usvg::normalize_css_important("[data-note='!IMPORTANT']{fill:red !IMPORTANT}"),
            "[data-note='!IMPORTANT']{fill:red !important}"
        );
    }
    #[test]
    fn admission_preserves_marker_and_font_dependencies_of_original_selectors() {
        let source = "<svg><style>path[d='M0 0H10']{d:path(\"M0 0H20\");MARKER-MID:url(#m);FONT-SIZE:1000em}</style><path d='M0 0H10' style='MARKER-END:url(#n) !IMPORTANT'/></svg>";
        let admitted = analyze(source).unwrap();
        let doc = Document::parse(&admitted.admission_source).unwrap();
        let path = doc.descendants().find(|n| n.has_tag_name("path")).unwrap();
        let style = path.attribute("style").unwrap();
        assert!(style.contains("marker-mid:url(#m)"));
        assert!(style.contains("marker-end:url(#n)!important"));
        assert!(style.contains("font-size:1000em"));
        assert_eq!(path.attribute("d"), Some("M0 0H20"));
    }
    #[test]
    fn selector_list_cross_product_is_rejected_before_css_parser_allocation() {
        let selector = vec![".a"; 1000].join(",");
        let declarations = "width:1px;".repeat(1000);
        assert_eq!(
            validate_stylesheet(&format!("{selector}{{{declarations}}}"))
                .unwrap_err()
                .code,
            "E_IMPORT_LIMIT"
        );
        let path = format!("M0 0{}", "L1 1 ".repeat(150_000));
        let source =
            format!("<svg><style>.a,.a,.a,.a{{d:path(\"{path}\")}}</style><path class='a'/></svg>");
        assert_eq!(analyze(&source).unwrap_err().code, "E_IMPORT_LIMIT");
    }
    #[test]
    fn large_shared_css_path_is_limited_while_values_are_accumulated() {
        let data = format!("M0 0{}", "L1 1 ".repeat(150_000));
        let source = format!(
            "<svg><style>path{{d:path(\"{data}\")}}</style>{}</svg>",
            "<path/>".repeat(10)
        );
        assert!(source.len() < MAX_SOURCE);
        assert_eq!(analyze(&source).unwrap_err().code, "E_IMPORT_LIMIT");
    }
    #[test]
    fn geometry_d_path_and_none_preserve_path_semantics() {
        let tree = tree(
            "<style>.p{d:path('M1 2H21V12H1Z')}</style><path class='p' d='M0 0H2V2Z'/><path d='M0 0H5V5Z' style='d:none'/>",
        );
        assert_eq!(paths(&tree).len(), 1);
        close(paths(&tree)[0].data().bounds().width(), 20.);
        assert_eq!(
            usvg::normalize_geometry_property("d", "path('\\4d 0 0L1 1')").as_deref(),
            Some("M0 0L1 1")
        );
    }
    #[test]
    fn geometry_defaults_are_not_implicitly_inherited() {
        let tree = tree(
            "<g style='r:10px;cx:20px;cy:20px'><circle style='r:inherit;cx:inherit;cy:inherit'/><circle/><circle r='5' cx='70' cy='20' style='r:unset'/></g><ellipse cx='100' cy='20' rx='8' ry='2' style='ry:initial'/>",
        );
        let paths = paths(&tree);
        assert_eq!(paths.len(), 2);
        close(paths[0].data().bounds().width(), 20.);
        close(paths[1].data().bounds().height(), 16.);
    }
    #[test]
    fn inherit_uses_computed_parent_font_units() {
        let tree = tree(
            "<g style='font-size:10px;r:2em'><circle cx='30' cy='30' style='font-size:50px;r:inherit'/></g><g font-size='10'><g font-size='2em'><circle cx='90' cy='30' r='1em' font-size='inherit'/></g></g>",
        );
        for path in paths(&tree) {
            close(path.data().bounds().width(), 40.);
        }
    }
    #[test]
    fn geometry_inherit_is_resolved_for_each_use_instance() {
        let tree = tree(
            "<defs><circle id='c' cy='20' style='r:inherit'/></defs><use href='#c' x='20' style='r:5px'/><use href='#c' x='70' style='r:10px'/>",
        );
        let paths = paths(&tree);
        assert_eq!(paths.len(), 2);
        close(paths[0].data().bounds().width(), 10.);
        close(paths[1].data().bounds().width(), 20.);
    }
    #[test]
    fn css_geometry_respects_symbol_viewport_percentages_and_units() {
        let tree = tree(
            "<defs><symbol id='s' viewBox='0 0 100 100'><rect style='x:10%;y:10%;width:50%;height:20%'/></symbol></defs><use href='#s' style='width:100px;height:100px'/><rect y='70' style='width:40q;height:1cm'/>",
        );
        let paths = paths(&tree);
        close(paths[0].data().bounds().width(), 50.);
        close(paths[0].data().bounds().height(), 20.);
        close(paths[1].data().bounds().width(), 96. / 2.54);
    }
    #[test]
    fn admission_does_not_change_rendered_attribute_selectors() {
        let source = "<svg width='100' height='100'><style>rect[width='2']{width:20px;fill:red}</style><rect width='2' height='10'/></svg>";
        let admitted = analyze(source).unwrap();
        assert!(admitted.admission_source.contains("width='20px'"));
        let rendered = usvg::Tree::from_str(source, &usvg::Options::default()).unwrap();
        let path = paths(&rendered)[0];
        close(path.data().bounds().width(), 20.);
        assert!(
            matches!(path.fill().unwrap().paint(), usvg::Paint::Color(color) if color.red == 255 && color.green == 0)
        );
    }
    #[test]
    fn effective_root_size_and_admission_retain_cascade_and_path_cost() {
        let source = "<svg width='2' height='3' style='width:20mm;height:30mm!important;height:2mm'><style>.p{d:path(\"M0 0H10V10Z\")}</style><g style='d:path(\"M0 0H20V20Z\")'><path class='p'/><path style='d:inherit'/></g></svg>";
        let admitted = analyze(source).unwrap();
        assert_eq!(admitted.root_width.as_deref(), Some("20mm"));
        assert_eq!(admitted.root_height.as_deref(), Some("30mm"));
        let document = Document::parse(&admitted.admission_source).unwrap();
        for node in document.descendants().filter(|n| n.has_tag_name("path")) {
            assert!(node.attribute("d").unwrap().contains("H10"));
        }
    }
    #[test]
    fn supported_css_selector_combinators_preserve_geometry() {
        let source = "<svg><style>g > rect.a[data-x='v'] + circle:first-child{r:2px} g rect.a[data-x='v']{width:12px}</style><g><rect class='a' data-x='v' height='10'/></g></svg>";
        analyze(source).unwrap();
        let rendered = usvg::Tree::from_str(source, &usvg::Options::default()).unwrap();
        close(paths(&rendered)[0].data().bounds().width(), 12.);
    }
    #[test]
    fn unsupported_valid_css_is_explicit_and_css_expansion_is_bounded() {
        for declaration in [
            "width:calc(2px + 3px)",
            "width:var(--w)",
            "r:clamp(1px,2px,3px)",
            "width:10vw",
            "width:10dvi",
            "height:1rem",
            "width:revert-layer",
            "min-width:5px",
        ] {
            let source = format!("<svg><rect width='20' height='10' style='{declaration}'/></svg>");
            assert_eq!(
                analyze(&source).unwrap_err().code,
                "E_IMPORT_UNSUPPORTED_FEATURE",
                "{declaration}"
            );
        }
        assert!(
            analyze(
                "<svg><style>@media screen {rect {width:20px}}</style><rect height='10'/></svg>"
            )
            .is_err()
        );
        for selector in ["a:link rect", "rect:lang(en)"] {
            assert_eq!(
                analyze(&format!(
                    "<svg><style>{selector}{{width:10px}}</style><rect height='10'/></svg>"
                ))
                .unwrap_err()
                .code,
                "E_IMPORT_UNSUPPORTED_FEATURE"
            );
        }
        let css = "rect{width:1px}".repeat(1000);
        let shapes = "<rect height='1'/>".repeat(2100);
        assert_eq!(
            analyze(&format!("<svg><style>{css}</style>{shapes}</svg>"))
                .unwrap_err()
                .code,
            "E_IMPORT_LIMIT"
        );
    }
}
