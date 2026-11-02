//! Source addresses minted only after fresh exact boundary correspondence.
/// Authority of an original span binding. Image variants do not imply equal
/// normalized parameter clocks, derivative scales, or retained error bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialSpanBindingKind {
    NormalizedMap,
    OrientedLineImage,
    OrientedProjectiveImage,
}
/// One pair of exact Bezier spans in the complete oriented material boundary.
/// Indices always address original input curves, not rounded extracted curves.
/// A span number counts nonempty original knot intervals in ascending order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialSpanBinding {
    ring: usize,
    profile: (usize, usize),
    trim: (usize, usize),
    kind: MaterialSpanBindingKind,
}
impl MaterialSpanBinding {
    pub(in crate::sweep_boundary_owned::closed) fn new(
        ring: usize,
        profile: (usize, usize),
        trim: (usize, usize),
        kind: MaterialSpanBindingKind,
    ) -> Self {
        Self {
            ring,
            profile,
            trim,
            kind,
        }
    }
    pub fn ring(&self) -> usize {
        self.ring
    }
    /// Global source-profile index and its nonempty knot-span index.
    pub fn profile(&self) -> (usize, usize) {
        self.profile
    }
    /// Curve index within the original trim ring and its knot-span index.
    pub fn trim(&self) -> (usize, usize) {
        self.trim
    }
    pub fn kind(&self) -> MaterialSpanBindingKind {
        self.kind
    }
}
