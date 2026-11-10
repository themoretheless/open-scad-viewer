use crate::{
    ChangeKind, ChangeProvenance, ChangeSet, Edge, Face, Loop, Model, TopoId, TopoKind,
    TopologyChange, TopologyIds, TopologyLineageRecord, rational_identity,
};
use nurbs_core::{curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct FaceIdentityRegion {
    key: String,
    plane: Option<(usize, f64, bool)>,
    triangles: Vec<[[f64; 2]; 3]>,
    bounds: [[f64; 2]; 2],
}



#[path="model_identity/hash.rs"]
mod hash;
#[path="model_identity/prepare_face_identity.rs"]
mod prepare_face_identity;
