//! SVG import, CSS validation and silhouette request handlers.
//!
//! Routed by `geometry-bridge`: [`dispatch`] hands back operations this
//! domain does not own.

pub(crate) use bridge_codec::{Error, Result, Routed, Value};

pub mod svg;
pub mod svg_css;
pub mod svg_silhouette;

/// Handles the `svg` operation; other operations are handed back.
pub fn dispatch(v: Value) -> Routed {
    match bridge_codec::op(&v) {
        "svg" => Routed::Handled(svg::dispatch(v)),
        _ => Routed::Unhandled(v),
    }
}
