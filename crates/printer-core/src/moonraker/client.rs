use crate::backend::{PrinterBackend, SubmitOutcome};
use crate::hash::sha256_hex;
use crate::http::{
    HttpRequest, HttpResponse, HttpTransport, escape_json, find_string_field, multipart_form,
};
use crate::job::{ArtifactKind, JobStatus, PrintJob, PrinterId, map_vendor_state};
use crate::moonraker::MoonrakerConfig;
use crate::scrub::scrub_secrets;
use crate::{Result, invalid};

pub struct MoonrakerBackend<T> {
    pub config: MoonrakerConfig,
    pub transport: T,
}

impl<T: HttpTransport> MoonrakerBackend<T> {
    pub fn new(config: MoonrakerConfig, transport: T) -> Result<Self> {
        config.validate()?;
        Ok(Self { config, transport })
    }

    #[cfg(feature = "network")]
    pub fn connect(
        config: MoonrakerConfig,
    ) -> Result<MoonrakerBackend<crate::http::live::UreqHttpTransport>> {
        config.validate()?;
        let mut http = crate::http::live::UreqHttpTransport::new(config.base_url.clone());
        http.timeout = config.timeout;
        if let Some(key) = &config.api_key {
            http = http.with_secret(key.clone());
        }
        Ok(MoonrakerBackend {
            config,
            transport: http,
        })
    }

    fn auth_headers(&self) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        if let Some(key) = &self.config.api_key {
            headers.push(("X-Api-Key".into(), key.clone()));
        }
        headers
    }

    fn secrets(&self) -> Vec<&str> {
        self.config.api_key.as_deref().into_iter().collect()
    }

    fn wrap(&self, code: &'static str, message: &str) -> crate::Error {
        invalid(code, &scrub_secrets(message, &self.secrets()))
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

    fn get(&mut self, path: &str) -> Result<HttpResponse> {
        self.transport.request(&HttpRequest {
            method: "GET".into(),
            path: path.into(),
            headers: self.auth_headers(),
            body: Vec::new(),
            content_type: None,
        })
    }
}

impl<T: HttpTransport> PrinterBackend for MoonrakerBackend<T> {
    fn id(&self) -> PrinterId {
        PrinterId {
            vendor: "moonraker".into(),
            serial: self.config.printer_name.clone(),
        }
    }

    fn submit_job(&mut self, job: &PrintJob) -> Result<SubmitOutcome> {
        job.validate()?;
        if job.artifact.kind != ArtifactKind::Gcode {
            return Err(invalid(
                "PRINTER_ARTIFACT_KIND",
                "Moonraker submit_job expects a plain .gcode artifact",
            ));
        }
        let checksum = if self.config.send_checksum {
            sha256_hex(&job.artifact.bytes)
        } else {
            String::new()
        };
        let mut fields: Vec<(&str, Option<&str>, &[u8])> = vec![
            (
                "file",
                Some(job.artifact.file_name.as_str()),
                job.artifact.bytes.as_slice(),
            ),
            ("root", None, self.config.root.as_bytes()),
        ];
        if !checksum.is_empty() {
            fields.push(("checksum", None, checksum.as_bytes()));
        }
        let (content_type, body) = multipart_form(&fields);
        let resp = self.transport.request(&HttpRequest {
            method: "POST".into(),
            path: "/server/files/upload".into(),
            headers: self.auth_headers(),
            body,
            content_type: Some(content_type),
        })?;
        self.ensure_ok(&resp, "Moonraker upload")?;
        let text = String::from_utf8_lossy(&resp.body);
        let remote = find_string_field(&text, "path")
            .or_else(|| find_string_field(&text, "filename"))
            .unwrap_or_else(|| job.artifact.file_name.clone());
        let start_body = format!(r#"{{"filename":"{}"}}"#, escape_json(&remote));
        let start = self.post_json("/printer/print/start", &start_body)?;
        self.ensure_ok(&start, "Moonraker print start")?;
        Ok(SubmitOutcome {
            remote_name: remote,
            verified: true,
            gcode_state: Some("printing".into()),
        })
    }

    fn pause(&mut self) -> Result<()> {
        let resp = self.post_json("/printer/print/pause", "{}")?;
        self.ensure_ok(&resp, "Moonraker pause")
    }

    fn resume(&mut self) -> Result<()> {
        let resp = self.post_json("/printer/print/resume", "{}")?;
        self.ensure_ok(&resp, "Moonraker resume")
    }

    fn stop(&mut self) -> Result<()> {
        let resp = self.post_json("/printer/print/cancel", "{}")?;
        self.ensure_ok(&resp, "Moonraker cancel")
    }

    fn status(&mut self) -> Result<JobStatus> {
        let resp = self.get("/printer/objects/query?print_stats")?;
        self.ensure_ok(&resp, "Moonraker status")?;
        let raw = String::from_utf8_lossy(&resp.body).into_owned();
        let vendor_state = find_string_field(&raw, "state").unwrap_or_default();
        Ok(JobStatus {
            state: map_vendor_state(&vendor_state),
            vendor_state,
            percent: None,
            layer: None,
            raw,
        })
    }
}
