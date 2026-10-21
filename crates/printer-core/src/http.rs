//! Minimal HTTP request seam for Moonraker / OctoPrint (mockable without sockets).

use crate::{Result, invalid};
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

pub trait HttpTransport {
    fn request(&mut self, req: &HttpRequest) -> Result<HttpResponse>;
}

#[derive(Clone, Debug, Default)]
pub struct MockHttpTransport {
    pub calls: Vec<HttpRequest>,
    pub responses: VecDeque<HttpResponse>,
}

impl MockHttpTransport {
    pub fn push_json(&mut self, status: u16, body: &str) {
        self.responses.push_back(HttpResponse {
            status,
            body: body.as_bytes().to_vec(),
        });
    }
}

impl HttpTransport for MockHttpTransport {
    fn request(&mut self, req: &HttpRequest) -> Result<HttpResponse> {
        self.calls.push(req.clone());
        self.responses
            .pop_front()
            .ok_or_else(|| invalid("PRINTER_HTTP", "Mock HTTP transport has no queued response"))
    }
}

/// Build a multipart/form-data body. Returns `(content_type, body)`.
pub fn multipart_form(fields: &[(&str, Option<&str>, &[u8])]) -> (String, Vec<u8>) {
    let boundary = "----printer-core-boundary7f3a";
    let cap: usize = 36
        + fields
            .iter()
            .map(|(name, file, data)| {
                120 + name.len() + file.map_or(0, str::len) + data.len()
            })
            .sum::<usize>();
    let mut body = Vec::with_capacity(cap);
    for (name, filename, data) in fields {
        body.extend_from_slice(b"--");
        body.extend_from_slice(boundary.as_bytes());
        body.extend_from_slice(b"\r\nContent-Disposition: form-data; name=\"");
        body.extend_from_slice(name.as_bytes());
        match filename {
            Some(file) => {
                body.extend_from_slice(b"\"; filename=\"");
                body.extend_from_slice(file.as_bytes());
                body.extend_from_slice(b"\"\r\nContent-Type: application/octet-stream\r\n\r\n");
            }
            None => {
                body.extend_from_slice(b"\"\r\n\r\n");
            }
        }
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");
    (format!("multipart/form-data; boundary={boundary}"), body)
}

/// Escape a string for embedding in a JSON string value.
pub fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn find_quoted_key(haystack: &str, key: &str) -> Option<usize> {
    let h = haystack.as_bytes();
    let k = key.as_bytes();
    let needed = k.len() + 2;
    if h.len() < needed {
        return None;
    }
    for i in 0..=h.len() - needed {
        if h[i] == b'"' && h[i + 1 + k.len()] == b'"' && &h[i + 1..i + 1 + k.len()] == k {
            return Some(i + needed);
        }
    }
    None
}

/// Find `"key":"value"` or `"key": number` in a flat-ish JSON object (best-effort).
pub fn find_string_field(json: &str, key: &str) -> Option<String> {
    let mut search = json;
    while let Some(start) = find_quoted_key(search, key) {
        let rest = search[start..].trim_start();
        let Some(rest) = rest.strip_prefix(':').map(str::trim_start) else {
            search = &search[start..];
            continue;
        };
        if let Some(rest) = rest.strip_prefix('"') {
            let end_quote = rest.find('"').unwrap_or(16);
            let mut out = String::with_capacity(end_quote);
            let mut chars = rest.chars();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => match chars.next() {
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some('n') => out.push('\n'),
                        Some('r') => out.push('\r'),
                        Some('t') => out.push('\t'),
                        Some(other) => out.push(other),
                        None => break,
                    },
                    '"' => return Some(out),
                    c => out.push(c),
                }
            }
            return None;
        }
        search = &search[start..];
    }
    None
}

pub fn find_number_field(json: &str, key: &str) -> Option<f64> {
    let start = find_quoted_key(json, key)?;
    let rest = json[start..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let end = rest
        .find(|c: char| {
            !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E')
        })
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

#[cfg(feature = "network")]
pub mod live {
    use super::{HttpRequest, HttpResponse, HttpTransport};
    use crate::scrub::scrub_secrets;
    use crate::{Result, invalid};
    use std::io::Read;
    use std::time::Duration;

    #[derive(Clone, Debug)]
    pub struct UreqHttpTransport {
        pub base_url: String,
        pub timeout: Duration,
        pub secrets: Vec<String>,
    }

    impl UreqHttpTransport {
        pub fn new(base_url: impl Into<String>) -> Self {
            Self {
                base_url: base_url.into().trim_end_matches('/').to_owned(),
                timeout: Duration::from_secs(30),
                secrets: Vec::new(),
            }
        }

        pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
            let s = secret.into();
            if !s.is_empty() {
                self.secrets.push(s);
            }
            self
        }
    }

    impl HttpTransport for UreqHttpTransport {
        fn request(&mut self, req: &HttpRequest) -> Result<HttpResponse> {
            let url = if req.path.starts_with("http://") || req.path.starts_with("https://") {
                req.path.clone()
            } else {
                format!("{}{}", self.base_url, req.path)
            };
            let agent = ureq::AgentBuilder::new()
                .timeout_connect(self.timeout)
                .timeout_read(self.timeout)
                .build();
            let mut builder = match req.method.as_str() {
                "GET" | "POST" | "PUT" | "DELETE" => agent.request(&req.method, &url),
                other => {
                    return Err(invalid(
                        "PRINTER_HTTP",
                        &format!("Unsupported HTTP method {other}"),
                    ));
                }
            };
            for (k, v) in &req.headers {
                builder = builder.set(k, v);
            }
            if let Some(ct) = &req.content_type {
                builder = builder.set("Content-Type", ct);
            }
            let result = if req.method == "GET" || (req.method == "DELETE" && req.body.is_empty()) {
                builder.call()
            } else {
                builder.send_bytes(&req.body)
            };
            let secrets: Vec<&str> = self.secrets.iter().map(String::as_str).collect();
            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let mut body = Vec::new();
                    resp.into_reader()
                        .take(64 * 1024 * 1024)
                        .read_to_end(&mut body)
                        .map_err(|e| {
                            invalid("PRINTER_HTTP", &scrub_secrets(&e.to_string(), &secrets))
                        })?;
                    Ok(HttpResponse { status, body })
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let mut body = Vec::new();
                    let _ = resp.into_reader().take(64 * 1024).read_to_end(&mut body);
                    Ok(HttpResponse { status: code, body })
                }
                Err(e) => Err(invalid(
                    "PRINTER_HTTP",
                    &scrub_secrets(&e.to_string(), &secrets),
                )),
            }
        }
    }
}
