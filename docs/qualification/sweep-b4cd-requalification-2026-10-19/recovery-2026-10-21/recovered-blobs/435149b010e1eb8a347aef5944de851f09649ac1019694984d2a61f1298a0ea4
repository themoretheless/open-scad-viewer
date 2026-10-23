use crate::backend::{PrinterBackend, SubmitOutcome};
use crate::http::{HttpRequest, HttpResponse, HttpTransport, find_number_field, find_string_field};
use crate::job::{
    ArtifactKind, JobStatus, PrintJob, PrinterId, admit_remote_name, map_vendor_state,
};
use crate::prusa::PrusaLinkConfig;
use crate::scrub::scrub_secrets;
use crate::{Result, invalid};

pub struct PrusaLinkBackend<T> {
    pub config: PrusaLinkConfig,
    pub transport: T,
    last_job_id: Option<u64>,
}

impl<T: HttpTransport> PrusaLinkBackend<T> {
    pub fn new(config: PrusaLinkConfig, transport: T) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            transport,
            last_job_id: None,
        })
    }

    #[cfg(feature = "network")]
    pub fn connect(
        config: PrusaLinkConfig,
    ) -> Result<PrusaLinkBackend<crate::http::live::UreqHttpTransport>> {
        config.validate()?;
        let mut http = crate::http::live::UreqHttpTransport::new(config.base_url.clone());
        http.timeout = config.timeout;
        http = http.with_secret(config.api_key.clone());
        Ok(PrusaLinkBackend {
            config,
            transport: http,
            last_job_id: None,
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

    fn get(&mut self, path: &str) -> Result<HttpResponse> {
        self.transport.request(&HttpRequest {
            method: "GET".into(),
            path: path.into(),
            headers: self.auth_headers(),
            body: Vec::new(),
            content_type: None,
        })
    }

    fn put_bytes(
        &mut self,
        path: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<HttpResponse> {
        self.transport.request(&HttpRequest {
            method: "PUT".into(),
            path: path.into(),
            headers,
            body,
            content_type: Some("application/octet-stream".into()),
        })
    }

    fn put_empty(&mut self, path: &str) -> Result<HttpResponse> {
        self.transport.request(&HttpRequest {
            method: "PUT".into(),
            path: path.into(),
            headers: self.auth_headers(),
            body: Vec::new(),
            content_type: None,
        })
    }

    fn delete(&mut self, path: &str) -> Result<HttpResponse> {
        self.transport.request(&HttpRequest {
            method: "DELETE".into(),
            path: path.into(),
            headers: self.auth_headers(),
            body: Vec::new(),
            content_type: None,
        })
    }

    fn resolve_job_id(&mut self) -> Result<u64> {
        if let Some(id) = self.last_job_id {
            return Ok(id);
        }
        let resp = self.get("/api/v1/job")?;
        if resp.status == 204 {
            return Err(invalid("PRINTER_JOB", "PrusaLink has no active job"));
        }
        self.ensure_ok(&resp, "PrusaLink job")?;
        let raw = String::from_utf8_lossy(&resp.body);
        let id = find_number_field(&raw, "id")
            .map(|v| v as u64)
            .ok_or_else(|| invalid("PRINTER_JOB", "PrusaLink job response missing id"))?;
        self.last_job_id = Some(id);
        Ok(id)
    }
}

/// Percent-encode a single path segment (file name).
fn encode_path_segment(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for b in name.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl<T: HttpTransport> PrinterBackend for PrusaLinkBackend<T> {
    fn id(&self) -> PrinterId {
        PrinterId {
            vendor: "prusa".into(),
            serial: self.config.printer_name.clone(),
        }
    }

    fn submit_job(&mut self, job: &PrintJob) -> Result<SubmitOutcome> {
        job.validate()?;
        if job.artifact.kind != ArtifactKind::Gcode {
            return Err(invalid(
                "PRINTER_ARTIFACT_KIND",
                "PrusaLink submit_job expects a plain .gcode artifact",
            ));
        }
        admit_remote_name(&job.artifact.file_name)?;
        let path = format!(
            "/api/v1/files/{}/{}",
            encode_path_segment(&self.config.storage),
            encode_path_segment(&job.artifact.file_name)
        );
        let mut headers = self.auth_headers();
        headers.push((
            "Print-After-Upload".into(),
            if self.config.print_after_upload {
                "?1".into()
            } else {
                "?0".into()
            },
        ));
        headers.push((
            "Overwrite".into(),
            if self.config.overwrite {
                "?1".into()
            } else {
                "?0".into()
            },
        ));
        headers.push((
            "Content-Length".into(),
            job.artifact.bytes.len().to_string(),
        ));
        let resp = self.put_bytes(&path, headers, job.artifact.bytes.clone())?;
        self.ensure_ok(&resp, "PrusaLink upload")?;
        self.last_job_id = None;
        Ok(SubmitOutcome {
            remote_name: job.artifact.file_name.clone(),
            verified: self.config.print_after_upload,
            gcode_state: if self.config.print_after_upload {
                Some("PRINTING".into())
            } else {
                None
            },
        })
    }

    fn pause(&mut self) -> Result<()> {
        let id = self.resolve_job_id()?;
        let resp = self.put_empty(&format!("/api/v1/job/{id}/pause"))?;
        self.ensure_ok(&resp, "PrusaLink pause")
    }

    fn resume(&mut self) -> Result<()> {
        let id = self.resolve_job_id()?;
        let resp = self.put_empty(&format!("/api/v1/job/{id}/resume"))?;
        self.ensure_ok(&resp, "PrusaLink resume")
    }

    fn stop(&mut self) -> Result<()> {
        let id = self.resolve_job_id()?;
        let resp = self.delete(&format!("/api/v1/job/{id}"))?;
        self.ensure_ok(&resp, "PrusaLink stop")?;
        self.last_job_id = None;
        Ok(())
    }

    fn status(&mut self) -> Result<JobStatus> {
        let resp = self.get("/api/v1/status")?;
        self.ensure_ok(&resp, "PrusaLink status")?;
        let raw = String::from_utf8_lossy(&resp.body).into_owned();
        if let Some(id) = find_number_field(&raw, "id") {
            self.last_job_id = Some(id as u64);
        }
        let vendor_state = find_string_field(&raw, "state")
            .or_else(|| find_string_field(&raw, "status"))
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
