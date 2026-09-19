use crate::artifact::{PrintArtifactBytes, load_artifact};
use printer_core::{
    ArtifactKind, BambuLanBackend, BambuLanConfig, BambuLanTransport, CrealityBackend,
    CrealityConfig, DiscoveredPrinter, DiscoveryOptions, DiscoveryVendor, JobStatus,
    MoonrakerBackend, MoonrakerConfig, OctoPrintBackend, OctoPrintConfig, PrintArtifact, PrintJob,
    PrinterBackend, PrinterDiscovery, PrusaLinkBackend, PrusaLinkConfig, Result, SnapmakerBackend,
    SnapmakerConfig, SubmitOutcome, UreqHttpTransport, default_live_discovery, scrub_secrets,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VendorKind {
    Bambu,
    Moonraker,
    OctoPrint,
    Prusa,
    Creality,
    Snapmaker,
}

impl FromStr for VendorKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "bambu" => Ok(Self::Bambu),
            "moonraker" => Ok(Self::Moonraker),
            "octoprint" => Ok(Self::OctoPrint),
            "prusa" | "prusalink" => Ok(Self::Prusa),
            "creality" => Ok(Self::Creality),
            "snapmaker" => Ok(Self::Snapmaker),
            other => Err(format!("unknown vendor '{other}'")),
        }
    }
}

impl VendorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bambu => "bambu",
            Self::Moonraker => "moonraker",
            Self::OctoPrint => "octoprint",
            Self::Prusa => "prusa",
            Self::Creality => "creality",
            Self::Snapmaker => "snapmaker",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ConnectionArgs {
    pub host: String,
    pub access_code: Option<String>,
    pub serial: Option<String>,
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub plate_gcode_path: String,
    pub verify_start: bool,
}

impl ConnectionArgs {
    pub fn secrets(&self) -> Vec<&str> {
        [
            self.access_code.as_deref(),
            self.api_key.as_deref(),
            self.token.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    Status,
    Pause,
    Resume,
    Stop,
}

impl FromStr for ControlAction {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "status" => Ok(Self::Status),
            "pause" => Ok(Self::Pause),
            "resume" => Ok(Self::Resume),
            "stop" => Ok(Self::Stop),
            other => Err(format!("unknown action '{other}'")),
        }
    }
}

pub fn discover(timeout: Duration, vendors: &[DiscoveryVendor]) -> Result<Vec<DiscoveredPrinter>> {
    let mut disc = default_live_discovery();
    let opts = DiscoveryOptions {
        timeout,
        vendors: if vendors.is_empty() {
            DiscoveryOptions::default().vendors
        } else {
            vendors.to_vec()
        },
    };
    disc.discover(&opts)
}

pub fn send_path(vendor: VendorKind, args: &ConnectionArgs, path: &Path) -> Result<SubmitOutcome> {
    let art = load_artifact(path)?;
    send_bytes(vendor, args, &art)
}

pub fn send_bytes(
    vendor: VendorKind,
    args: &ConnectionArgs,
    art: &PrintArtifactBytes,
) -> Result<SubmitOutcome> {
    match vendor {
        VendorKind::Bambu => {
            if art.kind != ArtifactKind::Gcode3mf {
                return Err(err(
                    "PRINTER_ARTIFACT",
                    "Bambu send expects a .gcode.3mf artifact",
                    args,
                ));
            }
            let config = bambu_config(args)?;
            let mut backend = BambuLanBackend::<BambuLanTransport>::connect_lan(config)
                .map_err(|e| map_err(e, args))?;
            let job = PrintJob {
                printer: backend.id(),
                artifact: PrintArtifact {
                    kind: art.kind,
                    bytes: art.bytes.clone(),
                    file_name: art.file_name.clone(),
                },
                plate_gcode_path: if args.plate_gcode_path.is_empty() {
                    "Metadata/plate_1.gcode".into()
                } else {
                    args.plate_gcode_path.clone()
                },
                verify_start: args.verify_start,
            };
            backend.submit_job(&job).map_err(|e| map_err(e, args))
        }
        VendorKind::Moonraker => {
            ensure_gcode(art, args)?;
            let config = moonraker_config(args)?;
            let mut backend = MoonrakerBackend::<UreqHttpTransport>::connect(config)
                .map_err(|e| map_err(e, args))?;
            submit_http(&mut backend, art, args)
        }
        VendorKind::OctoPrint => {
            ensure_gcode(art, args)?;
            let config = octoprint_config(args)?;
            let mut backend = OctoPrintBackend::<UreqHttpTransport>::connect(config)
                .map_err(|e| map_err(e, args))?;
            submit_http(&mut backend, art, args)
        }
        VendorKind::Prusa => {
            ensure_gcode(art, args)?;
            let config = prusa_config(args)?;
            let mut backend = PrusaLinkBackend::<UreqHttpTransport>::connect(config)
                .map_err(|e| map_err(e, args))?;
            submit_http(&mut backend, art, args)
        }
        VendorKind::Creality => {
            ensure_gcode(art, args)?;
            let config = creality_config(args)?;
            let mut backend = CrealityBackend::<UreqHttpTransport>::connect(config)
                .map_err(|e| map_err(e, args))?;
            submit_http(&mut backend, art, args)
        }
        VendorKind::Snapmaker => {
            ensure_gcode(art, args)?;
            let config = snapmaker_config(args)?;
            let mut backend = SnapmakerBackend::<UreqHttpTransport>::connect_live(config)
                .map_err(|e| map_err(e, args))?;
            submit_http(&mut backend, art, args)
        }
    }
}

pub fn control(
    vendor: VendorKind,
    args: &ConnectionArgs,
    action: ControlAction,
) -> Result<Option<JobStatus>> {
    match vendor {
        VendorKind::Bambu => {
            let config = bambu_config(args)?;
            let mut backend = BambuLanBackend::<BambuLanTransport>::connect_lan(config)
                .map_err(|e| map_err(e, args))?;
            run_control(&mut backend, action, args)
        }
        VendorKind::Moonraker => {
            let config = moonraker_config(args)?;
            let mut backend = MoonrakerBackend::<UreqHttpTransport>::connect(config)
                .map_err(|e| map_err(e, args))?;
            run_control(&mut backend, action, args)
        }
        VendorKind::OctoPrint => {
            let config = octoprint_config(args)?;
            let mut backend = OctoPrintBackend::<UreqHttpTransport>::connect(config)
                .map_err(|e| map_err(e, args))?;
            run_control(&mut backend, action, args)
        }
        VendorKind::Prusa => {
            let config = prusa_config(args)?;
            let mut backend = PrusaLinkBackend::<UreqHttpTransport>::connect(config)
                .map_err(|e| map_err(e, args))?;
            run_control(&mut backend, action, args)
        }
        VendorKind::Creality => {
            let config = creality_config(args)?;
            let mut backend = CrealityBackend::<UreqHttpTransport>::connect(config)
                .map_err(|e| map_err(e, args))?;
            run_control(&mut backend, action, args)
        }
        VendorKind::Snapmaker => {
            let config = snapmaker_config(args)?;
            let mut backend = SnapmakerBackend::<UreqHttpTransport>::connect_live(config)
                .map_err(|e| map_err(e, args))?;
            run_control(&mut backend, action, args)
        }
    }
}

fn run_control<B: PrinterBackend>(
    backend: &mut B,
    action: ControlAction,
    args: &ConnectionArgs,
) -> Result<Option<JobStatus>> {
    match action {
        ControlAction::Status => Ok(Some(backend.status().map_err(|e| map_err(e, args))?)),
        ControlAction::Pause => {
            backend.pause().map_err(|e| map_err(e, args))?;
            Ok(None)
        }
        ControlAction::Resume => {
            backend.resume().map_err(|e| map_err(e, args))?;
            Ok(None)
        }
        ControlAction::Stop => {
            backend.stop().map_err(|e| map_err(e, args))?;
            Ok(None)
        }
    }
}

fn submit_http<B: PrinterBackend>(
    backend: &mut B,
    art: &PrintArtifactBytes,
    args: &ConnectionArgs,
) -> Result<SubmitOutcome> {
    let job = PrintJob {
        printer: backend.id(),
        artifact: PrintArtifact {
            kind: art.kind,
            bytes: art.bytes.clone(),
            file_name: art.file_name.clone(),
        },
        plate_gcode_path: String::new(),
        verify_start: false,
    };
    backend.submit_job(&job).map_err(|e| map_err(e, args))
}

fn ensure_gcode(art: &PrintArtifactBytes, args: &ConnectionArgs) -> Result<()> {
    if art.kind != ArtifactKind::Gcode {
        return Err(err(
            "PRINTER_ARTIFACT",
            "This vendor expects a plain .gcode artifact",
            args,
        ));
    }
    Ok(())
}

fn http_base(host: &str) -> String {
    let host = host.trim().trim_end_matches('/');
    if host.starts_with("http://") || host.starts_with("https://") {
        host.to_owned()
    } else {
        format!("http://{host}")
    }
}

fn bambu_config(args: &ConnectionArgs) -> Result<BambuLanConfig> {
    let code = args.access_code.as_deref().unwrap_or("").to_owned();
    let serial = args.serial.as_deref().unwrap_or("").to_owned();
    let config = BambuLanConfig::new(args.host.trim(), code, serial);
    config.validate()?;
    Ok(config)
}

fn moonraker_config(args: &ConnectionArgs) -> Result<MoonrakerConfig> {
    let mut config = MoonrakerConfig::new(http_base(&args.host));
    if let Some(key) = &args.api_key {
        config = config.with_api_key(key.clone());
    }
    config.validate()?;
    Ok(config)
}

fn octoprint_config(args: &ConnectionArgs) -> Result<OctoPrintConfig> {
    let key = args.api_key.as_deref().unwrap_or("").to_owned();
    let config = OctoPrintConfig::new(http_base(&args.host), key);
    config.validate()?;
    Ok(config)
}

fn prusa_config(args: &ConnectionArgs) -> Result<PrusaLinkConfig> {
    let key = args.api_key.as_deref().unwrap_or("").to_owned();
    let config = PrusaLinkConfig::new(http_base(&args.host), key);
    config.validate()?;
    Ok(config)
}

fn creality_config(args: &ConnectionArgs) -> Result<CrealityConfig> {
    let mut config = CrealityConfig::new(http_base(&args.host));
    if let Some(key) = &args.api_key {
        config = config.with_api_key(key.clone());
    }
    config.validate()?;
    Ok(config)
}

fn snapmaker_config(args: &ConnectionArgs) -> Result<SnapmakerConfig> {
    let token = args
        .token
        .as_deref()
        .or(args.api_key.as_deref())
        .unwrap_or("")
        .to_owned();
    let config = SnapmakerConfig::new(http_base(&args.host), token);
    config.validate()?;
    Ok(config)
}

fn map_err(e: printer_core::Error, args: &ConnectionArgs) -> printer_core::Error {
    printer_core::Error::new(e.code, &scrub_secrets(&e.message, &args.secrets()))
}

fn err(code: &'static str, message: &str, args: &ConnectionArgs) -> printer_core::Error {
    printer_core::Error::new(code, &scrub_secrets(message, &args.secrets()))
}
