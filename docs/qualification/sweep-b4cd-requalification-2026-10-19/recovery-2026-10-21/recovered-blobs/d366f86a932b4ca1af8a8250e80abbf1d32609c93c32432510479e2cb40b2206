use printer_core::{
    ArtifactKind, BambuLanBackend, BambuLanConfig, MockTransport, PrintArtifact, PrintJob,
    PrinterBackend,
};

fn job_3mf(backend: &BambuLanBackend<MockTransport>, bytes: Vec<u8>) -> PrintJob {
    PrintJob {
        printer: backend.id(),
        artifact: PrintArtifact {
            kind: ArtifactKind::Gcode3mf,
            bytes,
            file_name: "box.gcode.3mf".into(),
        },
        plate_gcode_path: "Metadata/plate_1.gcode".into(),
        verify_start: false,
    }
}

#[test]
fn bambu_submit_uploads_then_publishes_project_file() {
    let config = BambuLanConfig::new("192.168.1.50", "12345678", "01P00A000000001");
    let mut client = BambuLanBackend::new(config, MockTransport::default()).unwrap();
    let bytes = b"fake-3mf".to_vec();
    let outcome = client.submit_job(&job_3mf(&client, bytes.clone())).unwrap();
    assert!(!outcome.verified);
    assert_eq!(client.transport.files.get("box.gcode.3mf"), Some(&bytes));
    let published = &client.transport.published[0];
    assert_eq!(published.topic, "device/01P00A000000001/request");
    assert!(published.payload.contains(r#""command":"project_file""#));
    assert!(
        published
            .payload
            .contains(r#""url":"ftp:///box.gcode.3mf""#)
    );
    assert!(published.payload.contains(r#""md5":""#) || published.payload.contains("md5"));
    let md5 = printer_core::bambu::artifact_md5(&bytes);
    assert!(published.payload.contains(&md5));
}

#[test]
fn rejects_plain_gcode_and_path_traversal() {
    let config = BambuLanConfig::new("printer.local", "abcd1234", "SERIAL1");
    let mut client = BambuLanBackend::new(config, MockTransport::default()).unwrap();
    let mut job = job_3mf(&client, b"G28\n".to_vec());
    job.artifact.kind = ArtifactKind::Gcode;
    job.artifact.file_name = "a.gcode".into();
    assert_eq!(
        client.submit_job(&job).unwrap_err().code,
        "PRINTER_ARTIFACT_KIND"
    );
    job.artifact.kind = ArtifactKind::Gcode3mf;
    job.artifact.file_name = "../x.gcode.3mf".into();
    assert_eq!(
        client.submit_job(&job).unwrap_err().code,
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
    let mut client = BambuLanBackend::new(config, transport).unwrap();
    let status = client.status().unwrap();
    assert_eq!(status.vendor_state, "IDLE");
    assert_eq!(status.percent, Some(0));
}

#[test]
fn verify_start_accepts_running() {
    let config = BambuLanConfig::new("10.0.0.3", "code0002", "S2");
    let mut transport = MockTransport::default();
    transport.push_report(
        r#"{"print":{"command":"push_status","msg":0,"gcode_state":"RUNNING","mc_percent":1,"layer_num":0,"print_error":0}}"#,
    );
    let mut client = BambuLanBackend::new(config, transport).unwrap();
    let mut job = job_3mf(&client, b"pk".to_vec());
    job.verify_start = true;
    let outcome = client.submit_job(&job).unwrap();
    assert!(outcome.verified);
    assert_eq!(outcome.gcode_state.as_deref(), Some("RUNNING"));
}

#[test]
fn verify_start_fails_on_print_error() {
    let config = BambuLanConfig::new("10.0.0.4", "code0003", "S3");
    let mut transport = MockTransport::default();
    transport.push_report(
        r#"{"print":{"command":"push_status","msg":0,"gcode_state":"IDLE","print_error":83935248}}"#,
    );
    let mut client = BambuLanBackend::new(config, transport).unwrap();
    let mut job = job_3mf(&client, b"pk".to_vec());
    job.verify_start = true;
    assert_eq!(
        client.submit_job(&job).unwrap_err().code,
        "PRINTER_START_FAILED"
    );
}
