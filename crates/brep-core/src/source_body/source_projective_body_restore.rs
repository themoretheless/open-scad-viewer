//! Restore a projectively parameterized body through fresh native gates.
pub use crate::source_body_restore::Limits;
use crate::{
    source_projective_edge::Edge, source_shell_geometry, source_shell_incidence, source_volume,
};
#[cfg(feature = "codec")]
use nurbs_core::{Error, Result};
#[cfg(feature = "codec")]
use value_codec::Value;

pub struct Report {
    pub incidence: source_shell_incidence::Report<Edge>,
    pub embedding: Option<source_shell_geometry::Report<Edge>>,
    pub volume: Option<source_volume::Report<Edge>>,
}
impl Report {
    pub fn body(&self) -> Option<&source_volume::Body<Edge>> {
        self.volume.as_ref()?.body.as_ref()
    }
    pub fn reason(&self) -> &'static str {
        self.volume
            .as_ref()
            .map(|r| r.reason)
            .or_else(|| self.embedding.as_ref().map(|r| r.reason))
            .unwrap_or(self.incidence.reason)
    }
}
/// Serialized bounds and admission flags never bypass original region,
/// incidence, injectivity, all-pair contact or signed-volume qualification.
#[cfg(feature = "codec")]
pub fn restore(value: Value, limits: Limits) -> Result<Report> {
    if value["version"].as_u64() != Some(1)
        || value["kind"].as_str() != Some("projective-body")
        || !value["inverseShear"].is_null()
    {
        return Err(Error::new(
            "BREP_PROJECTIVE_BODY_RESTORE",
            "Invalid original projective body definition",
        ));
    }
    let mut incidence =
        crate::source_projective_shell_restore::restore(value["shell"].clone(), &limits.shell)?;
    let shell = incidence.shell.take();
    let mut out = Report {
        incidence,
        embedding: None,
        volume: None,
    };
    let Some(shell) = shell else {
        return Ok(out);
    };
    let mut embedding = source_shell_geometry::qualify_projective(shell, limits.embedding)?;
    let geometry = embedding.geometry.take();
    out.embedding = Some(embedding);
    if let Some(geometry) = geometry {
        out.volume = Some(source_volume::qualify(geometry, limits.volume)?);
    }
    Ok(out)
}
