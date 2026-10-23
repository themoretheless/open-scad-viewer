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
    let lower: std::borrow::Cow<'_, str> = if value.bytes().any(|b| b.is_ascii_uppercase()) {
        std::borrow::Cow::Owned(value.to_ascii_lowercase())
    } else {
        std::borrow::Cow::Borrowed(value)
    };
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
    let mut declaration = String::with_capacity(64);
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
    let mut tail: &str = lower.as_ref();
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
                transform.sx * p.x as f64 + transform.kx * p.y as f64 + transform.tx,
                transform.ky * p.x as f64 + transform.sy * p.y as f64 + transform.ty,
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
        if let Some(fill) = path.fill()
            && (clipping || fill.opacity().get() > 0. && Self::painted(fill.paint())?)
        {
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
        if !clipping
            && let Some(stroke) = path.stroke()
            && stroke.opacity().get() > 0.
            && Self::painted(stroke.paint())?
        {
            if stroke.is_non_scaling() {
                // Width, dashes and joins are defined in the outer SVG
                // viewport; the result returns to local coordinates so
                // the same clips and CAD millimeter transform still apply.
                let centerline = path
                    .stroke_centerline()
                    .ok_or_else(|| invalid("SVG non-scaling stroke transform is invalid"))?;
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
            let scale = (ts.sx.hypot(ts.ky) + ts.kx.hypot(ts.sy)) / self.tolerance;
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



#[cfg(test)]
#[path = "svg/tests/tests_fixtures.rs"]
mod tests;

#[path = "svg/raster_normalization.rs"]
mod raster_normalization;
use raster_normalization::normalize_raster_image;
use raster_normalization::legacy_source;
pub(crate) use raster_normalization::parse_tree;

#[path = "svg/preflight_validation.rs"]
mod preflight_validation;
use preflight_validation::preflight_inner;

#[path = "svg/operation_dispatch.rs"]
mod operation_dispatch;
pub(crate) use operation_dispatch::dispatch;
