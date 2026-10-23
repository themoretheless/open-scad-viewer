use super::*;

/// OpenSCAD distinguishes explicit CSS px (96/in) from unitless coordinates
/// (the import dpi). usvg correctly uses one user-unit scale, so translate the
/// explicit px lengths to physical points only for the compatibility entrypoint.
pub(super) fn legacy_source(source: &str, doc: &usvg::roxmltree::Document<'_>, dpi: f64) -> String {
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
                if value[i..].starts_with("px")
                    && let Ok(n) = value[start..i].parse::<f64>() {
                        output.push_str(&format!("{}pt", n * 0.75));
                        i += 2;
                        continue;
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
pub(super) fn normalize_raster_image(mime: &str, data: std::sync::Arc<Vec<u8>>) -> Result<usvg::ImageKind> {
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

pub(super) fn admit_outline_font(bytes: &[u8]) -> Result<()> {
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
        .load_font_data(include_bytes!("../../../geometry-bridge/assets/NotoSans-Regular.ttf").to_vec());
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
pub(super) struct Matrix {
    pub(super) sx: f64,
    pub(super) ky: f64,
    pub(super) kx: f64,
    pub(super) sy: f64,
    pub(super) tx: f64,
    pub(super) ty: f64,
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
    pub(super) fn pre_concat(self, b: Transform) -> Self {
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
pub(super) fn physical_dimension_value(value: Option<&str>, fallback: f64, dpi: f64, legacy: bool) -> f64 {
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
pub(super) fn coordinate_transform(
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
