//! Local HTML development console entrypoint.

use std::env;
use std::error::Error;
use std::path::PathBuf;

use xavi_dev_console::{
    ChromeLaunchConfig, DEFAULT_BIND_ADDR, DEFAULT_CHROME_PATH, DEFAULT_CHROME_PROFILE_PATH,
    DEFAULT_OPEN_CYCLE_HOST, DEFAULT_OPEN_CYCLE_PORT, DEFAULT_REMOTE_DEBUGGING_PORT,
    DEFAULT_REPORT_LIMIT, DEFAULT_REPORTS_ROOT_PATH, DEFAULT_TRACE_DB_PATH, DevConsoleConfig,
    OpenCycleBrowserMode, OpenCycleConfig, OpenCycleServerStatus,
    copy_cycle_report_artifact_bundle, open_cycle_report, run_server, run_server_with_chrome,
    validate_cycle_report_artifact_bundle,
};

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().filter(|arg| !arg.starts_with("--")).map_or("serve", String::as_str);
    let options = CliOptions::parse(
        if matches!(command, "serve" | "export" | "open-cycle")
            && args.first().is_some_and(|arg| arg == command)
        {
            &args[1..]
        } else {
            &args
        },
    );
    let cycle_id = options
        .value("cycle-id")
        .or_else(|| options.value("cycle"))
        .unwrap_or("default")
        .to_owned();
    let db_path = options.value("db").unwrap_or(DEFAULT_TRACE_DB_PATH).to_owned();
    let bind_addr = options.value("addr").unwrap_or(DEFAULT_BIND_ADDR).to_owned();
    let report_limit = options
        .value("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_REPORT_LIMIT);
    let reports_dir = options.value("reports-dir").unwrap_or(DEFAULT_REPORTS_ROOT_PATH).to_owned();

    match command {
        "serve" => {
            let config =
                DevConsoleConfig { cycle_id, db_path, bind_addr, report_limit, reports_dir };
            if options.flag("launch-chrome") {
                let chrome_config = ChromeLaunchConfig {
                    chrome_path: PathBuf::from(
                        options.value("chrome-path").unwrap_or(DEFAULT_CHROME_PATH),
                    ),
                    profile_dir: PathBuf::from(
                        options.value("chrome-profile").unwrap_or(DEFAULT_CHROME_PROFILE_PATH),
                    ),
                    remote_debugging_port: options
                        .value("remote-debugging-port")
                        .and_then(|value| value.parse::<u16>().ok())
                        .unwrap_or(DEFAULT_REMOTE_DEBUGGING_PORT),
                };
                run_server_with_chrome(&config, &chrome_config)
            } else {
                run_server(&config)
            }
        }
        "export" => {
            if let Some(output_dir) = options.value("out").map(PathBuf::from) {
                copy_cycle_report_artifact_bundle(&reports_dir, &cycle_id, &output_dir)?;
                println!(
                    "copied existing cycle-report artifact: {} -> {}",
                    cycle_report_dir(&reports_dir, &cycle_id).display(),
                    output_dir.display()
                );
            } else {
                validate_cycle_report_artifact_bundle(&reports_dir, &cycle_id)?;
                println!(
                    "verified existing cycle-report artifact: {}",
                    cycle_report_dir(&reports_dir, &cycle_id).display()
                );
            }
            Ok(())
        }
        "open-cycle" => {
            let (host, port) = open_cycle_host_port(&options);
            let browser_mode = if options.flag("no-open") || options.flag("print-url-only") {
                OpenCycleBrowserMode::PrintUrlOnly
            } else {
                OpenCycleBrowserMode::OpenDefaultBrowser
            };
            let config = OpenCycleConfig { cycle_id, reports_dir, host, port, browser_mode };
            let outcome = open_cycle_report(&config)?;
            match outcome.server_status {
                OpenCycleServerStatus::Reused => {
                    println!("reused xavi-dev-console report server");
                }
                OpenCycleServerStatus::Started { process_id } => {
                    println!("started xavi-dev-console report server: pid={process_id}");
                }
            }
            if outcome.opened_browser {
                println!("opened cycle report: {}", outcome.url);
            } else {
                println!("{}", outcome.url);
            }
            Ok(())
        }
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn cycle_report_dir(reports_dir: &str, cycle_id: &str) -> PathBuf {
    PathBuf::from(reports_dir).join(cycle_id)
}

fn open_cycle_host_port(options: &CliOptions) -> (String, u16) {
    if let Some(addr) = options.value("addr")
        && let Some((host, port)) = addr.rsplit_once(':')
    {
        return (host.to_owned(), port.parse::<u16>().unwrap_or(DEFAULT_OPEN_CYCLE_PORT));
    }
    let host = options.value("host").unwrap_or(DEFAULT_OPEN_CYCLE_HOST).to_owned();
    let port = options
        .value("port")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_OPEN_CYCLE_PORT);
    (host, port)
}

fn print_help() {
    println!(
        "\
xavi-dev-console commands:
  serve [--cycle <id>] [--db <path>] [--addr 127.0.0.1:4176] [--limit <n>]
        [--reports-dir <path>] [--launch-chrome] [--chrome-path <path>]
        [--chrome-profile <path>] [--remote-debugging-port <port>]
  export [--cycle <id>] [--reports-dir <path>] [--out <dir>]
        verifies an existing cycle-report artifact; with --out, copies that bundle
  open-cycle --cycle-id <id> [--reports-dir <path>] [--host 127.0.0.1] [--port 4200]
        [--no-open|--print-url-only]
        reuses a matching report server or starts one, then opens /reports/<cycle_id>/

defaults:
  --db   .xavi/development_trace.sqlite3
  --addr 127.0.0.1:4176
  --reports-dir .xavi/reports/development_cycles
  open-cycle --host 127.0.0.1 --port 4200
  --chrome-path /Applications/Google Chrome.app/Contents/MacOS/Google Chrome
  --chrome-profile .xavi/chrome-dev-console-profile
  --remote-debugging-port 9223"
    );
}

struct CliOptions {
    pairs: Vec<(String, String)>,
    flags: Vec<String>,
}

impl CliOptions {
    fn parse(args: &[String]) -> Self {
        let mut pairs = Vec::new();
        let mut flags = Vec::new();
        let mut index = 0;
        while index < args.len() {
            let key = &args[index];
            if let Some(stripped) = key.strip_prefix("--") {
                if let Some((name, value)) = stripped.split_once('=') {
                    pairs.push((name.to_owned(), value.to_owned()));
                    index += 1;
                } else if args.get(index + 1).is_some_and(|value| !value.starts_with("--")) {
                    let value = args[index + 1].clone();
                    pairs.push((stripped.to_owned(), value));
                    index += 2;
                } else {
                    flags.push(stripped.to_owned());
                    index += 1;
                }
            } else {
                index += 1;
            }
        }
        Self { pairs, flags }
    }

    fn value(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value.as_str()))
    }

    fn flag(&self, key: &str) -> bool {
        self.flags.iter().any(|candidate| candidate == key)
    }
}
