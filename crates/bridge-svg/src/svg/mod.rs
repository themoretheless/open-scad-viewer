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
mod non_scaling_tests;

mod invalid;
mod preflight_inner;
mod legacy_source;
mod region;
#[cfg(test)]
mod tests;
use invalid::*;
use preflight_inner::*;
pub(crate) use legacy_source::*;
pub(crate) use region::*;
