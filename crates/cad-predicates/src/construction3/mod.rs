//! Exact projective line/plane recipes over retained authored inputs in 3D.
//! Includes bounded segment/triangle queries; no trim topology or solid certificate.
use crate::arithmetic::{Expansion, Interval};
use crate::construction::{MAX_CACHED_TERMS, fraction, normalize};
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Decision, InputError, LeafRef,
    MAX_CONSTRUCTION_DEPTH, MAX_CONSTRUCTION_NODES, Outcome, PredicateContext, Reason, Sign, Stage,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

type HPoint = [Expansion; 4];

mod constructed_point3;
mod classify_line_plane_domains3d;
mod intersect_triangles3d;
pub use constructed_point3::*;
pub use classify_line_plane_domains3d::*;
pub use intersect_triangles3d::*;
