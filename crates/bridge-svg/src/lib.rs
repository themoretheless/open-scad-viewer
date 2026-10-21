//! SVG import, CSS validation and silhouette request handlers.
//!
//! Routed by `geometry-bridge`: [`dispatch`] hands back operations this
//! domain does not own.

pub(crate) use bridge_codec::{Error, Result, Routed, Value};

mod drawing_pdf;
pub mod svg;
pub mod svg_css;
pub mod svg_silhouette;

/// Handles the `svg` operation; other operations are handed back.
pub fn dispatch(v: Value) -> Routed {
    match bridge_codec::op(&v) {
        "svg_color_trace" => Routed::Handled((|| {
            let tree=svg::parse_tree(&bridge_codec::field::<String>(&v,"svg")?,96.,None)?;
            svg_silhouette::color_contours(&tree,bridge_codec::field(&v,"widthMm")?,bridge_codec::field(&v,"heightMm")?,bridge_codec::field(&v,"resolution")?,bridge_codec::field(&v,"alpha")?,bridge_codec::field(&v,"colors")?,bridge_codec::field(&v,"ignoreWhite")?,bridge_codec::field(&v,"minArea")?)
        })()),
        "drawing_pdf"=>Routed::Handled((|| {
            let bytes=drawing_pdf::generate(&bridge_codec::field::<Vec<String>>(&v,"sheets")?)?;
            value_codec::to_value(bytes).map_err(|e|bridge_codec::input(e.to_string()))
        })()),
        "svg" => Routed::Handled(svg::dispatch(v)),
        _ => Routed::Unhandled(v),
    }
}
