use super::*;

pub(super) fn invalid(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_INVALID_DATA", message)
}
pub(super) fn limit(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_LIMIT", message)
}
pub(super) fn unsupported(message: impl Into<String>) -> Error {
    Error::new("E_IMPORT_UNSUPPORTED_FEATURE", message)
}
pub(super) fn effect(name: &str) -> Error {
    unsupported(format!(
        "SVG {name} requires rendered silhouette mode. Select silhouette to convert the rendered alpha channel to contours, or convert this effect to vector paths."
    ))
}
pub(super) fn number(v: &Value, key: &str, default: f64, min: f64, max: f64) -> Result<f64> {
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

pub(super) fn check_urls(value: &str) -> Result<()> {
    let lower = value.to_ascii_lowercase();
    // Inspect every declaration independently. A `none` declaration elsewhere
    // in a stylesheet must not hide an unsupported value on another element.
    // Delimiters inside quoted strings/comments are not CSS declarations.
    let check_declaration = |declaration: &str| -> Result<()> {
        if let Some((name, value)) = declaration.split_once(':')
            && name.trim() == "vector-effect"
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
pub(super) fn arc_units(radius: f64) -> usize {
    let count = (11.163 * radius.abs()).powf(1. / 6.).max(4.).ceil() + 1.;
    if !count.is_finite() || count > MAX_POINTS as f64 {
        MAX_POINTS + 1
    } else {
        count as usize
    }
}
pub(super) fn path_units(data: &str) -> usize {
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
pub(super) struct MarkerLink<'a, 'input> {
    pub(super) node: usvg::roxmltree::Node<'a, 'input>,
    pub(super) positions: u8, // start = 1, mid = 2, end = 4
}
pub(super) fn merge_markers<'a, 'input>(
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
pub(super) struct CssNode<'a, 'input>(pub(super) usvg::roxmltree::Node<'a, 'input>);
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

pub(super) fn preflight(source: &str) -> Result<usvg::roxmltree::Document<'_>> {
    preflight_inner(source, 0, true)
}
