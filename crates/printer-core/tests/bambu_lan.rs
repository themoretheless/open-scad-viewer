use printer_core::{
    ArtifactKind, BambuLanClient, BambuLanConfig, MockTransport, PrintArtifact, PrintJob,
};

#[test]
fn bambu_submit_uploads_then_publishes_project_file() {
    let config = BambuLanConfig::new("192.168.1.50", "12345678", "01P00A000000001");
    let mut client = BambuLanClient::new(config, MockTransport::default()).unwrap();
    let bytes = b"fake-3mf".to_vec();
    let job = PrintJob {
        printer: client.printer_id(),
        artifact: PrintArtifact {
            kind: ArtifactKind::Gcode3mf,
            bytes: bytes.clone(),
            file_name: "box.gcode.3mf".into(),
        },
        plate_gcode_path: "Metadata/plate_1.gcode".into(),
    };
    client.submit_job(&job).unwrap();
    assert_eq!(client.transport.files.get("box.gcode.3mf"), Some(&bytes));
    let published = &client.transport.published[0];
    assert_eq!(published.topic, "device/01P00A000000001/request");
    assert!(published.payload.contains(r#""command":"project_file""#));
    assert!(published.payload.contains(r#""url":"ftp:///box.gcode.3mf""#));
}

#[test]
fn rejects_plain_gcode_and_path_traversal() {
    let config = BambuLanConfig::new("printer.local", "abcd1234", "SERIAL1");
    let mut client = BambuLanClient::new(config, MockTransport::default()).unwrap();
    let job = PrintJob {
        printer: client.printer_id(),
        artifact: PrintArtifact {
            kind: ArtifactKind::Gcode,
            bytes: b"G28\n".to_vec(),
            file_name: "a.gcode".into(),
        },
        plate_gcode_path: "Metadata/plate_1.gcode".into(),
    };
    assert_eq!(
        client.submit_job(&job).unwrap_err().code,
        "PRINTER_ARTIFACT_KIND"
    );
    let mut bad = job;
    bad.artifact.kind = ArtifactKind::Gcode3mf;
    bad.artifact.file_name = "../x.gcode.3mf".into();
    assert_eq!(
        client.submit_job(&bad).unwrap_err().code,
        "PRINTER_ARTIFACT_NAME"
    );
}

#[test]
fn push_status_reads_mock_report() {
    let config = BambuLanConfig::new("10.0.0.2", "code0001", "S1");
    let transport = MockTransport {
        next_report: r#"{"print":{"command":"push_status","msg":0,"gcode_state":"IDLE","mc_percent":0,"layer_num":0}}"#
            .into(),
        ..MockTransport::default()
    };
    let mut client = BambuLanClient::new(config, transport).unwrap();
    let status = client.push_status().unwrap();
    assert_eq!(status.gcode_state, "IDLE");
    assert_eq!(status.percent, Some(0));
}
