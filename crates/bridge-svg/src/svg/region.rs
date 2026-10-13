use super::*;

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
                transform.sx * p.x as f64
                    + transform.kx * p.y as f64
                    + transform.tx,
                transform.ky * p.x as f64
                    + transform.sy * p.y as f64
                    + transform.ty,
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
            && (clipping || fill.opacity().get() > 0. && Self::painted(fill.paint())?) {
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
                && stroke.opacity().get() > 0. && Self::painted(stroke.paint())? {
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
                    let scale = (ts.sx.hypot(ts.ky)
                        + ts.kx.hypot(ts.sy))
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
