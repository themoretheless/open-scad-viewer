use printer_core::{
    ArtifactKind, CrealityBackend, CrealityConfig, MockHttpTransport, MoonrakerBackend,
    MoonrakerConfig, OctoPrintBackend, OctoPrintConfig, PrintArtifact, PrintJob, PrinterBackend,
    PrusaLinkBackend, PrusaLinkConfig, SnapmakerBackend, SnapmakerConfig,
};

fn gcode_job(serial: &str) -> PrintJob {
    PrintJob {
        printer: printer_core::PrinterId {
            vendor: "x".into(),
            serial: serial.into(),
        },
        artifact: PrintArtifact {
            kind: ArtifactKind::Gcode,
            bytes: b"G28\nG1 X10\n".to_vec(),
            file_name: "part.gcode".into(),
        },
        plate_gcode_path: String::new(),
        verify_start: true,
    }
}

#[test]
fn moonraker_upload_then_start() {
    let config = MoonrakerConfig::new("http://127.0.0.1:7125");
    let mut http = MockHttpTransport::default();
    http.push_json(
        200,
        r#"{"result":{"item":{"path":"part.gcode","root":"gcodes"}}}"#,
    );
    http.push_json(200, r#"{"result":"ok"}"#);
    let mut backend = MoonrakerBackend::new(config, http).unwrap();
    let mut job = gcode_job("moonraker");
    job.printer = backend.id();
    let outcome = backend.submit_job(&job).unwrap();
    assert!(outcome.verified);
    assert_eq!(outcome.remote_name, "part.gcode");
    assert_eq!(backend.transport.calls[0].path, "/server/files/upload");
    assert_eq!(backend.transport.calls[1].path, "/printer/print/start");
    assert!(
        std::str::from_utf8(&backend.transport.calls[1].body)
            .unwrap()
            .contains("part.gcode")
    );
}

#[test]
fn moonraker_rejects_3mf() {
    let config = MoonrakerConfig::new("http://printer:7125");
    let mut backend = MoonrakerBackend::new(config, MockHttpTransport::default()).unwrap();
    let mut job = gcode_job("moonraker");
    job.printer = backend.id();
    job.artifact.kind = ArtifactKind::Gcode3mf;
    job.artifact.file_name = "a.gcode.3mf".into();
    assert_eq!(
        backend.submit_job(&job).unwrap_err().code,
        "PRINTER_ARTIFACT_KIND"
    );
}

#[test]
fn octoprint_upload_select_print() {
    let config = OctoPrintConfig::new("http://127.0.0.1:5000", "APIKEY123");
    let mut http = MockHttpTransport::default();
    http.push_json(201, r#"{"files":{"local":{"name":"part.gcode"}}}"#);
    let mut backend = OctoPrintBackend::new(config, http).unwrap();
    let mut job = gcode_job("octoprint");
    job.printer = backend.id();
    let outcome = backend.submit_job(&job).unwrap();
    assert!(outcome.verified);
    assert_eq!(backend.transport.calls[0].path, "/api/files/local");
    let body = std::str::from_utf8(&backend.transport.calls[0].body).unwrap();
    assert!(body.contains("name=\"print\""));
    assert!(body.contains("true"));
}

#[test]
fn octoprint_pause_resume_cancel_and_status() {
    let config = OctoPrintConfig::new("http://octo.local", "SECRETKEY");
    let mut http = MockHttpTransport::default();
    http.push_json(204, "");
    http.push_json(204, "");
    http.push_json(204, "");
    http.push_json(
        200,
        r#"{"state":"Printing","progress":{"completion":42.5}}"#,
    );
    let mut backend = OctoPrintBackend::new(config, http).unwrap();
    backend.pause().unwrap();
    backend.resume().unwrap();
    backend.stop().unwrap();
    let status = backend.status().unwrap();
    assert_eq!(status.vendor_state, "Printing");
    assert_eq!(status.percent, Some(42));
    assert!(
        std::str::from_utf8(&backend.transport.calls[0].body)
            .unwrap()
            .contains(r#""action":"pause""#)
    );
}

#[test]
fn octoprint_scrubs_api_key_from_http_errors() {
    let config = OctoPrintConfig::new("http://octo.local", "SECRETKEY");
    let mut http = MockHttpTransport::default();
    http.push_json(401, "bad key SECRETKEY rejected");
    let mut backend = OctoPrintBackend::new(config, http).unwrap();
    let mut job = gcode_job("octoprint");
    job.printer = backend.id();
    let err = backend.submit_job(&job).unwrap_err();
    assert_eq!(err.code, "PRINTER_HTTP");
    assert!(!err.message.contains("SECRETKEY"));
    assert!(err.message.contains("[redacted]"));
}

#[test]
fn prusalink_put_upload_and_job_control() {
    let config = PrusaLinkConfig::new("http://192.168.1.40", "PRUSAKEY");
    let mut http = MockHttpTransport::default();
    http.push_json(201, "");
    http.push_json(200, r#"{"id":42,"state":"PRINTING"}"#);
    http.push_json(204, "");
    http.push_json(204, "");
    http.push_json(204, "");
    http.push_json(
        200,
        r#"{"printer":{"state":"PRINTING"},"job":{"id":42,"progress":0.25,"state":"PRINTING"}}"#,
    );
    let mut backend = PrusaLinkBackend::new(config, http).unwrap();
    let mut job = gcode_job("prusa");
    job.printer = backend.id();
    let outcome = backend.submit_job(&job).unwrap();
    assert!(outcome.verified);
    assert_eq!(backend.transport.calls[0].method, "PUT");
    assert_eq!(
        backend.transport.calls[0].path,
        "/api/v1/files/local/part.gcode"
    );
    assert!(
        backend.transport.calls[0]
            .headers
            .iter()
            .any(|(k, v)| k == "Print-After-Upload" && v == "?1")
    );
    backend.pause().unwrap();
    assert_eq!(backend.transport.calls[1].path, "/api/v1/job");
    assert_eq!(backend.transport.calls[2].path, "/api/v1/job/42/pause");
    backend.resume().unwrap();
    backend.stop().unwrap();
    let status = backend.status().unwrap();
    assert_eq!(status.percent, Some(25));
}

#[test]
fn creality_uses_moonraker_wire_with_creality_vendor() {
    let config = CrealityConfig::new("http://192.168.1.50:7125");
    let mut http = MockHttpTransport::default();
    http.push_json(
        200,
        r#"{"result":{"item":{"path":"part.gcode","root":"gcodes"}}}"#,
    );
    http.push_json(200, r#"{"result":"ok"}"#);
    let mut backend = CrealityBackend::new(config, http).unwrap();
    assert_eq!(backend.id().vendor, "creality");
    let mut job = gcode_job("creality");
    job.printer = backend.id();
    let outcome = backend.submit_job(&job).unwrap();
    assert!(outcome.verified);
    assert_eq!(
        backend.moonraker().transport.calls[0].path,
        "/server/files/upload"
    );
}

#[test]
fn snapmaker_connect_upload_start_and_control() {
    let config = SnapmakerConfig::new("http://192.168.1.60:8080", "SMTOKEN");
    let mut http = MockHttpTransport::default();
    // connect + upload + start_print
    http.push_json(200, r#"{"token":"SMTOKEN"}"#);
    http.push_json(200, r#"{"ok":true}"#);
    http.push_json(200, r#"{"ok":true}"#);
    // pause/resume/stop each reconnect? we cache connected, so only one connect
    http.push_json(200, "");
    http.push_json(200, "");
    http.push_json(200, "");
    http.push_json(
        200,
        r#"{"status":"RUNNING","printStatus":"RUNNING","progress":0.4}"#,
    );
    let mut backend = SnapmakerBackend::new(config, http).unwrap();
    let mut job = gcode_job("snapmaker");
    job.printer = backend.id();
    let outcome = backend.submit_job(&job).unwrap();
    assert!(outcome.verified);
    assert!(
        backend.transport.calls[0]
            .path
            .starts_with("/api/v1/connect?")
    );
    assert!(
        backend.transport.calls[1]
            .path
            .starts_with("/api/v1/upload?")
    );
    assert!(
        backend.transport.calls[2]
            .path
            .starts_with("/api/v1/start_print?")
    );
    assert!(
        std::str::from_utf8(&backend.transport.calls[1].body)
            .unwrap()
            .contains("part.gcode")
    );
    backend.pause().unwrap();
    backend.resume().unwrap();
    backend.stop().unwrap();
    let status = backend.status().unwrap();
    assert_eq!(status.percent, Some(40));
    assert_eq!(status.vendor_state, "RUNNING");
}

#[test]
fn snapmaker_scrubs_token() {
    let config = SnapmakerConfig::new("http://192.168.1.60:8080", "SMTOKEN99");
    let mut http = MockHttpTransport::default();
    http.push_json(401, "bad token SMTOKEN99");
    let mut backend = SnapmakerBackend::new(config, http).unwrap();
    let mut job = gcode_job("snapmaker");
    job.printer = backend.id();
    let err = backend.submit_job(&job).unwrap_err();
    assert!(!err.message.contains("SMTOKEN99"));
}
