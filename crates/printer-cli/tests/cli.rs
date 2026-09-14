use printer_cli::infer_artifact_kind;
use printer_core::ArtifactKind;
use std::path::Path;

#[test]
fn clap_help_mentions_serve() {
    // Binary exists as a workspace member; artifact inference is the shared contract.
    assert_eq!(
        infer_artifact_kind(Path::new("x.gcode.3mf")).unwrap(),
        ArtifactKind::Gcode3mf
    );
    assert_eq!(
        infer_artifact_kind(Path::new("x.gcode")).unwrap(),
        ArtifactKind::Gcode
    );
}
