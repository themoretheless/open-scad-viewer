use clap::{Parser, Subcommand, ValueEnum};
use printer_cli::{
    ConnectionArgs, ControlAction, ServeOptions, VendorKind, control, discover, run_serve,
    send_path,
};
use printer_core::{DiscoveryVendor, scrub_secrets};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "printer-cli",
    about = "LAN printer discover / send / control companion"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// SSDP/UDP discovery for Bambu and Snapmaker (JSON by default).
    Discover {
        #[arg(long, default_value_t = 3)]
        timeout_secs: u64,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long, value_enum)]
        vendor: Vec<DiscoverVendorFlag>,
    },
    /// Upload + start a print artifact on a vendor backend.
    Send {
        #[arg(long, value_enum)]
        vendor: VendorFlag,
        #[arg(long)]
        host: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        access_code: Option<String>,
        #[arg(long)]
        serial: Option<String>,
        #[arg(long)]
        api_key: Option<String>,
        #[arg(long)]
        token: Option<String>,
        #[arg(long, default_value = "Metadata/plate_1.gcode")]
        plate: String,
        #[arg(long, default_value_t = true)]
        verify_start: bool,
        #[arg(long, default_value_t = false)]
        no_verify_start: bool,
    },
    Status {
        #[command(flatten)]
        conn: ConnArgs,
    },
    Pause {
        #[command(flatten)]
        conn: ConnArgs,
    },
    Resume {
        #[command(flatten)]
        conn: ConnArgs,
    },
    Stop {
        #[command(flatten)]
        conn: ConnArgs,
    },
    /// Loopback HTTP API for the Vue app (127.0.0.1 only).
    Serve {
        #[arg(long, default_value_t = 17890)]
        port: u16,
        #[arg(long, default_value_t = 3)]
        discover_timeout_secs: u64,
    },
}

#[derive(clap::Args, Debug)]
struct ConnArgs {
    #[arg(long, value_enum)]
    vendor: VendorFlag,
    #[arg(long)]
    host: String,
    #[arg(long)]
    access_code: Option<String>,
    #[arg(long)]
    serial: Option<String>,
    #[arg(long)]
    api_key: Option<String>,
    #[arg(long)]
    token: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum VendorFlag {
    Bambu,
    Moonraker,
    Octoprint,
    Prusa,
    Creality,
    Snapmaker,
}

impl From<VendorFlag> for VendorKind {
    fn from(v: VendorFlag) -> Self {
        match v {
            VendorFlag::Bambu => VendorKind::Bambu,
            VendorFlag::Moonraker => VendorKind::Moonraker,
            VendorFlag::Octoprint => VendorKind::OctoPrint,
            VendorFlag::Prusa => VendorKind::Prusa,
            VendorFlag::Creality => VendorKind::Creality,
            VendorFlag::Snapmaker => VendorKind::Snapmaker,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DiscoverVendorFlag {
    Bambu,
    Snapmaker,
}

impl From<DiscoverVendorFlag> for DiscoveryVendor {
    fn from(v: DiscoverVendorFlag) -> Self {
        match v {
            DiscoverVendorFlag::Bambu => DiscoveryVendor::Bambu,
            DiscoverVendorFlag::Snapmaker => DiscoveryVendor::Snapmaker,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Json,
    Table,
}

fn main() {
    let cli = Cli::parse();
    if let Err(message) = run(cli) {
        eprintln!("error: {message}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Discover {
            timeout_secs,
            format,
            vendor,
        } => {
            let vendors: Vec<_> = vendor.into_iter().map(DiscoveryVendor::from).collect();
            let found =
                discover(Duration::from_secs(timeout_secs), &vendors).map_err(|e| e.message)?;
            match format {
                OutputFormat::Json => {
                    let rows: Vec<_> = found
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "vendor": p.vendor,
                                "displayName": p.display_name,
                                "host": p.host,
                                "port": p.port,
                                "serial": p.serial,
                                "model": p.model,
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
                }
                OutputFormat::Table => {
                    println!(
                        "{:<10} {:<16} {:<22} {:<18} {}",
                        "VENDOR", "HOST", "SERIAL", "MODEL", "NAME"
                    );
                    for p in found {
                        println!(
                            "{:<10} {:<16} {:<22} {:<18} {}",
                            p.vendor,
                            p.host,
                            p.serial.unwrap_or_default(),
                            p.model.unwrap_or_default(),
                            p.display_name.unwrap_or_default()
                        );
                    }
                }
            }
            Ok(())
        }
        Commands::Send {
            vendor,
            host,
            file,
            access_code,
            serial,
            api_key,
            token,
            plate,
            verify_start,
            no_verify_start,
        } => {
            let args = ConnectionArgs {
                host,
                access_code,
                serial,
                api_key,
                token,
                plate_gcode_path: plate,
                verify_start: verify_start && !no_verify_start,
            };
            let secrets = args.secrets();
            let outcome = send_path(vendor.into(), &args, &file)
                .map_err(|e| scrub_secrets(&e.message, &secrets))?;
            println!(
                "{}",
                serde_json::json!({
                    "remoteName": outcome.remote_name,
                    "verified": outcome.verified,
                    "gcodeState": outcome.gcode_state,
                })
            );
            Ok(())
        }
        Commands::Status { conn } => do_control(conn, ControlAction::Status),
        Commands::Pause { conn } => do_control(conn, ControlAction::Pause),
        Commands::Resume { conn } => do_control(conn, ControlAction::Resume),
        Commands::Stop { conn } => do_control(conn, ControlAction::Stop),
        Commands::Serve {
            port,
            discover_timeout_secs,
        } => run_serve(ServeOptions {
            port,
            discover_timeout: Duration::from_secs(discover_timeout_secs),
        }),
    }
}

fn do_control(conn: ConnArgs, action: ControlAction) -> Result<(), String> {
    let args = ConnectionArgs {
        host: conn.host,
        access_code: conn.access_code,
        serial: conn.serial,
        api_key: conn.api_key,
        token: conn.token,
        plate_gcode_path: "Metadata/plate_1.gcode".into(),
        verify_start: true,
    };
    let secrets = args.secrets();
    let status = control(conn.vendor.into(), &args, action)
        .map_err(|e| scrub_secrets(&e.message, &secrets))?;
    if let Some(s) = status {
        println!(
            "{}",
            serde_json::json!({
                "state": format!("{:?}", s.state).to_ascii_lowercase(),
                "vendorState": s.vendor_state,
                "percent": s.percent,
                "layer": s.layer,
            })
        );
    } else {
        println!("{}", serde_json::json!({ "ok": true }));
    }
    Ok(())
}
