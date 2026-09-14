use crate::backend::{PrinterBackend, SubmitOutcome};
use crate::http::{
    find_number_field, find_string_field, multipart_form, HttpRequest, HttpResponse, HttpTransport,
};
use crate::job::{admit_remote_name, map_vendor_state, ArtifactKind, JobStatus, PrintJob, PrinterId};
use crate::scrub::scrub_secrets;
use crate::snapmaker::SnapmakerConfig;
use crate::{invalid, Result};

pub struct SnapmakerBackend<T> {
    pub config: SnapmakerConfig,
    pub transport: T,
    connected: bool,
}

impl<T: HttpTransport> SnapmakerBackend<T> {
    pub fn new(config: SnapmakerConfig, transport: T) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            transport,
            connected: false,
        })
    }

    #[cfg(feature = "network")]
    pub fn connect_live(
        config: SnapmakerConfig,
    ) -> Result<SnapmakerBackend<crate::http::live::UreqHttpTransport>> {
        config.validate()?;
        let mut http = crate::http::live::UreqHttpTransport::new(config.base_url.clone());
        http.timeout = config.timeout;
        http = http.with_secret(config.token.clone());
        let mut backend = SnapmakerBackend {
            config,
            transport: http,
            connected: false,
        };
        backend.ensure_connected()?;
        Ok(backend)
    }

    fn wrap(&self, code: &'static str, message: &str) -> crate::Error {
        invalid(
            code,
            &scrub_secrets(message, &[self.config.token.as_str()]),
        )
    }

    fn ensure_ok(&self, resp: &HttpResponse, context: &str) -> Result<()> {
        // 204 means connect pending confirmation on the controller.
        if resp.status == 204 {
            return Err(self.wrap(
                "PRINTER_AUTH",
                "Snapmaker connect pending controller confirmation (HTTP 204)",
            ));
        }
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

    fn token_query(&self) -> String {
        format!("token={}", encode_query(&self.config.token))
    }

    fn post_query(&mut self, path: &str) -> Result<HttpResponse> {
        let path = format!("{path}?{}", self.token_query());
        self.transport.request(&HttpRequest {
            method: "POST".into(),
            path,
            headers: Vec::new(),
            body: Vec::new(),
            content_type: None,
        })
    }

    fn get_query(&mut self, path: &str) -> Result<HttpResponse> {
        let path = format!("{path}?{}", self.token_query());
        self.transport.request(&HttpRequest {
            method: "GET".into(),
            path,
            headers: Vec::new(),
            body: Vec::new(),
            content_type: None,
        })
    }

    fn ensure_connected(&mut self) -> Result<()> {
        if self.connected {
            return Ok(());
        }
        let resp = self.post_query("/api/v1/connect")?;
        self.ensure_ok(&resp, "Snapmaker connect")?;
        self.connected = true;
        Ok(())
    }
}

fn encode_query(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl<T: HttpTransport> PrinterBackend for SnapmakerBackend<T> {
    fn id(&self) -> PrinterId {
        PrinterId {
            vendor: "snapmaker".into(),
            serial: self.config.printer_name.clone(),
        }
    }

    fn submit_job(&mut self, job: &PrintJob) -> Result<SubmitOutcome> {
        job.validate()?;
        if job.artifact.kind != ArtifactKind::Gcode {
            return Err(invalid(
                "PRINTER_ARTIFACT_KIND",
                "Snapmaker submit_job expects a plain .gcode artifact",
            ));
        }
        admit_remote_name(&job.artifact.file_name)?;
        self.ensure_connected()?;
        let fields = [(
            "file",
            Some(job.artifact.file_name.as_str()),
            job.artifact.bytes.as_slice(),
        )];
        let (content_type, body) = multipart_form(&fields);
        let path = format!("/api/v1/upload?{}", self.token_query());
        let resp = self.transport.request(&HttpRequest {
            method: "POST".into(),
            path,
            headers: Vec::new(),
            body,
            content_type: Some(content_type),
        })?;
        self.ensure_ok(&resp, "Snapmaker upload")?;
        if self.config.print_after_upload {
            let start = self.post_query("/api/v1/start_print")?;
            self.ensure_ok(&start, "Snapmaker start_print")?;
        }
        Ok(SubmitOutcome {
            remote_name: job.artifact.file_name.clone(),
            verified: self.config.print_after_upload,
            gcode_state: if self.config.print_after_upload {
                Some("RUNNING".into())
            } else {
                None
            },
        })
    }

    fn pause(&mut self) -> Result<()> {
        self.ensure_connected()?;
        let resp = self.post_query("/api/v1/pause")?;
        self.ensure_ok(&resp, "Snapmaker pause")
    }

    fn resume(&mut self) -> Result<()> {
        self.ensure_connected()?;
        let resp = self.post_query("/api/v1/resume")?;
        self.ensure_ok(&resp, "Snapmaker resume")
    }

    fn stop(&mut self) -> Result<()> {
        self.ensure_connected()?;
        let resp = self.post_query("/api/v1/stop")?;
        self.ensure_ok(&resp, "Snapmaker stop")
    }

    fn status(&mut self) -> Result<JobStatus> {
        self.ensure_connected()?;
        let resp = self.get_query("/api/v1/status")?;
        self.ensure_ok(&resp, "Snapmaker status")?;
        let raw = String::from_utf8_lossy(&resp.body).into_owned();
        let vendor_state = find_string_field(&raw, "status")
            .or_else(|| find_string_field(&raw, "printStatus"))
            .unwrap_or_default();
        let percent = find_number_field(&raw, "progress").and_then(|v| {
            let pct = if (0.0..=1.0).contains(&v) {
                v * 100.0
            } else {
                v
            };
            if (0.0..=100.0).contains(&pct) {
                Some(pct as u8)
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

/// Alias matching other backends' `connect` naming.
#[cfg(feature = "network")]
impl SnapmakerBackend<crate::http::live::UreqHttpTransport> {
    pub fn connect(config: SnapmakerConfig) -> Result<Self> {
        Self::connect_live(config)
    }
}
