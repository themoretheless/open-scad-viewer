//! Bounded vector drawing PDF with embedded searchable Unicode text.
use crate::{Error, Result};
use std::{collections::BTreeMap, fmt::Write};
const FONT: &[u8] = include_bytes!("../assets/NotoSans-Regular.ttf");
fn unicode_hex(text: &str) -> String {
    text.encode_utf16().map(|v| format!("{v:04X}")).collect()
}
use usvg::{Node, Paint, tiny_skia_path::PathSegment};
const MAX_BYTES: usize = 32 * 1024 * 1024;
fn invalid(message: &str) -> Error {
    Error::new("DRAWING_PDF_UNSUPPORTED", message)
}
fn color(paint: &Paint) -> Result<[f32; 3]> {
    match paint {
        Paint::Color(c) => Ok([
            c.red as f32 / 255.,
            c.green as f32 / 255.,
            c.blue as f32 / 255.,
        ]),
        _ => Err(invalid("Vector drawings require solid colors")),
    }
}
struct Writer<'a> {
    font: &'a ttf_parser::Face<'a>,
    fontdb: &'a usvg::fontdb::Database,
    glyphs: &'a mut BTreeMap<u16, (String, u16)>,
    checked_fonts: Vec<usvg::fontdb::ID>,
    content: String,
    nodes: usize,
    segments: usize,
}
impl Writer<'_> {
    fn text(&mut self, text: &usvg::Text) -> Result<()> {
        for span in text.layouted() {
            if !span.visible {
                continue;
            }
            if span.stroke.is_some() || span.fill.as_ref().is_some_and(|f| f.opacity().get() != 1.)
            {
                return Err(invalid("PDF text requires opaque fills without strokes"));
            }
            let Some(fill) = &span.fill else {
                continue;
            };
            let c = color(fill.paint())?;
            for glyph in &span.positioned_glyphs {
                if !self.checked_fonts.contains(&glyph.font) {
                    if self
                        .fontdb
                        .with_face_data(glyph.font, |bytes, index| index == 0 && bytes == FONT)
                        != Some(true)
                    {
                        return Err(invalid("PDF text requires the bundled Noto Sans font"));
                    }
                    self.checked_fonts.push(glyph.font);
                }
                let advance = self.font.glyph_hor_advance(glyph.id).unwrap_or(0);
                self.glyphs
                    .entry(glyph.id.0)
                    .or_insert((glyph.text.clone(), advance));
                let t = glyph.outline_transform();
                writeln!(self.content, "q {} {} {} rg /Span << /ActualText <FEFF{}> >> BDC BT /F0 {} Tf {} {} {} {} {} {} Tm <{:04X}> Tj ET EMC Q", c[0],c[1],c[2],unicode_hex(&glyph.text), self.font.units_per_em(), t.sx,t.ky,t.kx,t.sy,t.tx,t.ty,glyph.id.0).unwrap();
            }
            for decoration in [&span.underline, &span.overline, &span.line_through]
                .into_iter()
                .flatten()
            {
                self.path(decoration)?;
            }
        }
        Ok(())
    }
    fn group(&mut self, g: &usvg::Group, depth: usize) -> Result<()> {
        if depth > 128 || self.nodes > 100000 {
            return Err(invalid("Drawing node/depth budget exceeded"));
        }
        if g.opacity().get() != 1.
            || g.mask().is_some()
            || g.clip_path().is_some()
            || !g.filters().is_empty()
        {
            return Err(invalid(
                "Drawing PDF does not admit opacity, masks, clips or filters",
            ));
        }
        let t = g.transform();
        writeln!(
            self.content,
            "q {} {} {} {} {} {} cm",
            t.sx, t.ky, t.kx, t.sy, t.tx, t.ty
        )
        .unwrap();
        for node in g.children() {
            self.nodes += 1;
            match node {
                Node::Group(child) => self.group(child, depth + 1)?,
                Node::Text(text) => self.text(text)?,
                Node::Path(path) => self.path(path)?,
                Node::Image(_) => return Err(invalid("Vector drawings do not admit images")),
            }
        }
        self.content.push_str("Q\n");
        Ok(())
    }
    fn path(&mut self, p: &usvg::Path) -> Result<()> {
        if !p.is_visible() {
            return Ok(());
        }
        if p.fill().is_some_and(|f| f.opacity().get() != 1.)
            || p.stroke()
                .is_some_and(|s| s.opacity().get() != 1. || s.is_non_scaling())
        {
            return Err(invalid(
                "Drawing paths require opaque scaling strokes and fills",
            ));
        }
        self.content.push_str("q\n");
        if let Some(fill) = p.fill() {
            let c = color(fill.paint())?;
            writeln!(self.content, "{} {} {} rg", c[0], c[1], c[2]).unwrap();
        }
        if let Some(stroke) = p.stroke() {
            let c = color(stroke.paint())?;
            let cap = match stroke.linecap() {
                usvg::LineCap::Butt => 0,
                usvg::LineCap::Round => 1,
                usvg::LineCap::Square => 2,
            };
            let join = match stroke.linejoin() {
                usvg::LineJoin::Miter => 0,
                usvg::LineJoin::Round => 1,
                usvg::LineJoin::Bevel => 2,
                usvg::LineJoin::MiterClip => {
                    return Err(invalid("Miter-clip stroke is unsupported"));
                }
            };
            writeln!(
                self.content,
                "{} {} {} RG {} w {} J {} j {} M",
                c[0],
                c[1],
                c[2],
                stroke.width().get(),
                cap,
                join,
                stroke.miterlimit().get()
            )
            .unwrap();
            let dash = stroke
                .dasharray()
                .unwrap_or(&[])
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            writeln!(self.content, "[{dash}] {} d", stroke.dashoffset()).unwrap();
        }
        let mut data = String::new();
        let mut current = [0., 0.];
        let mut start = current;
        for segment in p.data().segments() {
            self.segments += 1;
            if self.segments > 1000000 {
                return Err(invalid("Drawing exceeds 1000000 vector segments"));
            }
            match segment {
                PathSegment::MoveTo(p) => {
                    current = [p.x, p.y];
                    start = current;
                    writeln!(data, "{} {} m", p.x, p.y).unwrap();
                }
                PathSegment::LineTo(p) => {
                    current = [p.x, p.y];
                    writeln!(data, "{} {} l", p.x, p.y).unwrap();
                }
                PathSegment::QuadTo(q, p) => {
                    let c1 = [
                        current[0] + (q.x - current[0]) * 2. / 3.,
                        current[1] + (q.y - current[1]) * 2. / 3.,
                    ];
                    let c2 = [p.x + (q.x - p.x) * 2. / 3., p.y + (q.y - p.y) * 2. / 3.];
                    writeln!(
                        data,
                        "{} {} {} {} {} {} c",
                        c1[0], c1[1], c2[0], c2[1], p.x, p.y
                    )
                    .unwrap();
                    current = [p.x, p.y];
                }
                PathSegment::CubicTo(a, b, p) => {
                    writeln!(data, "{} {} {} {} {} {} c", a.x, a.y, b.x, b.y, p.x, p.y).unwrap();
                    current = [p.x, p.y];
                }
                PathSegment::Close => {
                    data.push_str("h\n");
                    current = start;
                }
            }
        }
        let fill = p.fill().map(|f| {
            if f.rule() == usvg::FillRule::EvenOdd {
                "f*\n"
            } else {
                "f\n"
            }
        });
        let stroke = p.stroke().map(|_| "S\n");
        let operations = if p.paint_order() == usvg::PaintOrder::StrokeAndFill {
            [stroke, fill]
        } else {
            [fill, stroke]
        };
        for op in operations.into_iter().flatten() {
            self.content.push_str(&data);
            self.content.push_str(op);
        }
        self.content.push_str("Q\n");
        if self.content.len() > MAX_BYTES {
            return Err(invalid("Drawing page exceeds 32 MiB"));
        }
        Ok(())
    }
}
pub(crate) fn generate(sources: &[String]) -> Result<String> {
    if sources.is_empty()
        || sources.len() > 40
        || sources.iter().map(String::len).sum::<usize>() > 16 * 1024 * 1024
    {
        return Err(invalid("Expected 1–40 sheets within 16 MiB"));
    }
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = vec![0];
    fn object(pdf: &mut String, offsets: &mut Vec<usize>, body: String) {
        offsets.push(pdf.len());
        writeln!(pdf, "{} 0 obj\n{body}\nendobj", offsets.len() - 1).unwrap();
    }
    object(
        &mut pdf,
        &mut offsets,
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
    );
    object(
        &mut pdf,
        &mut offsets,
        format!(
            "<< /Type /Pages /Count {} /Kids [{}] >>",
            sources.len(),
            (0..sources.len())
                .map(|i| format!("{} 0 R", 3 + i * 2))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    );
    let font = ttf_parser::Face::parse(FONT, 0).map_err(|_| invalid("Invalid bundled font"))?;
    let mut glyphs = BTreeMap::new();
    let font_id = 3 + sources.len() * 2;
    for (i, source) in sources.iter().enumerate() {
        let tree = crate::svg::parse_tree(source, 72., None)?;
        let w = tree.size().width();
        let h = tree.size().height();
        if w > 2000. || h > 2000. {
            return Err(invalid("Drawing page dimensions exceed 2000 points"));
        }
        let mut writer = Writer {
            font: &font,
            fontdb: tree.fontdb(),
            glyphs: &mut glyphs,
            checked_fonts: Vec::new(),
            content: format!("1 0 0 -1 0 {h} cm\n"),
            nodes: 0,
            segments: 0,
        };
        writer.group(tree.root(), 0)?;
        object(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Resources << /Font << /F0 {font_id} 0 R >> >> /Contents {} 0 R >>",
                4 + i * 2
            ),
        );
        object(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Length {} >>\nstream\n{}endstream",
                writer.content.len(),
                writer.content
            ),
        );
        if pdf.len() > MAX_BYTES {
            return Err(invalid("Drawing PDF exceeds 32 MiB"));
        }
    }
    object(
        &mut pdf,
        &mut offsets,
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /NotoSans-Regular /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>",
            font_id + 1,
            font_id + 4
        ),
    );
    let upem = font.units_per_em() as f64;
    let widths = glyphs
        .iter()
        .map(|(id, (_, advance))| format!("{id} [{}]", *advance as f64 * 1000. / upem))
        .collect::<Vec<_>>()
        .join(" ");
    object(
        &mut pdf,
        &mut offsets,
        format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /NotoSans-Regular /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /CIDToGIDMap /Identity /FontDescriptor {} 0 R /DW 1000 /W [{widths}] >>",
            font_id + 2
        ),
    );
    let bb = font.global_bounding_box();
    object(
        &mut pdf,
        &mut offsets,
        format!(
            "<< /Type /FontDescriptor /FontName /NotoSans-Regular /Flags 32 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {} 0 R >>",
            bb.x_min as f64 * 1000. / upem,
            bb.y_min as f64 * 1000. / upem,
            bb.x_max as f64 * 1000. / upem,
            bb.y_max as f64 * 1000. / upem,
            font.ascender() as f64 * 1000. / upem,
            font.descender() as f64 * 1000. / upem,
            font.capital_height().unwrap_or(font.ascender()) as f64 * 1000. / upem,
            font_id + 3
        ),
    );
    let encoded = FONT.iter().map(|v| format!("{v:02X}")).collect::<String>() + ">";
    object(
        &mut pdf,
        &mut offsets,
        format!(
            "<< /Length {} /Length1 {} /Filter /ASCIIHexDecode >>\nstream\n{encoded}\nendstream",
            encoded.len(),
            FONT.len()
        ),
    );
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /NotoSansUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let entries = glyphs.iter().collect::<Vec<_>>();
    for chunk in entries.chunks(100) {
        writeln!(cmap, "{} beginbfchar", chunk.len()).unwrap();
        for (id, (text, _)) in chunk {
            writeln!(cmap, "<{id:04X}> <{}>", unicode_hex(text)).unwrap();
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    object(
        &mut pdf,
        &mut offsets,
        format!("<< /Length {} >>\nstream\n{cmap}endstream", cmap.len()),
    );
    if pdf.len() > MAX_BYTES {
        return Err(invalid("Drawing PDF exceeds 32 MiB"));
    }
    let start = pdf.len();
    writeln!(pdf, "xref\n0 {}\n0000000000 65535 f ", offsets.len()).unwrap();
    for offset in offsets.iter().skip(1) {
        writeln!(pdf, "{offset:010} 00000 n ").unwrap();
    }
    writeln!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF",
        offsets.len()
    )
    .unwrap();
    Ok(pdf)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vector_pages_keep_lines_and_unicode_glyphs() {
        let svg="<svg xmlns='http://www.w3.org/2000/svg' width='297mm' height='210mm' viewBox='0 0 297 210'><path d='M10 10 L20 20' fill='none' stroke='black'/><text x='10' y='30'>Деталь</text></svg>".to_string();
        let pdf = generate(&[svg.clone(), svg]).unwrap();
        assert!(pdf.contains("/Count 2"));
        assert!(pdf.contains(" l\n"));
        assert!(pdf.contains("/Subtype /Type0"));
        assert!(pdf.contains(" Tj"));
        assert!(pdf.contains("/FontFile2"));
        assert!(pdf.contains("0414"));
        assert!(!pdf.contains("/Subtype /Image"));
        let xref = pdf.split("xref\n").nth(1).unwrap();
        for (i, line) in xref.lines().skip(2).take(11).enumerate() {
            let offset = line[..10].parse::<usize>().unwrap();
            assert!(pdf[offset..].starts_with(&format!("{} 0 obj", i + 1)));
        }
    }
    #[test]
    fn refuses_images_and_empty_export() {
        assert!(generate(&[]).is_err());
        assert!(generate(&["<svg xmlns='http://www.w3.org/2000/svg'><rect width='1' height='1' opacity='.5'/></svg>".into()]).is_err());
    }
}
