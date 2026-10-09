//! STEP and IGES envelopes for audited topology complexes. The envelope
//! encoder lives in `brep_core::close_topology`; this module supplies the
//! manifold payload writers and the interchange capability names.
use brep_core::close_topology::{decode_complex, encode_complex, interchange_certificate};
use brep_core::{AuditedTopologyComplex, ComplexInterchangeCertificate};
use nurbs_core::Result;

pub const CLOSE_TOPOLOGY_STEP_CAPABILITY: &str = "close-topology-step/1";
pub const CLOSE_TOPOLOGY_IGES_CAPABILITY: &str = "close-topology-iges/1";

pub fn export_complex_step(
    audited: &AuditedTopologyComplex,
) -> Result<(String, ComplexInterchangeCertificate)> {
    let text = encode_complex(audited, "STEP", |model| {
        crate::export_step_v4(model).map(|v| v.0)
    })?;
    Ok((
        text,
        interchange_certificate(audited, CLOSE_TOPOLOGY_STEP_CAPABILITY),
    ))
}
pub fn import_complex_step(
    text: &str,
) -> Result<(AuditedTopologyComplex, ComplexInterchangeCertificate)> {
    let audited = decode_complex(text, "STEP", |payload| {
        crate::import_step_v4(payload).map(|v| v.0)
    })?;
    let certificate = interchange_certificate(&audited, CLOSE_TOPOLOGY_STEP_CAPABILITY);
    Ok((audited, certificate))
}
pub fn export_complex_iges(
    audited: &AuditedTopologyComplex,
) -> Result<(String, ComplexInterchangeCertificate)> {
    let text = encode_complex(audited, "IGES", |model| {
        crate::export_iges_v2(model).map(|v| v.0)
    })?;
    Ok((
        text,
        interchange_certificate(audited, CLOSE_TOPOLOGY_IGES_CAPABILITY),
    ))
}
pub fn import_complex_iges(
    text: &str,
) -> Result<(AuditedTopologyComplex, ComplexInterchangeCertificate)> {
    let audited = decode_complex(text, "IGES", |payload| {
        crate::import_iges_v2(payload).map(|v| v.0)
    })?;
    let certificate = interchange_certificate(&audited, CLOSE_TOPOLOGY_IGES_CAPABILITY);
    Ok((audited, certificate))
}
