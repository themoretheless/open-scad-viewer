//! STEP and IGES interchange for `brep-core` models: the constructor corpus
//! (`step_interchange`), AP242 topology v3–v10 (`step_interchange_v3`),
//! freeform NURBS faces and solids, IGES v2, close-topology complexes and the
//! original source-body carriers. Everything here reads `brep_core` through its
//! public API; the kernel does not depend back on this crate.
pub mod close_topology_interchange;
pub mod iges_interchange_v1;
pub mod iges_interchange_v2;
pub mod nurbs_step_interchange;
mod nurbs_step_shared;
pub mod nurbs_step_solid;
pub mod nurbs_step_trimmed;
pub mod source_exchange_step;
pub mod step_interchange;
pub mod step_interchange_v3;

pub use close_topology_interchange::{
    CLOSE_TOPOLOGY_IGES_CAPABILITY, CLOSE_TOPOLOGY_STEP_CAPABILITY, export_complex_iges,
    export_complex_step, import_complex_iges, import_complex_step,
};
pub use iges_interchange_v1::{export_iges, import_iges};
pub use iges_interchange_v2::{
    IGES_INTERCHANGE_V2_CAPABILITY, IgesV2Report, export_iges_v2, import_iges_v2,
};
pub use nurbs_step_interchange::{
    NURBS_STEP_BICUBIC_FACE_CAPABILITY, bicubic_open_face, export_nurbs_step, import_nurbs_step,
};
pub use nurbs_step_solid::{
    NURBS_STEP_SOLID_CAPABILITY, NURBS_STEP_SOLID_V2_CAPABILITY, export_nurbs_step_solid,
    export_nurbs_step_solid_v2, freeform_cuboid_solid, freeform_cuboid_with_bump_face,
    import_nurbs_step_solid, import_nurbs_step_solid_v2,
};
pub use nurbs_step_trimmed::{
    NURBS_STEP_TRIMMED_BICUBIC_CAPABILITY, bicubic_trimmed_face, export_nurbs_step_trimmed,
    import_nurbs_step_trimmed,
};
pub use step_interchange::{
    STEP_INTERCHANGE_V2_CAPABILITY, StepIdentityReport, export_step, export_step_v2, import_step,
    import_step_v2,
};
pub use step_interchange_v3::{
    STEP_INTERCHANGE_V3_CAPABILITY, STEP_INTERCHANGE_V4_CAPABILITY, STEP_INTERCHANGE_V5_CAPABILITY,
    STEP_INTERCHANGE_V6_CAPABILITY, STEP_INTERCHANGE_V7_CAPABILITY, STEP_INTERCHANGE_V8_CAPABILITY,
    STEP_INTERCHANGE_V9_CAPABILITY, STEP_INTERCHANGE_V10_CAPABILITY, StepRegularityEvidence,
    StepV3Report, StepV8Certificate, StepV10Document, compose_step_v7_occurrences,
    compose_step_v8_occurrences, compose_step_v9_occurrences, export_step_v3, export_step_v4,
    export_step_v5, export_step_v6, export_step_v7, export_step_v8, export_step_v9,
    export_step_v10, import_step_v3, import_step_v4, import_step_v5, import_step_v6,
    import_step_v7, import_step_v8, import_step_v9, import_step_v10,
};
pub(crate) use nurbs_core::curve::line;
