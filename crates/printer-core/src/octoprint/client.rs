use crate::backend::{PrinterBackend, SubmitOutcome};
use crate::http::{
    HttpRequest, HttpResponse, HttpTransport, find_number_field, find_string_field, multipart_form,
};
use crate::job::{ArtifactKind, JobStatus, PrintJob, PrinterId, map_vendor_state};
use crate::octoprint::OctoPrintConfig;
use crate::scrub::scrub_secrets;
use crate::{Result, invalid};

pub struct OctoPrintBackend<T> {
    pub config: OctoPrintConfig,
    pub transport: T,
}

impl<T: HttpTransport> OctoPrintBackend<T> {
    pub fn new(config: OctoPrintConfig, transport: T) -> Result<Self> {
        config.validate()?;
        Ok(Self { config, transport })
    }

    #[cfg(feature = "network")]
    pub fn connect(
        config: OctoPrintConfig,
    ) -> Result<OctoPrintBackend<crate::http::live::UreqHttpTransport>> {
        config.validate()?;
        let mut http = crate::http::live::UreqHttpTransport::new(config.base_url.clone());
        http.timeout = config.timeout;
        http = http.with_secret(config.api_key.clone());
        Ok(OctoPrintBackend {
            config,
            transport: http,
        })
    }

    fn auth_headers(&self) -> Vec<(String, String)> {
        vec![("X-Api-Key".into(), self.config.api_key.clone())]
    }

    fn wrap(&self, code: &'static str, message: &str) -> crate::Error {
        invalid(
            code,
            &scrub_secrets(message, &[self.config.api_key.as_str()]),
        )
    }

    fn ensure_ok(&self, resp: &HttpResponse, context: &str) -> Result<()> {
        if (200..300).contains(&resp.status) {
            Ok(())
        } else {
            let body = String::from_utf8_lossy(&resp.body);
            Err(self.wrap(
                "PRINTER_HTTP",
                &format!("{context}: HTTP {} {body}", resp.status),
            ))
        }
    }

    fn post_json(&mut self, path: &str, body: &str) -> Result<HttpResponse> {
        let mut headers = self.auth_headers();
        headers.push(("Content-Type".into(), "application/json".into()));
        self.transport.request(&HttpRequest {
            method: "POST".into(),
            path: path.into(),
            headers,
            body: body.as_bytes().to_vec(),
            content_type: Some("application/json".into()),
        })
    }
}

impl<T: HttpTransport> PrinterBackend for OctoPrintBackend<T> {
    fn id(&self) -> PrinterId {
        PrinterId {
            vendor: "octoprint".into(),
            serial: self.config.printer_name.clone(),
        }
    }

    fn submit_job(&mut self, job: &PrintJob) -> Result<SubmitOutcome> {
        job.validate()?;
        if job.artifact.kind != ArtifactKind::Gcode {
            return Err(invalid(
                "PRINTER_ARTIFACT_KIND",
                "OctoPrint submit_job expects a plain .gcode artifact",
            ));
        }
        let fields = [
            (
                "file",
                Some(job.artifact.file_name.as_str()),
                job.artifact.bytes.as_slice(),
            ),
            ("select", None, b"true"),
            ("print", None, b"true"),
        ];
        let (content_type, body) = multipart_form(&fields);
        let resp = self.transport.request(&HttpRequest {
            method: "POST".into(),
            path: "/api/files/local".into(),
            headers: self.auth_headers(),
            body,
            content_type: Some(content_type),
        })?;
        self.ensure_ok(&resp, "OctoPrint upload")?;
        Ok(SubmitOutcome {
            remote_name: job.artifact.file_name.clone(),
            verified: true,
            gcode_state: Some("Printing".into()),
        })
    }

    fn pause(&mut self) -> Result<()> {
        let resp = self.post_json("/api/job", r#"{"command":"pause","action":"pause"}"#)?;
        self.ensure_ok(&resp, "OctoPrint pause")
    }

    fn resume(&mut self) -> Result<()> {
        let resp = self.post_json("/api/job", r#"{"command":"pause","action":"resume"}"#)?;
        self.ensure_ok(&resp, "OctoPrint resume")
    }

    fn stop(&mut self) -> Result<()> {
        let resp = self.post_json("/api/job", r#"{"command":"cancel"}"#)?;
        self.ensure_ok(&resp, "OctoPrint cancel")
    }

    fn status(&mut self) -> Result<JobStatus> {
        let resp = self.transport.request(&HttpRequest {
            method: "GET".into(),
            path: "/api/job".into(),
            headers: self.auth_headers(),
            body: Vec::new(),
            content_type: None,
        })?;
        self.ensure_ok(&resp, "OctoPrint status")?;
        let raw = String::from_utf8_lossy(&resp.body).into_owned();
        let vendor_state = find_string_field(&raw, "state").unwrap_or_default();
        let percent = find_number_field(&raw, "completion").and_then(|v| {
            if (0.0..=100.0).contains(&v) {
                Some(v as u8)
            } else {
                None
            }
        });
        Ok(JobStatus {
            state: map_vendor_state(&vendor_state),
            vendor_state,
            percent,
            layer: None,
            raw,
        })
    }
}
