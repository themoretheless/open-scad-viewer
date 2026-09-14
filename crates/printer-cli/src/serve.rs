use crate::artifact::PrintArtifactBytes;
use crate::connect::{
    control, discover, send_bytes, ConnectionArgs, ControlAction, VendorKind,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use printer_core::{ArtifactKind, DiscoveryVendor, scrub_secrets};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Cursor;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

#[derive(Clone, Debug)]
pub struct ServeOptions {
    pub port: u16,
    pub discover_timeout: Duration,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            port: 17890,
            discover_timeout: Duration::from_secs(3),
        }
    }
}

#[derive(Deserialize)]
struct SendBody {
    vendor: String,
    config: ConfigBody,
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "bytesBase64")]
    bytes_base64: String,
}

#[derive(Deserialize)]
struct ControlBody {
    vendor: String,
    config: ConfigBody,
    action: String,
}

#[derive(Deserialize, Default)]
struct ConfigBody {
    #[serde(default)]
    host: String,
    #[serde(default)]
    access_code: Option<String>,
    #[serde(default)]
    serial: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    plate_gcode_path: Option<String>,
    #[serde(default)]
    verify_start: Option<bool>,
}

impl ConfigBody {
    fn into_args(self) -> ConnectionArgs {
        ConnectionArgs {
            host: self.host,
            access_code: self.access_code,
            serial: self.serial,
            api_key: self.api_key,
            token: self.token,
            plate_gcode_path: self
                .plate_gcode_path
                .unwrap_or_else(|| "Metadata/plate_1.gcode".into()),
            verify_start: self.verify_start.unwrap_or(true),
        }
    }
}

#[derive(Serialize)]
struct DiscoveredJson {
    vendor: String,
    display_name: Option<String>,
    host: String,
    port: Option<u16>,
    serial: Option<String>,
    model: Option<String>,
}

/// Bind `127.0.0.1` only and serve the Vue companion JSON API.
pub fn run_serve(opts: ServeOptions) -> Result<(), String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], opts.port));
    let server = Server::http(addr).map_err(|e| format!("bind {addr}: {e}"))?;
    eprintln!("printer-cli serve listening on http://{addr}");
    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_owned();
        let path = url.split('?').next().unwrap_or(&url);
        let result = match (method, path) {
            (Method::Get, "/health") => Ok(json!({ "ok": true })),
            (Method::Get, "/discover") => handle_discover(opts.discover_timeout),
            (Method::Post, "/send") => handle_send(&mut request),
            (Method::Post, "/control") => handle_control(&mut request),
            (Method::Options, _) => Ok(json!({ "ok": true })),
            _ => Err((404, "not found".into())),
        };
        respond(request, result);
    }
    Ok(())
}

fn handle_discover(timeout: Duration) -> std::result::Result<serde_json::Value, (u16, String)> {
    let found = discover(timeout, &[DiscoveryVendor::Bambu, DiscoveryVendor::Snapmaker])
        .map_err(|e| (502, e.message))?;
    let printers: Vec<_> = found
        .into_iter()
        .map(|p| DiscoveredJson {
            vendor: p.vendor,
            display_name: p.display_name,
            host: p.host,
            port: p.port,
            serial: p.serial,
            model: p.model,
        })
        .collect();
    Ok(json!({ "printers": printers }))
}

fn handle_send(request: &mut Request) -> std::result::Result<serde_json::Value, (u16, String)> {
    let body = read_json::<SendBody>(request)?;
    let vendor = VendorKind::from_str(&body.vendor).map_err(|e| (400, e))?;
    let args = body.config.into_args();
    let bytes = B64
        .decode(body.bytes_base64.as_bytes())
        .map_err(|e| (400, format!("invalid bytesBase64: {e}")))?;
    let kind = if body.file_name.to_ascii_lowercase().ends_with(".gcode.3mf")
        || body.file_name.to_ascii_lowercase().ends_with(".3mf")
    {
        ArtifactKind::Gcode3mf
    } else {
        ArtifactKind::Gcode
    };
    let art = PrintArtifactBytes {
        kind,
        bytes,
        file_name: body.file_name,
    };
    let outcome = send_bytes(vendor, &args, &art).map_err(|e| {
        (
            502,
            scrub_secrets(&e.message, &args.secrets()),
        )
    })?;
    Ok(json!({
        "remoteName": outcome.remote_name,
        "verified": outcome.verified,
        "gcodeState": outcome.gcode_state,
    }))
}

fn handle_control(request: &mut Request) -> std::result::Result<serde_json::Value, (u16, String)> {
    let body = read_json::<ControlBody>(request)?;
    let vendor = VendorKind::from_str(&body.vendor).map_err(|e| (400, e))?;
    let action = ControlAction::from_str(&body.action).map_err(|e| (400, e))?;
    let args = body.config.into_args();
    let status = control(vendor, &args, action).map_err(|e| {
        (
            502,
            scrub_secrets(&e.message, &args.secrets()),
        )
    })?;
    match status {
        Some(s) => Ok(json!({
            "state": format!("{:?}", s.state).to_ascii_lowercase(),
            "vendorState": s.vendor_state,
            "percent": s.percent,
            "layer": s.layer,
        })),
        None => Ok(json!({ "ok": true })),
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(
    request: &mut Request,
) -> std::result::Result<T, (u16, String)> {
    let mut buf = Vec::new();
    request
        .as_reader()
        .read_to_end(&mut buf)
        .map_err(|e| (400, format!("read body: {e}")))?;
    serde_json::from_slice(&buf).map_err(|e| (400, format!("invalid JSON: {e}")))
}

fn respond(request: Request, result: std::result::Result<serde_json::Value, (u16, String)>) {
    let cors = Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap();
    let allow_headers =
        Header::from_bytes("Access-Control-Allow-Headers", "Content-Type").unwrap();
    let allow_methods =
        Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").unwrap();
    let (status, body) = match result {
        Ok(value) => (200u16, value.to_string()),
        Err((code, message)) => (code, json!({ "error": message }).to_string()),
    };
    let response = Response::new(
        StatusCode(status),
        vec![
            cors,
            allow_headers,
            allow_methods,
            Header::from_bytes("Content-Type", "application/json; charset=utf-8").unwrap(),
        ],
        Cursor::new(body.clone().into_bytes()),
        Some(body.len()),
        None,
    );
    let _ = request.respond(response);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_body_maps_defaults() {
        let args = ConfigBody {
            host: "192.168.1.1".into(),
            ..Default::default()
        }
        .into_args();
        assert_eq!(args.host, "192.168.1.1");
        assert!(args.verify_start);
        assert_eq!(args.plate_gcode_path, "Metadata/plate_1.gcode");
    }
}
