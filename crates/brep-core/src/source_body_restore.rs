//! Restore a native body only by replaying all original geometry gates.
use crate::{linear_canal_body, source_shell_geometry, source_shell_restore, source_volume};
use value_codec::Value;
pub struct Limits {
    pub shell: source_shell_restore::Limits,
    pub embedding: source_shell_geometry::Limits,
    pub volume: source_volume::Limits,
}
pub fn restore(value: Value, limits: Limits) -> crate::Result<linear_canal_body::Report> {
    if value["version"].as_u64() != Some(1) {
        return Err(crate::Error::new(
            "BREP_SOURCE_BODY_RESTORE",
            "Unsupported source body version",
        ));
    }
    let incidence = source_shell_restore::restore(value["shell"].clone(), &limits.shell)
        .map_err(|e| crate::Error::new(e.code, e.message))?;
    linear_canal_body::from_incidence(incidence, limits.embedding, limits.volume)
}
