//! Local HTML development console for `development_trace` cycles.

use std::error::Error;
use std::fmt;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;
use std::time::Duration;

use xavi_application::services::development_trace_service::DevelopmentTraceService;
use xavi_domain::development_cycle::validate_development_cycle_alias;
use xavi_domain::development_trace::{
    DevelopmentTraceAuditReport, DevelopmentTraceEntry, DevelopmentTraceFilter,
    DevelopmentTraceKind, NewDevelopmentTraceEntry, audit_development_trace_cycle,
    render_development_trace_audit_json, trace_text_sha256_hex,
};
use xavi_infrastructure::development_trace::sqlite_development_trace_store::SqliteDevelopmentTraceStore;

/// Default development trace DB used by the console.
pub const DEFAULT_TRACE_DB_PATH: &str = ".xavi/development_trace.sqlite3";

/// Default local-only bind address.
pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:4176";

/// Default number of trace rows returned in one report.
pub const DEFAULT_REPORT_LIMIT: usize = 200;

/// Default root directory for cycle-report artifacts.
pub const DEFAULT_REPORTS_ROOT_PATH: &str = ".xavi/reports/development_cycles";

/// Default host used by `open-cycle`.
pub const DEFAULT_OPEN_CYCLE_HOST: &str = "127.0.0.1";

/// Default report-server port used by `open-cycle`.
pub const DEFAULT_OPEN_CYCLE_PORT: u16 = 4200;

const ROLE_LANES: [&str; 10] = [
    "orchestra",
    "planning",
    "codegen",
    "review",
    "test",
    "analysis",
    "user-docs",
    "ai-docs",
    "cycle-report",
    "dev-console",
];

const SSE_POLL_INTERVAL: Duration = Duration::from_millis(900);
const SSE_MAX_POLLS: usize = 120;
const SSE_MAX_IDLE_POLLS: usize = 20;
const SSE_MAX_POLL_ENTRIES: usize = 100;
const SSE_WINDOW_MULTIPLIER: usize = 4;
const SSE_MAX_WINDOW_ENTRIES: usize = 400;
const REPORT_SOURCE_WINDOW_MULTIPLIER: usize = 4;
const REPORT_MAX_SOURCE_WINDOW_ENTRIES: usize = 1_000;
const CYCLE_REPORT_INDEX_FILE: &str = "index.html";
const CYCLE_REPORT_DIFF_FILE: &str = "diff.html";
const CYCLE_REPORT_REQUIRED_ARTIFACT_FILES: [&str; 5] =
    [CYCLE_REPORT_INDEX_FILE, "report.json", "raw.json", "audit.json", "context.md"];
const CYCLE_REPORT_DIRECT_ARTIFACT_FILES: [&str; 6] = [
    CYCLE_REPORT_INDEX_FILE,
    CYCLE_REPORT_DIFF_FILE,
    "report.json",
    "raw.json",
    "audit.json",
    "context.md",
];
const CYCLE_REPORT_BROWSER_ARTIFACT_FILES: [&str; 2] =
    [CYCLE_REPORT_INDEX_FILE, CYCLE_REPORT_DIFF_FILE];
const CYCLE_REPORT_ALIAS_INDEX_FILE: &str = "aliases.json";
const MAX_ACTIVE_CONNECTIONS: usize = 16;
const REPORT_SERVER_HTTP_MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
/// Maximum bytes read from one cycle-report artifact text file.
pub const MAX_REPORT_ARTIFACT_BYTES: usize = REPORT_SERVER_HTTP_MAX_RESPONSE_BYTES;
const REPORT_SERVER_READINESS_ATTEMPTS: usize = 40;
const REPORT_SERVER_READINESS_INTERVAL: Duration = Duration::from_millis(150);
const REPORT_SERVER_HTTP_TIMEOUT: Duration = Duration::from_secs(1);
const MAX_REQUEST_BYTES: usize = 1_048_576;
const MAX_POST_BODY_BYTES: usize = 65_536;
const PUBLIC_REDACTION_PLACEHOLDER: &str = "[공개 UI에서 숨김 처리된 민감 텍스트]";
const PUBLIC_EXCERPT_CHAR_LIMIT: usize = 1_400;

#[derive(Debug)]
enum HttpRequestReadError {
    Io(std::io::Error),
    PayloadTooLarge(&'static str),
}

impl HttpRequestReadError {
    fn is_payload_too_large(&self) -> bool {
        matches!(self, Self::PayloadTooLarge(_))
    }
}

impl fmt::Display for HttpRequestReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::PayloadTooLarge(message) => write!(formatter, "{message}"),
        }
    }
}

impl Error for HttpRequestReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::PayloadTooLarge(_) => None,
        }
    }
}

impl From<std::io::Error> for HttpRequestReadError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextFileSizeLimitError {
    path: PathBuf,
    max_bytes: usize,
    actual_bytes: Option<u64>,
}

impl fmt::Display for TextFileSizeLimitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.actual_bytes {
            Some(actual_bytes) => write!(
                formatter,
                "text file {} exceeded size limit: {actual_bytes} bytes > {} bytes",
                self.path.display(),
                self.max_bytes
            ),
            None => write!(
                formatter,
                "text file {} exceeded size limit: more than {} bytes",
                self.path.display(),
                self.max_bytes
            ),
        }
    }
}

impl Error for TextFileSizeLimitError {}

/// Runtime configuration for the local dev console.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevConsoleConfig {
    /// Cycle displayed by the first view.
    pub cycle_id: String,
    /// `SQLite` DB path.
    pub db_path: String,
    /// Local bind address.
    pub bind_addr: String,
    /// Maximum trace rows per report.
    pub report_limit: usize,
    /// Root directory containing cycle-report artifacts.
    pub reports_dir: String,
}

/// Browser behavior for `open-cycle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenCycleBrowserMode {
    /// Use `/usr/bin/open <url>` after server readiness succeeds.
    OpenDefaultBrowser,
    /// Print or return the URL without opening a GUI browser.
    PrintUrlOnly,
}

/// Runtime configuration for opening a cycle-report viewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCycleConfig {
    /// Cycle-report artifact id to display.
    pub cycle_id: String,
    /// Root directory containing cycle-report artifacts.
    pub reports_dir: String,
    /// Host used for the local report server.
    pub host: String,
    /// Port used for the local report server.
    pub port: u16,
    /// Whether the browser should be opened or only the URL should be returned.
    pub browser_mode: OpenCycleBrowserMode,
}

/// Planned command for starting the report server in the background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCycleServerPlan {
    /// Executable path.
    pub program: PathBuf,
    /// Command-line arguments passed to `xavi-dev-console`.
    pub args: Vec<String>,
    /// Process spawn mode used for the server.
    pub spawn_mode: OpenCycleServerSpawnMode,
}

/// How `open-cycle` starts a report-server process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenCycleServerSpawnMode {
    /// Keep the server alive after the short-lived `open-cycle` process exits.
    DetachedPersistent,
}

/// Planned command for opening the report URL in the default macOS browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserOpenPlan {
    /// Executable path.
    pub program: PathBuf,
    /// Command-line arguments passed to the opener.
    pub args: Vec<String>,
}

/// Whether `open-cycle` reused an existing server or started a new one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenCycleServerStatus {
    /// A matching xavi-dev-console report server was already listening.
    Reused,
    /// A new background server process was spawned.
    Started {
        /// Spawned process id reported by the OS.
        process_id: u32,
    },
}

/// Result returned by `open-cycle`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCycleOutcome {
    /// URL for the cycle-report viewer.
    pub url: String,
    /// Server action used to make the URL ready.
    pub server_status: OpenCycleServerStatus,
    /// Whether the default browser opener was executed.
    pub opened_browser: bool,
}

/// A cycle report projected from append-only trace entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentCycleReport {
    /// Cycle identifier.
    pub cycle_id: String,
    /// Timestamp string generated when the report was built.
    pub generated_at: String,
    /// Entries returned by the current query.
    pub entries: Vec<DevelopmentTraceEntry>,
    /// Total entries in this cycle before display limiting.
    pub total_entry_count: usize,
    /// Entries displayed in this report after latest-row limiting.
    pub displayed_entry_count: usize,
    /// User query/input count.
    pub user_query_count: usize,
    /// Orchestra judgment count.
    pub orchestra_judgment_count: usize,
    /// Agent dispatch count.
    pub agent_dispatch_count: usize,
    /// Agent return count.
    pub agent_return_count: usize,
    /// File summary count.
    pub file_summary_count: usize,
    /// Test summary count.
    pub test_summary_count: usize,
    /// Project knowledge note count.
    pub project_knowledge_note_count: usize,
    /// Strict metadata/phase trace integrity audit for the full cycle.
    pub trace_integrity: DevelopmentTraceAuditReport,
    /// Console-originated pending user inputs.
    pub pending_inputs: Vec<DevelopmentTraceEntry>,
}

/// Runs the local blocking HTTP server.
///
/// # Errors
///
/// Returns an error when the server cannot bind, read requests, or access the trace DB.
pub fn run_server(config: &DevConsoleConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    run_server_inner(config)
}

/// Opens an existing cycle-report artifact in the local report server.
///
/// # Errors
///
/// Returns an error when the artifact bundle is missing, the configured port is occupied by a
/// non-dev-console server, the server cannot become ready, or the browser opener fails.
pub fn open_cycle_report(
    config: &OpenCycleConfig,
) -> Result<OpenCycleOutcome, Box<dyn Error + Send + Sync>> {
    cycle_report_browser_index_file(&config.reports_dir, &config.cycle_id)?;
    let url = cycle_report_url(&config.host, config.port, &config.cycle_id);
    let server_status = match probe_existing_report_server(config) {
        ReportServerProbe::Matching => OpenCycleServerStatus::Reused,
        ReportServerProbe::NotRunning => {
            let exe_path = std::env::current_exe()?;
            let plan = open_cycle_server_start_plan(&exe_path, config);
            let mut child = spawn_report_server(&plan)?;
            wait_for_report_server_readiness(config, &mut child)?
        }
        ReportServerProbe::WrongServer(message) => {
            return Err(format!(
                "{}:{} is already in use but is not the matching xavi-dev-console report server: {message}",
                config.host, config.port
            )
            .into());
        }
    };
    let opened_browser = match config.browser_mode {
        OpenCycleBrowserMode::OpenDefaultBrowser => {
            open_browser_url(&url)?;
            true
        }
        OpenCycleBrowserMode::PrintUrlOnly => false,
    };
    Ok(OpenCycleOutcome { url, server_status, opened_browser })
}

/// Builds the canonical report URL used by `open-cycle`.
#[must_use]
pub fn cycle_report_url(host: &str, port: u16, cycle_id: &str) -> String {
    format!("http://{host}:{port}/reports/{}/", url_component_encode(cycle_id))
}

/// Builds the background server command used by `open-cycle`.
#[must_use]
pub fn open_cycle_server_start_plan(
    executable: impl AsRef<Path>,
    config: &OpenCycleConfig,
) -> OpenCycleServerPlan {
    OpenCycleServerPlan {
        program: executable.as_ref().to_path_buf(),
        args: vec![
            "serve".to_owned(),
            "--cycle".to_owned(),
            config.cycle_id.clone(),
            "--addr".to_owned(),
            format!("{}:{}", config.host, config.port),
            "--reports-dir".to_owned(),
            config.reports_dir.clone(),
        ],
        spawn_mode: OpenCycleServerSpawnMode::DetachedPersistent,
    }
}

/// Builds the macOS default-browser opener command without executing it.
#[must_use]
pub fn browser_open_plan(url: &str) -> BrowserOpenPlan {
    BrowserOpenPlan { program: PathBuf::from("/usr/bin/open"), args: vec![url.to_owned()] }
}

fn spawn_report_server(plan: &OpenCycleServerPlan) -> Result<Child, Box<dyn Error + Send + Sync>> {
    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    match plan.spawn_mode {
        OpenCycleServerSpawnMode::DetachedPersistent => {
            configure_detached_report_server_command(&mut command);
        }
    }
    Ok(command.spawn()?)
}

fn configure_detached_report_server_command(command: &mut Command) {
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        command.process_group(0);
    }
}

fn wait_for_report_server_readiness(
    config: &OpenCycleConfig,
    child: &mut Child,
) -> Result<OpenCycleServerStatus, Box<dyn Error + Send + Sync>> {
    for _ in 0..REPORT_SERVER_READINESS_ATTEMPTS {
        if let Some(status) = child.try_wait()? {
            return Err(format!(
                "xavi-dev-console report server exited before readiness: {status}"
            )
            .into());
        }
        match probe_existing_report_server(config) {
            ReportServerProbe::Matching => {
                return Ok(OpenCycleServerStatus::Started { process_id: child.id() });
            }
            ReportServerProbe::NotRunning => thread::sleep(REPORT_SERVER_READINESS_INTERVAL),
            ReportServerProbe::WrongServer(message) => {
                return Err(format!(
                    "{}:{} became occupied by a non-matching server during startup: {message}",
                    config.host, config.port
                )
                .into());
            }
        }
    }
    Err(format!(
        "xavi-dev-console report server did not become ready at {}:{}",
        config.host, config.port
    )
    .into())
}

fn open_browser_url(url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let plan = browser_open_plan(url);
    let status = Command::new(&plan.program).args(&plan.args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("browser opener exited unsuccessfully: {status}").into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReportServerProbe {
    Matching,
    NotRunning,
    WrongServer(String),
}

fn probe_existing_report_server(config: &OpenCycleConfig) -> ReportServerProbe {
    let health = match report_server_http_get(&config.host, config.port, "/api/health") {
        Ok(response) => response,
        Err(error) if is_connection_refused(error.as_ref()) => {
            return ReportServerProbe::NotRunning;
        }
        Err(error) => return ReportServerProbe::WrongServer(error.to_string()),
    };
    if !health.status_ok() {
        return ReportServerProbe::WrongServer(format!(
            "health endpoint returned {}",
            health.status_line
        ));
    }
    if let Err(message) = validate_report_server_health_body(&health.body, &config.reports_dir) {
        return ReportServerProbe::WrongServer(message);
    }

    let ready_path = format!("/api/reports/{}/ready", url_component_encode(&config.cycle_id));
    let ready = match report_server_http_get(&config.host, config.port, &ready_path) {
        Ok(response) => response,
        Err(error) => return ReportServerProbe::WrongServer(error.to_string()),
    };
    if !ready.status_ok() {
        return ReportServerProbe::WrongServer(format!(
            "readiness endpoint returned {} for {}",
            ready.status_line, ready_path
        ));
    }
    if let Err(message) =
        validate_report_server_readiness_body(&ready.body, &config.reports_dir, &config.cycle_id)
    {
        return ReportServerProbe::WrongServer(message);
    }

    ReportServerProbe::Matching
}

fn validate_report_server_health_body(
    body: &str,
    expected_reports_dir: &str,
) -> Result<(), String> {
    let service = json_string_field(body, "service").unwrap_or_default();
    if service != "xavi-dev-console" {
        return Err("health endpoint is missing xavi-dev-console service marker".to_owned());
    }
    let status = json_string_field(body, "status").unwrap_or_default();
    if status != "ok" {
        return Err(format!("health endpoint reported non-ok status: {status}"));
    }
    let reports_dir = json_string_field(body, "reports_dir").unwrap_or_default();
    if reports_dir != expected_reports_dir {
        return Err(format!("health endpoint reports different reports_dir: {reports_dir}"));
    }
    Ok(())
}

fn validate_report_server_readiness_body(
    body: &str,
    expected_reports_dir: &str,
    expected_cycle_id: &str,
) -> Result<(), String> {
    let service = json_string_field(body, "service").unwrap_or_default();
    if service != "xavi-dev-console" {
        return Err("readiness endpoint is missing xavi-dev-console service marker".to_owned());
    }
    let status = json_string_field(body, "status").unwrap_or_default();
    if status != "ok" {
        return Err(format!("readiness endpoint reported non-ok status: {status}"));
    }
    let reports_dir = json_string_field(body, "reports_dir").unwrap_or_default();
    if reports_dir != expected_reports_dir {
        return Err(format!("readiness endpoint reports different reports_dir: {reports_dir}"));
    }
    let cycle_id = json_string_field(body, "cycle_id").unwrap_or_default();
    if cycle_id != expected_cycle_id {
        return Err(format!("readiness endpoint reports different cycle_id: {cycle_id}"));
    }
    if json_bool_field(body, "artifact_files_present") != Some(true) {
        return Err("readiness endpoint reports incomplete artifact files".to_owned());
    }
    let Some(index_html_bytes) = json_u64_field(body, "index_html_bytes") else {
        return Err("readiness endpoint is missing index_html_bytes".to_owned());
    };
    if index_html_bytes == 0 {
        return Err("readiness endpoint reports invalid index.html: 0 bytes".to_owned());
    }
    Ok(())
}

fn is_connection_refused(error: &(dyn Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(io_error) = error.downcast_ref::<std::io::Error>()
            && matches!(
                io_error.kind(),
                std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
            )
        {
            return true;
        }
        current = error.source();
    }
    false
}

fn run_server_inner(config: &DevConsoleConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(&config.bind_addr)?;
    let frontend_url = frontend_url(config);
    println!("xavi-dev-console listening on {frontend_url}");
    let active_connections = Arc::new(AtomicUsize::new(0));

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if !try_reserve_connection(&active_connections) {
                    let _ = write_response(
                        &mut stream,
                        "503 Service Unavailable",
                        "text/plain; charset=utf-8",
                        "too many active dev-console connections",
                    );
                    continue;
                }
                let config = config.clone();
                let active_connections = Arc::clone(&active_connections);
                thread::spawn(move || {
                    let _permit = ActiveConnectionPermit::new(active_connections);
                    let mut stream = stream;
                    if let Err(error) = handle_connection(&mut stream, &config) {
                        let _ = write_response(
                            &mut stream,
                            "500 Internal Server Error",
                            "text/plain; charset=utf-8",
                            &format!("dev console error: {error}"),
                        );
                    }
                });
            }
            Err(error) => eprintln!("dev console connection error: {error}"),
        }
    }
    Ok(())
}

struct ActiveConnectionPermit {
    active_connections: Arc<AtomicUsize>,
}

impl ActiveConnectionPermit {
    fn new(active_connections: Arc<AtomicUsize>) -> Self {
        Self { active_connections }
    }
}

impl Drop for ActiveConnectionPermit {
    fn drop(&mut self) {
        self.active_connections.fetch_sub(1, Ordering::AcqRel);
    }
}

fn try_reserve_connection(active_connections: &AtomicUsize) -> bool {
    let mut current = active_connections.load(Ordering::Acquire);
    loop {
        if current >= MAX_ACTIVE_CONNECTIONS {
            return false;
        }
        match active_connections.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return true,
            Err(next) => current = next,
        }
    }
}

fn frontend_url(config: &DevConsoleConfig) -> String {
    format!("http://{}?cycle={}", config.bind_addr, url_component_encode(&config.cycle_id))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalHttpResponse {
    status_line: String,
    body: String,
}

impl LocalHttpResponse {
    fn status_ok(&self) -> bool {
        self.status_line.starts_with("HTTP/1.1 200") || self.status_line.starts_with("HTTP/1.0 200")
    }
}

fn report_server_http_get(
    host: &str,
    port: u16,
    path: &str,
) -> Result<LocalHttpResponse, Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(format!("{host}:{port}"))?;
    stream.set_read_timeout(Some(REPORT_SERVER_HTTP_TIMEOUT))?;
    stream.set_write_timeout(Some(REPORT_SERVER_HTTP_TIMEOUT))?;
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                response.extend_from_slice(&buffer[..read]);
                if response.len() > REPORT_SERVER_HTTP_MAX_RESPONSE_BYTES {
                    return Err(format!(
                        "report server {path} response exceeded size limit ({REPORT_SERVER_HTTP_MAX_RESPONSE_BYTES} bytes)"
                    )
                    .into());
                }
                if http_response_has_full_content_length_body(&response) {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Err(format!("report server {path} timed out waiting for response").into());
            }
            Err(error) => return Err(error.into()),
        }
    }

    let response = String::from_utf8(response)?;
    let status_line = response.lines().next().unwrap_or("").to_owned();
    let body = response.split_once("\r\n\r\n").map_or("", |(_, body)| body).to_owned();
    Ok(LocalHttpResponse { status_line, body })
}

fn http_response_has_full_content_length_body(response: &[u8]) -> bool {
    let Some((header_end, content_length)) = http_response_content_length(response) else {
        return false;
    };
    response.len().saturating_sub(header_end) >= content_length
}

fn http_response_content_length(response: &[u8]) -> Option<(usize, usize)> {
    let header_end = response.windows(4).position(|window| window == b"\r\n\r\n")? + 4;
    let headers = std::str::from_utf8(&response[..header_end]).ok()?;
    let content_length = headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    })?;
    Some((header_end, content_length))
}

fn json_object_slices(input: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut depth = 0_usize;
    let mut start = None;

    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start.take() {
                        objects.push(&input[start..=index]);
                    }
                }
            }
            _ => {}
        }
    }

    objects
}

fn json_string_field(object: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut search_start = 0;
    while let Some(relative_index) = object[search_start..].find(&needle) {
        let key_start = search_start + relative_index;
        let after_key = key_start + needle.len();
        let after_colon = object[after_key..].find(':').map(|index| after_key + index + 1)?;
        if let Some(value) = parse_json_string_value(&object[after_colon..]) {
            return Some(value);
        }
        search_start = after_key;
    }
    None
}

fn json_bool_field(object: &str, key: &str) -> Option<bool> {
    let value = json_raw_field_value(object, key)?;
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn json_u64_field(object: &str, key: &str) -> Option<u64> {
    let value = json_raw_field_value(object, key)?;
    let number = value.chars().take_while(char::is_ascii_digit).collect::<String>();
    if number.is_empty() { None } else { number.parse().ok() }
}

fn json_raw_field_value<'a>(object: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let key_start = object.find(&needle)?;
    let after_key = key_start + needle.len();
    let after_colon = object[after_key..].find(':').map(|index| after_key + index + 1)?;
    Some(object[after_colon..].trim_start())
}

fn parse_json_string_value(value: &str) -> Option<String> {
    let mut chars = value.trim_start().chars();
    if chars.next()? != '"' {
        return None;
    }

    let mut parsed = String::new();
    let mut escaped = false;
    for character in chars {
        if escaped {
            match character {
                '"' => parsed.push('"'),
                '\\' => parsed.push('\\'),
                '/' => parsed.push('/'),
                'n' => parsed.push('\n'),
                'r' => parsed.push('\r'),
                't' => parsed.push('\t'),
                'b' => parsed.push('\u{08}'),
                'f' => parsed.push('\u{0c}'),
                other => parsed.push(other),
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(parsed);
        } else {
            parsed.push(character);
        }
    }
    None
}

/// Builds a cycle report from the development trace service.
///
/// # Errors
///
/// Returns an error when trace entries cannot be read.
pub fn build_report(
    service: &DevelopmentTraceService,
    cycle_id: &str,
    limit: usize,
) -> Result<DevelopmentCycleReport, Box<dyn Error + Send + Sync>> {
    let source_window_limit = report_source_window_limit(limit);
    let source_entries = service.list_latest_entries(&DevelopmentTraceFilter {
        cycle_id: Some(cycle_id.to_owned()),
        kind: None,
        limit: Some(source_window_limit),
    })?;
    let trace_integrity = audit_development_trace_cycle(cycle_id, &source_entries);
    Ok(DevelopmentCycleReport::from_entries(
        cycle_id,
        latest_entries(source_entries, limit),
        trace_integrity,
    ))
}

fn report_source_window_limit(report_limit: usize) -> usize {
    report_limit
        .max(1)
        .saturating_mul(REPORT_SOURCE_WINDOW_MULTIPLIER)
        .min(REPORT_MAX_SOURCE_WINDOW_ENTRIES)
}

fn latest_entries(
    mut entries: Vec<DevelopmentTraceEntry>,
    limit: usize,
) -> (Vec<DevelopmentTraceEntry>, usize) {
    entries.sort_by_key(|entry| entry.id);
    let total_entry_count = entries.len();
    if limit == 0 {
        return (Vec::new(), total_entry_count);
    }
    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }
    (entries, total_entry_count)
}

impl DevelopmentCycleReport {
    /// Creates a report by grouping trace entries.
    #[must_use]
    pub fn from_entries(
        cycle_id: &str,
        entries: (Vec<DevelopmentTraceEntry>, usize),
        trace_integrity: DevelopmentTraceAuditReport,
    ) -> Self {
        let (entries, total_entry_count) = entries;
        let displayed_entry_count = entries.len();
        let mut report = Self {
            cycle_id: cycle_id.to_owned(),
            generated_at: epoch_timestamp(),
            entries,
            total_entry_count,
            displayed_entry_count,
            user_query_count: 0,
            orchestra_judgment_count: 0,
            agent_dispatch_count: 0,
            agent_return_count: 0,
            file_summary_count: 0,
            test_summary_count: 0,
            project_knowledge_note_count: 0,
            trace_integrity,
            pending_inputs: Vec::new(),
        };

        for entry in &report.entries {
            match entry.kind {
                DevelopmentTraceKind::UserQuery => report.user_query_count += 1,
                DevelopmentTraceKind::OrchestraJudgment => report.orchestra_judgment_count += 1,
                DevelopmentTraceKind::AgentDispatch => report.agent_dispatch_count += 1,
                DevelopmentTraceKind::AgentReturn => report.agent_return_count += 1,
                DevelopmentTraceKind::FileSummary => report.file_summary_count += 1,
                DevelopmentTraceKind::TestSummary => report.test_summary_count += 1,
                DevelopmentTraceKind::ProjectKnowledgeNote => {
                    report.project_knowledge_note_count += 1;
                }
            }
            if is_console_input(entry) {
                report.pending_inputs.push(entry.clone());
            }
        }

        report
    }
}

/// Appends user input submitted through the HTML console.
///
/// # Errors
///
/// Returns an error when the message is empty or the trace entry cannot be stored.
pub fn append_console_input(
    service: &DevelopmentTraceService,
    cycle_id: &str,
    input_type: &str,
    message: &str,
) -> Result<DevelopmentTraceEntry, Box<dyn Error + Send + Sync>> {
    append_console_input_with_invalidation(service, cycle_id, input_type, message, None)
}

fn append_console_input_with_invalidation(
    service: &DevelopmentTraceService,
    cycle_id: &str,
    input_type: &str,
    message: &str,
    invalidates_after_event_id: Option<&str>,
) -> Result<DevelopmentTraceEntry, Box<dyn Error + Send + Sync>> {
    let input_type = normalized_input_type(input_type);
    let message = message.trim();
    if message.is_empty() {
        return Err("콘솔 입력 내용이 비어 있습니다".into());
    }
    let invalidates_after_event_id =
        invalidates_after_event_id.map(str::trim).filter(|value| !value.is_empty());

    let summary =
        format!("개발 콘솔 {}: {}", input_type_public_label(input_type), one_line_summary(message));
    let event_id = generated_event_id("dev_console_input");
    let created_at = epoch_timestamp();
    let entry = NewDevelopmentTraceEntry {
        event_id: event_id.clone(),
        cycle_id: cycle_id.to_owned(),
        user_turn_id: Some(format!("dev-console-{cycle_id}")),
        kind: DevelopmentTraceKind::UserQuery,
        role_name: Some("user".to_owned()),
        summary,
        body: message.to_owned(),
        metadata_json: console_input_metadata_json(
            input_type,
            message,
            &event_id,
            &created_at,
            invalidates_after_event_id,
        ),
        created_at,
    };
    service.append_entry(&entry)
}

fn console_input_metadata_json(
    input_type: &str,
    message: &str,
    event_id: &str,
    timestamp: &str,
    invalidates_after_event_id: Option<&str>,
) -> String {
    let order = trace_sequence_no(event_id);
    let source_ref = format!("development_trace://events/{event_id}");
    let hash = trace_text_sha256_hex(message);
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "trace_contract_version", "2", true);
    push_json_field(&mut output, "phase_id", &json_string("dev-console-input"), false);
    push_json_field(&mut output, "cycle_step", &order.to_string(), false);
    push_json_field(&mut output, "role", &json_string("user"), false);
    push_json_field(&mut output, "status", &json_string("requested"), false);
    push_json_field(&mut output, "source", &json_string("xavi-dev-console"), false);
    push_json_field(&mut output, "source_kind", &json_string("dev_console_input"), false);
    push_json_field(&mut output, "source_event_id", &json_string(event_id), false);
    push_json_field(&mut output, "input_type", &json_string(input_type), false);
    if let Some(event_id) = invalidates_after_event_id {
        push_json_field(&mut output, "invalidates_after_event_id", &json_string(event_id), false);
    }
    let mut content = String::new();
    content.push('{');
    push_json_field(&mut content, "user_request", &json_string(message), true);
    push_json_field(&mut content, "constraints", "[\"entered through xavi-dev-console\"]", false);
    push_json_field(
        &mut content,
        "acceptance_criteria",
        "[\"pending input is visible to orchestra\"]",
        false,
    );
    let mut evidence = String::new();
    evidence.push('{');
    push_json_field(&mut evidence, "text", &json_string(message), true);
    push_json_field(&mut evidence, "source_type", &json_string("dev_console_input"), false);
    push_json_field(&mut evidence, "source_ref", &json_string(&source_ref), false);
    push_json_field(&mut evidence, "role", &json_string("user"), false);
    push_json_field(&mut evidence, "agent_id", "null", false);
    push_json_field(&mut evidence, "hash_sha256", &json_string(&hash), false);
    push_json_field(&mut evidence, "timestamp", &json_string(timestamp), false);
    push_json_field(&mut evidence, "order", &order.to_string(), false);
    evidence.push('}');
    push_json_field(&mut content, "user_request_verbatim", &evidence, false);
    content.push('}');
    push_json_field(&mut output, "content_json", &content, false);
    output.push('}');
    output
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoleLaneSummary {
    role: &'static str,
    status: &'static str,
    entry_count: usize,
    latest_entry_id: Option<i64>,
    latest_kind: Option<&'static str>,
    latest_summary: Option<String>,
    latest_role_source: Option<&'static str>,
}

fn role_lane_summaries(entries: &[DevelopmentTraceEntry]) -> Vec<RoleLaneSummary> {
    ROLE_LANES
        .iter()
        .map(|role| {
            let matching_entries = entries
                .iter()
                .filter(|entry| role_lane_for_entry(entry) == *role)
                .collect::<Vec<_>>();
            let latest = matching_entries.last().copied();
            RoleLaneSummary {
                role,
                status: latest.map_or("idle", status_for_entry),
                entry_count: matching_entries.len(),
                latest_entry_id: latest.map(|entry| entry.id),
                latest_kind: latest.map(|entry| entry.kind.as_str()),
                latest_summary: latest.map(|entry| public_text(&entry.summary)),
                latest_role_source: latest.map(|entry| role_resolution_for_entry(entry).source),
            }
        })
        .collect()
}

fn role_lane_for_entry(entry: &DevelopmentTraceEntry) -> &'static str {
    role_resolution_for_entry(entry).role
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoleResolution {
    role: &'static str,
    source: &'static str,
}

fn role_resolution_for_entry(entry: &DevelopmentTraceEntry) -> RoleResolution {
    if is_console_input(entry) {
        return RoleResolution { role: "dev-console", source: "console_input" };
    }

    if let Some(role_name) = entry.role_name.as_deref().and_then(known_role_name) {
        return RoleResolution { role: role_name, source: "explicit" };
    }

    if entry.kind == DevelopmentTraceKind::AgentReturn && role_name_is_missing(entry) {
        if let Some(role) = infer_role_from_event_id_prefix(&entry.event_id) {
            return RoleResolution { role, source: "inferred_from_event_id" };
        }
        if let Some(role) = infer_role_from_summary_prefix(&entry.summary) {
            return RoleResolution { role, source: "inferred_from_summary" };
        }
    }

    match entry.kind {
        DevelopmentTraceKind::TestSummary => {
            RoleResolution { role: "test", source: "kind_default" }
        }
        _ => RoleResolution { role: "orchestra", source: "missing_role_metadata" },
    }
}

fn role_name_is_missing(entry: &DevelopmentTraceEntry) -> bool {
    entry.role_name.as_deref().map_or("", str::trim).is_empty()
}

fn infer_role_from_event_id_prefix(event_id: &str) -> Option<&'static str> {
    infer_role_from_prefix(event_id, &['-', ':'])
}

fn infer_role_from_summary_prefix(summary: &str) -> Option<&'static str> {
    infer_role_from_prefix(summary, &[':', '-'])
}

fn infer_role_from_prefix(value: &str, separators: &[char]) -> Option<&'static str> {
    let normalized = value.trim_start().to_ascii_lowercase().replace('_', "-");
    ROLE_LANES.iter().copied().find(|role| {
        normalized == *role
            || separators
                .iter()
                .any(|separator| normalized.starts_with(&format!("{role}{separator}")))
    })
}

fn known_role_name(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
    ROLE_LANES.iter().copied().find(|role| *role == normalized)
}

fn status_for_entry(entry: &DevelopmentTraceEntry) -> &'static str {
    match entry.kind {
        DevelopmentTraceKind::UserQuery => "queued",
        DevelopmentTraceKind::OrchestraJudgment => "accepted",
        DevelopmentTraceKind::AgentDispatch => "running",
        DevelopmentTraceKind::AgentReturn => "returned",
        DevelopmentTraceKind::FileSummary
        | DevelopmentTraceKind::TestSummary
        | DevelopmentTraceKind::ProjectKnowledgeNote => "complete",
    }
}

fn source_type_for_entry(entry: &DevelopmentTraceEntry) -> &'static str {
    match entry.kind {
        DevelopmentTraceKind::UserQuery => "user",
        DevelopmentTraceKind::OrchestraJudgment => "orchestra",
        DevelopmentTraceKind::AgentDispatch | DevelopmentTraceKind::AgentReturn => "agent",
        DevelopmentTraceKind::FileSummary | DevelopmentTraceKind::TestSummary => "tool",
        DevelopmentTraceKind::ProjectKnowledgeNote => "system",
    }
}

/// Renders report JSON consumed by the HTML console.
#[must_use]
pub fn render_report_json(report: &DevelopmentCycleReport) -> String {
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "cycle_id", &json_string(&report.cycle_id), true);
    push_json_field(&mut output, "generated_at", &json_string(&report.generated_at), false);
    push_json_field(&mut output, "entry_count", &report.displayed_entry_count.to_string(), false);
    push_json_field(
        &mut output,
        "displayed_entry_count",
        &report.displayed_entry_count.to_string(),
        false,
    );
    push_json_field(&mut output, "total_entry_count", &report.total_entry_count.to_string(), false);
    output.push_str(",\"counts\":{");
    push_json_field(&mut output, "user_query", &report.user_query_count.to_string(), true);
    push_json_field(
        &mut output,
        "orchestra_judgment",
        &report.orchestra_judgment_count.to_string(),
        false,
    );
    push_json_field(&mut output, "agent_dispatch", &report.agent_dispatch_count.to_string(), false);
    push_json_field(&mut output, "agent_return", &report.agent_return_count.to_string(), false);
    push_json_field(&mut output, "file_summary", &report.file_summary_count.to_string(), false);
    push_json_field(&mut output, "test_summary", &report.test_summary_count.to_string(), false);
    push_json_field(
        &mut output,
        "project_knowledge_note",
        &report.project_knowledge_note_count.to_string(),
        false,
    );
    output.push('}');
    output.push_str(",\"role_lanes\":");
    push_role_lanes_array(&mut output, &role_lane_summaries(&report.entries));
    output.push_str(",\"trace_integrity\":");
    output.push_str(&render_development_trace_audit_json(&report.trace_integrity));
    push_json_field(
        &mut output,
        "evidence_status",
        &json_string(trace_evidence_status(&report.trace_integrity)),
        false,
    );
    output.push_str(",\"commands\":");
    output.push_str(&command_records_json(&report.entries));
    output.push_str(",\"cycle_refs\":");
    output.push_str(&cycle_refs_json(&report.entries));
    output.push_str(",\"audit\":");
    output.push_str(&trace_report_audit_projection_json(&report.trace_integrity));
    output.push_str(",\"pending_inputs\":");
    push_entries_array(&mut output, &report.pending_inputs);
    output.push_str(",\"entries\":");
    push_entries_array(&mut output, &report.entries);
    output.push('}');
    output
}

/// Renders raw trace rows for an artifact bundle.
#[must_use]
pub fn render_raw_trace_json(entries: &[DevelopmentTraceEntry]) -> String {
    let mut output = String::new();
    output.push('[');
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push('{');
        push_json_field(&mut output, "id", &entry.id.to_string(), true);
        push_json_field(&mut output, "event_id", &json_string(&entry.event_id), false);
        push_json_field(&mut output, "cycle_id", &json_string(&entry.cycle_id), false);
        push_json_field(
            &mut output,
            "user_turn_id",
            &json_optional_string(entry.user_turn_id.as_deref()),
            false,
        );
        push_json_field(&mut output, "kind", &json_string(entry.kind.as_str()), false);
        push_json_field(
            &mut output,
            "role_name",
            &json_optional_string(entry.role_name.as_deref()),
            false,
        );
        push_json_field(&mut output, "summary", &json_string(&entry.summary), false);
        push_json_field(&mut output, "body", &json_string(&entry.body), false);
        push_json_field(&mut output, "metadata_json", &json_string(&entry.metadata_json), false);
        let content_json = json_value_field_snippet(&entry.metadata_json, "content_json")
            .unwrap_or_else(|| "null".to_owned());
        push_json_field(&mut output, "metadata_content_json", &content_json, false);
        push_json_field(&mut output, "created_at", &json_string(&entry.created_at), false);
        output.push('}');
    }
    output.push(']');
    output
}

fn command_records_json(entries: &[DevelopmentTraceEntry]) -> String {
    let records = entries
        .iter()
        .filter(|entry| entry.kind == DevelopmentTraceKind::TestSummary)
        .filter_map(command_record_from_entry)
        .collect::<Vec<_>>();
    let mut output = String::new();
    output.push('[');
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(record);
    }
    output.push(']');
    output
}

fn command_record_from_entry(entry: &DevelopmentTraceEntry) -> Option<String> {
    metadata_content_json(entry)
        .and_then(|content| json_value_field_snippet(&content, "command_record"))
        .filter(|record| record.trim_start().starts_with('{'))
}

fn cycle_refs_json(entries: &[DevelopmentTraceEntry]) -> String {
    let baseline =
        entries.iter().find_map(|entry| cycle_ref_from_entry(entry, "cycle_baseline_ref"));
    let head = entries.iter().rev().find_map(|entry| cycle_ref_from_entry(entry, "cycle_head_ref"));
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "baseline", baseline.as_deref().unwrap_or("null"), true);
    push_json_field(&mut output, "head", head.as_deref().unwrap_or("null"), false);
    output.push('}');
    output
}

fn cycle_ref_from_entry(entry: &DevelopmentTraceEntry, field_name: &str) -> Option<String> {
    metadata_content_json(entry)
        .and_then(|content| json_value_field_snippet(&content, field_name))
        .filter(|reference| reference.trim_start().starts_with('{'))
}

fn metadata_content_json(entry: &DevelopmentTraceEntry) -> Option<String> {
    json_value_field_snippet(&entry.metadata_json, "content_json")
}

fn trace_report_audit_projection_json(report: &DevelopmentTraceAuditReport) -> String {
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "status", &json_string(trace_audit_schema_status(report)), true);
    push_json_field(
        &mut output,
        "evidence_status",
        &json_string(trace_evidence_status(report)),
        false,
    );
    push_json_field(
        &mut output,
        "missing_required_inputs",
        &audit_messages_json(report, is_missing_required_input_finding),
        false,
    );
    push_json_field(
        &mut output,
        "missing_evidence",
        &audit_finding_notes_json(report, is_missing_evidence_finding),
        false,
    );
    push_json_field(
        &mut output,
        "invalid_evidence",
        &audit_finding_notes_json(report, is_invalid_evidence_finding),
        false,
    );
    push_json_field(
        &mut output,
        "warnings",
        &audit_messages_json(report, |finding| finding.severity == "warning"),
        false,
    );
    push_json_field(&mut output, "derived_not_verbatim", "[]", false);
    output.push_str(",\"trace_audit\":");
    output.push_str(&render_development_trace_audit_json(report));
    output.push_str(",\"findings\":");
    output.push_str(&audit_finding_notes_json(report, |_| true));
    output.push('}');
    output
}

fn trace_audit_schema_status(report: &DevelopmentTraceAuditReport) -> &'static str {
    if report.failure_count > 0 {
        "fail"
    } else if report.warning_count > 0 {
        "warn"
    } else {
        "pass"
    }
}

fn trace_evidence_status(report: &DevelopmentTraceAuditReport) -> &'static str {
    if report.failure_count > 0 {
        "fail"
    } else if report.warning_count > 0 {
        "incomplete"
    } else {
        "complete"
    }
}

fn audit_messages_json<F>(report: &DevelopmentTraceAuditReport, predicate: F) -> String
where
    F: Fn(&xavi_domain::development_trace::DevelopmentTraceAuditFinding) -> bool,
{
    let mut output = String::new();
    output.push('[');
    let mut first = true;
    for finding in report.findings.iter().filter(|finding| predicate(finding)) {
        if !first {
            output.push(',');
        }
        first = false;
        output.push_str(&json_string(&finding.message));
    }
    output.push(']');
    output
}

fn audit_finding_notes_json<F>(report: &DevelopmentTraceAuditReport, predicate: F) -> String
where
    F: Fn(&xavi_domain::development_trace::DevelopmentTraceAuditFinding) -> bool,
{
    let mut output = String::new();
    output.push('[');
    let mut first = true;
    for finding in report.findings.iter().filter(|finding| predicate(finding)) {
        if !first {
            output.push(',');
        }
        first = false;
        output.push('{');
        push_json_field(&mut output, "field", &json_string(finding.code), true);
        push_json_field(&mut output, "reason", &json_string(&finding.message), false);
        push_json_field(
            &mut output,
            "current_value_classification",
            &json_string(finding.severity),
            false,
        );
        push_json_field(
            &mut output,
            "source_ref",
            &json_optional_string(finding.event_id.as_deref()),
            false,
        );
        output.push('}');
    }
    output.push(']');
    output
}

fn is_missing_required_input_finding(
    finding: &xavi_domain::development_trace::DevelopmentTraceAuditFinding,
) -> bool {
    matches!(
        finding.code,
        "missing_required_event"
            | "missing_cycle_report"
            | "missing_cycle_baseline_ref"
            | "missing_cycle_head_ref"
            | "missing_agent_return"
            | "missing_parent_dispatch"
            | "missing_parent_event_id"
    )
}

fn is_missing_evidence_finding(
    finding: &xavi_domain::development_trace::DevelopmentTraceAuditFinding,
) -> bool {
    matches!(finding.code, "missing_verbatim_evidence" | "missing_command_evidence")
}

fn is_invalid_evidence_finding(
    finding: &xavi_domain::development_trace::DevelopmentTraceAuditFinding,
) -> bool {
    finding.code.starts_with("invalid_")
        || matches!(finding.code, "source_mismatch" | "agent_id_mismatch")
}

/// Renders a compact markdown context view from a cycle report model.
#[must_use]
pub fn render_cycle_report_context_markdown(report: &DevelopmentCycleReport) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "# Cycle Report Context\n");
    let _ = writeln!(output, "- cycle_id: `{}`", report.cycle_id);
    let _ = writeln!(output, "- generated_at: `{}`", report.generated_at);
    let _ = writeln!(output, "- displayed_entry_count: `{}`", report.displayed_entry_count);
    let _ = writeln!(output, "- total_entry_count: `{}`", report.total_entry_count);
    let _ = writeln!(output, "- trace_integrity: `{}`", report.trace_integrity.status);
    output.push_str("\n## Notes\n\n");
    output.push_str(
        "This artifact is a derived viewer bundle. The append-only development_trace ledger remains the source of truth.\n",
    );
    output
}

fn javascript_safe_json_literal(json: &str) -> String {
    let mut escaped = String::with_capacity(json.len());
    for character in json.chars() {
        match character {
            '<' => escaped.push_str("\\u003c"),
            '>' => escaped.push_str("\\u003e"),
            '&' => escaped.push_str("\\u0026"),
            '\u{2028}' => escaped.push_str("\\u2028"),
            '\u{2029}' => escaped.push_str("\\u2029"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// File-backed cycle report artifact produced after an orchestration cycle closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleReportArtifact {
    /// Cycle identifier from the artifact path.
    pub cycle_id: String,
    /// Public structured report JSON.
    pub report_json: String,
    /// Raw trace JSON artifact, when the cycle-report role produced it.
    pub raw_json: Option<String>,
    /// Audit JSON artifact, when the cycle-report role produced it.
    pub audit_json: Option<String>,
    /// Context markdown artifact, when the cycle-report role produced it.
    pub context_markdown: Option<String>,
}

impl CycleReportArtifact {
    /// Builds an in-memory artifact from file contents.
    #[must_use]
    pub fn from_parts(
        cycle_id: &str,
        report_json: String,
        raw_json: Option<String>,
        audit_json: Option<String>,
        context_markdown: Option<String>,
    ) -> Self {
        Self { cycle_id: cycle_id.to_owned(), report_json, raw_json, audit_json, context_markdown }
    }
}

/// Reads a cycle-report artifact bundle from `reports_root/<cycle_id>/`.
///
/// # Errors
///
/// Returns an error when the cycle id is unsafe, the required `report.json` is absent, or a file
/// cannot be read.
pub fn read_cycle_report_artifact(
    reports_root: impl AsRef<Path>,
    cycle_id: &str,
) -> Result<CycleReportArtifact, Box<dyn Error + Send + Sync>> {
    let report_dir = canonical_cycle_report_dir(reports_root.as_ref(), cycle_id)?;
    let report_json = read_cycle_report_artifact_text_file(&report_dir, "report.json")?;
    let raw_json = read_optional_cycle_report_artifact_text_file(&report_dir, "raw.json")?;
    let audit_json = read_optional_cycle_report_artifact_text_file(&report_dir, "audit.json")?;
    let context_markdown =
        read_optional_cycle_report_artifact_text_file(&report_dir, "context.md")?;
    let artifact = CycleReportArtifact::from_parts(
        cycle_id,
        report_json,
        raw_json,
        audit_json,
        context_markdown,
    );
    ensure_cycle_report_artifact_text_total_within_limit(
        &artifact,
        &report_dir,
        MAX_REPORT_ARTIFACT_BYTES,
    )?;
    Ok(artifact)
}

/// Reads the report-list artifact from `reports_root/latest.json`.
///
/// # Errors
///
/// Returns an error when `latest.json` cannot be read.
pub fn read_cycle_report_latest_json(
    reports_root: impl AsRef<Path>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    read_text_file_bounded(&reports_root.as_ref().join("latest.json"), MAX_REPORT_ARTIFACT_BYTES)
}

/// Reads the report alias index artifact from `reports_root/aliases.json`.
///
/// # Errors
///
/// Returns an error when `aliases.json` cannot be read.
pub fn read_cycle_report_aliases_json(
    reports_root: impl AsRef<Path>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    read_text_file_bounded(
        &reports_root.as_ref().join(CYCLE_REPORT_ALIAS_INDEX_FILE),
        MAX_REPORT_ARTIFACT_BYTES,
    )
}

/// Resolves a human-readable report alias through `aliases.json`.
///
/// # Errors
///
/// Returns an error when the alias is invalid, the alias index is missing or malformed, or the
/// alias maps ambiguously.
pub fn resolve_cycle_report_alias(
    reports_root: impl AsRef<Path>,
    cycle_alias: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    validate_development_cycle_alias(cycle_alias)
        .map_err(|error| format!("invalid cycle report alias: {error}"))?;
    let aliases_json = read_cycle_report_aliases_json(reports_root)?;
    resolve_cycle_report_alias_from_json(&aliases_json, cycle_alias)
}

/// Reads one file from an existing cycle-report artifact bundle.
///
/// # Errors
///
/// Returns an error when the cycle id or file name is unsafe, the file is absent, or the file
/// cannot be read. This viewer path never builds a replacement artifact from the trace DB.
pub fn read_cycle_report_artifact_file(
    reports_root: impl AsRef<Path>,
    cycle_id: &str,
    file_name: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    validate_cycle_report_artifact_file_name(file_name)?;
    let report_dir = validate_cycle_report_artifact_bundle_dir(reports_root.as_ref(), cycle_id)?;
    read_cycle_report_artifact_text_file(&report_dir, file_name)
}

/// Verifies that the cycle-report role already produced a complete artifact bundle.
///
/// # Errors
///
/// Returns an error when any required artifact file is missing or not a regular file.
pub fn validate_cycle_report_artifact_bundle(
    reports_root: impl AsRef<Path>,
    cycle_id: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    validate_cycle_report_artifact_bundle_dir(reports_root.as_ref(), cycle_id).map(|_| ())
}

fn cycle_report_browser_index_file(
    reports_root: impl AsRef<Path>,
    cycle_id: &str,
) -> Result<(PathBuf, u64), Box<dyn Error + Send + Sync>> {
    cycle_report_browser_artifact_file(reports_root, cycle_id, CYCLE_REPORT_INDEX_FILE)
}

fn cycle_report_browser_artifact_file(
    reports_root: impl AsRef<Path>,
    cycle_id: &str,
    file_name: &str,
) -> Result<(PathBuf, u64), Box<dyn Error + Send + Sync>> {
    if !CYCLE_REPORT_BROWSER_ARTIFACT_FILES.contains(&file_name) {
        return Err(format!("unsupported cycle-report browser artifact: {file_name}").into());
    }
    let report_dir = validate_cycle_report_artifact_bundle_dir(reports_root.as_ref(), cycle_id)?;
    validate_cycle_report_browser_html_file(&report_dir, file_name)
}

fn validate_cycle_report_browser_html_file(
    canonical_report_dir: &Path,
    file_name: &str,
) -> Result<(PathBuf, u64), Box<dyn Error + Send + Sync>> {
    let html_file = validated_cycle_report_artifact_file(canonical_report_dir, file_name)?;
    if html_file.bytes == 0 {
        return Err(
            format!("cycle-report {file_name} is empty: {}", html_file.path.display()).into()
        );
    }
    Ok((html_file.path, html_file.bytes))
}

fn cycle_report_optional_diff_html_present(reports_root: impl AsRef<Path>, cycle_id: &str) -> bool {
    let Ok(report_dir) = validate_cycle_report_artifact_bundle_dir(reports_root.as_ref(), cycle_id)
    else {
        return false;
    };
    optional_validated_cycle_report_artifact_file(&report_dir, CYCLE_REPORT_DIFF_FILE)
        .map(|artifact| artifact.is_some())
        .unwrap_or(false)
}

/// Copies an existing cycle-report artifact bundle to another directory.
///
/// # Errors
///
/// Returns an error when the source bundle is incomplete or the files cannot be copied. This
/// command path is intentionally file-backed and does not read the development trace DB.
pub fn copy_cycle_report_artifact_bundle(
    reports_root: impl AsRef<Path>,
    cycle_id: &str,
    output_dir: impl AsRef<Path>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let source_dir = validate_cycle_report_artifact_bundle_dir(reports_root.as_ref(), cycle_id)?;
    if source_dir == output_dir.as_ref() {
        return Err(
            "output directory must differ from source cycle-report artifact directory".into()
        );
    }
    std::fs::create_dir_all(output_dir.as_ref())?;
    for file_name in CYCLE_REPORT_REQUIRED_ARTIFACT_FILES {
        let source_file = validated_cycle_report_artifact_file(&source_dir, file_name)?;
        std::fs::copy(source_file.path, output_dir.as_ref().join(file_name))?;
    }
    if let Some(diff_file) =
        optional_validated_cycle_report_artifact_file(&source_dir, CYCLE_REPORT_DIFF_FILE)?
    {
        std::fs::copy(diff_file.path, output_dir.as_ref().join(CYCLE_REPORT_DIFF_FILE))?;
    }
    Ok(())
}

fn validate_cycle_report_artifact_bundle_dir(
    reports_root: &Path,
    cycle_id: &str,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let report_dir = canonical_cycle_report_dir(reports_root, cycle_id)?;
    ensure_cycle_report_artifact_bundle(&report_dir)?;
    Ok(report_dir)
}

fn ensure_cycle_report_artifact_bundle(
    canonical_report_dir: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for file_name in CYCLE_REPORT_REQUIRED_ARTIFACT_FILES {
        validated_cycle_report_artifact_file(canonical_report_dir, file_name)?;
    }
    Ok(())
}

fn read_cycle_report_artifact_text_file(
    canonical_report_dir: &Path,
    file_name: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let artifact_file = validated_cycle_report_artifact_file(canonical_report_dir, file_name)?;
    read_text_file_bounded(&artifact_file.path, MAX_REPORT_ARTIFACT_BYTES)
}

fn read_optional_cycle_report_artifact_text_file(
    canonical_report_dir: &Path,
    file_name: &str,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let Some(artifact_file) =
        optional_validated_cycle_report_artifact_file(canonical_report_dir, file_name)?
    else {
        return Ok(None);
    };
    read_text_file_bounded(&artifact_file.path, MAX_REPORT_ARTIFACT_BYTES).map(Some)
}

#[derive(Debug)]
struct ValidatedCycleReportArtifactFile {
    path: PathBuf,
    bytes: u64,
}

fn canonical_cycle_report_dir(
    reports_root: &Path,
    cycle_id: &str,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    validate_cycle_report_id(cycle_id)?;
    let canonical_reports_root = canonical_reports_root(reports_root)?;
    let lexical_report_dir = reports_root.join(cycle_id);
    let canonical_report_dir = std::fs::canonicalize(&lexical_report_dir).map_err(|error| {
        format!("missing cycle-report artifact directory {}: {error}", lexical_report_dir.display())
    })?;
    let metadata = std::fs::metadata(&canonical_report_dir).map_err(|error| {
        format!(
            "cycle-report artifact directory cannot be inspected {}: {error}",
            canonical_report_dir.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "cycle-report artifact path is not a directory: {}",
            canonical_report_dir.display()
        )
        .into());
    }
    ensure_path_inside_boundary(
        &canonical_report_dir,
        &canonical_reports_root,
        "cycle-report artifact directory",
        "reports root",
    )?;
    Ok(canonical_report_dir)
}

fn canonical_reports_root(reports_root: &Path) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let canonical_reports_root = std::fs::canonicalize(reports_root).map_err(|error| {
        format!("missing cycle-report reports root {}: {error}", reports_root.display())
    })?;
    let metadata = std::fs::metadata(&canonical_reports_root).map_err(|error| {
        format!(
            "cycle-report reports root cannot be inspected {}: {error}",
            canonical_reports_root.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "cycle-report reports root is not a directory: {}",
            canonical_reports_root.display()
        )
        .into());
    }
    Ok(canonical_reports_root)
}

fn validated_cycle_report_artifact_file(
    canonical_report_dir: &Path,
    file_name: &str,
) -> Result<ValidatedCycleReportArtifactFile, Box<dyn Error + Send + Sync>> {
    optional_validated_cycle_report_artifact_file(canonical_report_dir, file_name)?.ok_or_else(
        || {
            format!(
                "missing cycle-report artifact file {}",
                canonical_report_dir.join(file_name).display()
            )
            .into()
        },
    )
}

fn optional_validated_cycle_report_artifact_file(
    canonical_report_dir: &Path,
    file_name: &str,
) -> Result<Option<ValidatedCycleReportArtifactFile>, Box<dyn Error + Send + Sync>> {
    validate_cycle_report_artifact_file_name(file_name)?;
    let artifact_path = canonical_report_dir.join(file_name);
    let symlink_metadata = match std::fs::symlink_metadata(&artifact_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "cycle-report artifact file cannot be inspected {}: {error}",
                artifact_path.display()
            )
            .into());
        }
    };
    if CYCLE_REPORT_BROWSER_ARTIFACT_FILES.contains(&file_name)
        && symlink_metadata.file_type().is_symlink()
    {
        return Err(format!(
            "cycle-report {file_name} must not be a symlink: {}",
            artifact_path.display()
        )
        .into());
    }
    let canonical_artifact_path = std::fs::canonicalize(&artifact_path).map_err(|error| {
        format!(
            "cycle-report artifact file cannot be canonicalized {}: {error}",
            artifact_path.display()
        )
    })?;
    ensure_path_inside_boundary(
        &canonical_artifact_path,
        canonical_report_dir,
        "cycle-report artifact file",
        "cycle-report artifact directory",
    )?;
    let metadata = std::fs::metadata(&canonical_artifact_path).map_err(|error| {
        format!(
            "cycle-report artifact file cannot be inspected {}: {error}",
            canonical_artifact_path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "cycle-report artifact path is not a file: {}",
            canonical_artifact_path.display()
        )
        .into());
    }
    Ok(Some(ValidatedCycleReportArtifactFile {
        path: canonical_artifact_path,
        bytes: metadata.len(),
    }))
}

fn ensure_path_inside_boundary(
    path: &Path,
    boundary: &Path,
    path_label: &str,
    boundary_label: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if path.starts_with(boundary) && path != boundary {
        return Ok(());
    }
    Err(format!(
        "{path_label} escapes {boundary_label}: {} is outside {}",
        path.display(),
        boundary.display()
    )
    .into())
}

fn read_text_file_bounded(
    path: &Path,
    max_bytes: usize,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let metadata = std::fs::metadata(path)?;
    let max_bytes_u64 = max_bytes as u64;
    if metadata.len() > max_bytes_u64 {
        return Err(TextFileSizeLimitError {
            path: path.to_path_buf(),
            max_bytes,
            actual_bytes: Some(metadata.len()),
        }
        .into());
    }

    let file = std::fs::File::open(path)?;
    let mut reader = file.take(max_bytes_u64.saturating_add(1));
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(TextFileSizeLimitError {
            path: path.to_path_buf(),
            max_bytes,
            actual_bytes: None,
        }
        .into());
    }

    Ok(String::from_utf8(bytes)?)
}

fn ensure_cycle_report_artifact_text_total_within_limit(
    artifact: &CycleReportArtifact,
    report_dir: &Path,
    max_bytes: usize,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let total_bytes = artifact.report_json.len()
        + artifact.raw_json.as_ref().map_or(0, String::len)
        + artifact.audit_json.as_ref().map_or(0, String::len)
        + artifact.context_markdown.as_ref().map_or(0, String::len);
    if total_bytes > max_bytes {
        return Err(TextFileSizeLimitError {
            path: report_dir.to_path_buf(),
            max_bytes,
            actual_bytes: Some(total_bytes as u64),
        }
        .into());
    }
    Ok(())
}

fn is_text_file_size_limit_error(error: &(dyn Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(error) = current {
        if error.downcast_ref::<TextFileSizeLimitError>().is_some() {
            return true;
        }
        current = error.source();
    }
    false
}

fn is_cycle_report_alias_index_malformed_error(error: &(dyn Error + 'static)) -> bool {
    error.to_string().starts_with("aliases.json malformed:")
}

fn validate_cycle_report_id(cycle_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let trimmed = cycle_id.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.chars().any(char::is_control)
    {
        return Err(format!("unsafe cycle report id: {cycle_id}").into());
    }
    Ok(())
}

fn validate_cycle_report_artifact_file_name(
    file_name: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if CYCLE_REPORT_DIRECT_ARTIFACT_FILES.contains(&file_name) {
        Ok(())
    } else {
        Err(format!("unsupported cycle-report artifact file: {file_name}").into())
    }
}

macro_rules! render_cycle_report_index_html_impl {
    ($latest_json:expr, $aliases_json:expr) => {{
    let latest_json = $latest_json;
    let aliases_json = $aliases_json;
    let cycle_ids = latest_json.map_or_else(Vec::new, extract_cycle_ids_from_latest_json);
    let list_html = if cycle_ids.is_empty() {
        r#"<div class="empty">latest.json에서 cycle_id 목록을 찾지 못했습니다. 직접 URL 입력을 사용하세요.</div>"#
            .to_owned()
    } else {
        cycle_ids
            .iter()
            .map(|cycle_id| {
                format!(
                    r#"<a class="report-link" href="/reports/{}/">
        <strong>{}</strong>
        <span>artifact viewer 열기</span>
      </a>"#,
                    url_component_encode(cycle_id),
                    html_escape(cycle_id)
                )
            })
            .collect::<Vec<_>>()
            .join("\n      ")
    };
    let (alias_entries, alias_error) = match aliases_json {
        Some(json) => match extract_cycle_alias_entries(json) {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(error)),
        },
        None => (Vec::new(), None),
    };
    let alias_list_html = if let Some(error) = &alias_error {
        format!(
            r#"<div class="empty">aliases.json 오류: {}</div>"#,
            html_escape(error)
        )
    } else if alias_entries.is_empty() {
        r#"<div class="empty">aliases.json에서 별칭 목록을 찾지 못했습니다.</div>"#
            .to_owned()
    } else {
        alias_entries
            .iter()
            .map(|entry| {
                let title = entry.title.as_deref().unwrap_or(&entry.id);
                format!(
                    r#"<a class="report-link" href="/reports/by-alias/{}/">
        <strong>{}</strong>
        <span>{}</span>
        <span>canonical: {}</span>
      </a>"#,
                    url_component_encode(&entry.alias),
                    html_escape(&entry.alias),
                    html_escape(title),
                    html_escape(&entry.id)
                )
            })
            .collect::<Vec<_>>()
            .join("\n      ")
    };
    let latest_status = latest_json.map_or(
        "latest.json 없음. cycle-report artifact가 만들어진 뒤 수동 새로고침하세요.".to_owned(),
        |json| format!("latest.json 로드됨 · {} bytes", json.len()),
    );
    let aliases_status = aliases_json.map_or(
        "aliases.json 없음. 별칭 예약 뒤 수동 새로고침하세요.".to_owned(),
        |json| {
            alias_error.as_ref().map_or_else(
                || format!("aliases.json 로드됨 · {} bytes", json.len()),
                |error| format!("aliases.json 오류 · {error}"),
            )
        },
    );
    let latest_raw = latest_json.map_or_else(String::new, html_escape);
    let aliases_raw = aliases_json.map_or_else(String::new, html_escape);
    let mut html = r##"<!doctype html>
<html lang="ko">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Xavi Cycle Reports</title>
<style>
:root {
  color-scheme: light;
  --bg: #f4f5f7;
  --ink: #20242b;
  --muted: #646b76;
  --line: #d9dee6;
  --panel: #ffffff;
  --accent: #146c5f;
  --accent-soft: #eaf7f3;
  --warn-soft: #fff6e3;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  min-height: 100vh;
  background: var(--bg);
  color: var(--ink);
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 14px;
  letter-spacing: 0;
}
.topbar {
  min-height: 58px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 20px;
  border-bottom: 1px solid var(--line);
  background: #ffffff;
}
.brand { display: grid; gap: 2px; min-width: 0; }
.brand strong { font-size: 17px; }
.brand span { color: var(--muted); font-size: 12px; overflow-wrap: anywhere; }
.actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
.btn {
  min-height: 34px;
  display: inline-flex;
  align-items: center;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  color: var(--ink);
  padding: 0 12px;
  text-decoration: none;
  cursor: pointer;
}
.btn.primary { background: var(--accent); border-color: var(--accent); color: #ffffff; }
.shell { width: min(1040px, calc(100% - 32px)); margin: 0 auto; padding: 24px 0 36px; display: grid; gap: 20px; }
.section { display: grid; gap: 10px; }
.section h1 { margin: 0; font-size: 30px; letter-spacing: 0; }
.section h2 { margin: 0; font-size: 18px; }
.panel {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--panel);
  padding: 14px;
  display: grid;
  gap: 10px;
}
.status { color: var(--muted); line-height: 1.6; }
.manual-row { display: flex; gap: 8px; flex-wrap: wrap; }
input {
  min-height: 36px;
  flex: 1 1 260px;
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 0 10px;
}
.report-list { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); gap: 10px; }
.report-link {
  min-height: 92px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  color: var(--ink);
  padding: 12px;
  display: grid;
  align-content: start;
  gap: 8px;
  text-decoration: none;
}
.report-link span { color: var(--muted); font-size: 12px; }
.empty {
  min-height: 92px;
  display: grid;
  place-items: center;
  border: 1px dashed var(--line);
  border-radius: 6px;
  background: #ffffff;
  color: var(--muted);
  text-align: center;
  padding: 14px;
}
details {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  padding: 12px;
}
summary { cursor: pointer; font-weight: 700; }
pre {
  margin: 12px 0 0;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  background: #f8f9fa;
  border-radius: 6px;
  padding: 12px;
}
@media (max-width: 720px) {
  .topbar { align-items: flex-start; flex-direction: column; padding: 12px 14px; }
  .actions { justify-content: flex-start; }
}
</style>
</head>
<body>
<header class="topbar">
  <div class="brand">
    <strong>Xavi cycle reports</strong>
    <span>cycle-report artifact viewer · polling 없음 · 직접 URL/수동 새로고침</span>
  </div>
  <nav class="actions" aria-label="보고서 이동">
    <a class="btn" href="/">개발 콘솔</a>
    <a class="btn" href="/api/reports">latest.json API</a>
    <button class="btn primary" type="button" id="manual-refresh">수동 새로고침</button>
  </nav>
</header>
<main class="shell">
  <section class="section">
    <h1>사이클 보고서</h1>
    <div class="panel">
      <div class="status">__LATEST_STATUS__</div>
      <div class="manual-row">
        <input id="cycle-id-input" type="text" placeholder="cycle_id 직접 입력">
        <button class="btn primary" type="button" id="open-cycle">열기</button>
      </div>
    </div>
  </section>
  <section class="section">
    <h2>latest.json 목록</h2>
    <div class="report-list">
      __REPORT_LIST__
    </div>
  </section>
  <section class="section">
    <h2>aliases.json 별칭 목록</h2>
    <div class="panel">
      <div class="status">__ALIASES_STATUS__</div>
    </div>
    <div class="report-list">
      __ALIAS_LIST__
    </div>
  </section>
  <details>
    <summary>latest.json 원문</summary>
    <pre>__LATEST_RAW__</pre>
  </details>
  <details>
    <summary>aliases.json 원문</summary>
    <pre>__ALIASES_RAW__</pre>
  </details>
</main>
<script>
function openCycleReport() {
  const value = document.getElementById('cycle-id-input').value.trim();
  if (!value) return;
  window.location.href = `/reports/${encodeURIComponent(value)}/`;
}
document.getElementById('open-cycle').addEventListener('click', openCycleReport);
document.getElementById('cycle-id-input').addEventListener('keydown', event => {
  if (event.key === 'Enter') openCycleReport();
});
document.getElementById('manual-refresh').addEventListener('click', () => {
  window.location.reload();
});
</script>
</body>
</html>"##
        .to_owned();
    html = html.replace("__LATEST_STATUS__", &html_escape(&latest_status));
    html = html.replace("__ALIASES_STATUS__", &html_escape(&aliases_status));
    html = html.replace("__REPORT_LIST__", &list_html);
    html = html.replace("__ALIAS_LIST__", &alias_list_html);
    html = html.replace("__LATEST_RAW__", &latest_raw);
    html.replace("__ALIASES_RAW__", &aliases_raw)
    }};
}

/// Renders the manual cycle-report artifact index page.
#[must_use]
pub fn render_cycle_report_index_html(latest_json: Option<&str>) -> String {
    render_cycle_report_index_html_impl!(latest_json, None)
}

/// Renders the manual cycle-report artifact index page with alias links.
#[must_use]
pub fn render_cycle_report_index_html_with_aliases(
    latest_json: Option<&str>,
    aliases_json: Option<&str>,
) -> String {
    render_cycle_report_index_html_impl!(latest_json, aliases_json)
}

/// Renders a Korean HTML view for a cycle-report artifact bundle.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render_cycle_report_artifact_html(artifact: &CycleReportArtifact) -> String {
    let summary = CycleReportArtifactSummary::from_artifact(artifact);
    let status_label = cycle_report_status_label(&summary.status);
    let cycle_alias_label = summary.cycle_alias.as_deref().unwrap_or("별칭 없음").to_owned();
    let cycle_title_label = summary.cycle_title.as_deref().unwrap_or("제목 없음").to_owned();
    let cycle_category_label =
        summary.cycle_category.as_deref().unwrap_or("category 없음").to_owned();
    let cycle_category_key_label =
        summary.cycle_category_key.as_deref().unwrap_or("category_key 없음").to_owned();
    let cycle_sequence_label =
        summary.cycle_sequence.as_deref().unwrap_or("sequence 없음").to_owned();
    let audit_reliability_label =
        audit_reliability_label(summary.audit_status.as_deref(), &summary.evidence_status);
    let audit_reliability_description =
        audit_reliability_description(summary.audit_status.as_deref(), &summary.evidence_status);
    let evidence_warning = render_evidence_status_warning(&summary);
    let workflow_map = render_cycle_report_workflow_map(&summary);
    let user_request_verbatim = render_verbatim_evidence_block(
        &summary.user_request_verbatim,
        "사용자 요청 원문 증거 없음",
    );
    let user_request_evidence_warning = render_missing_user_request_evidence_warning(&summary);
    let full_verbatim_evidence = render_full_verbatim_evidence_index(artifact);
    let orchestra_delegations = render_orchestra_delegations(&artifact.report_json, &summary);
    let role_board =
        render_cycle_report_role_board(&artifact.report_json, artifact.raw_json.as_deref());
    let command_evidence = render_command_evidence_view(artifact);
    let code_change_index =
        render_code_changes_index_view(&artifact.report_json, &artifact.cycle_id);
    let derived_summaries = render_derived_summaries(&artifact.report_json);
    let evidence_audit = render_evidence_audit(artifact);
    let raw_details = render_artifact_details(artifact);
    let mut html = r##"<!doctype html>
<html lang="ko">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Xavi Cycle Report</title>
<style>
:root {
  color-scheme: light;
  --bg: #f4f5f7;
  --ink: #20242b;
  --muted: #646b76;
  --line: #d9dee6;
  --panel: #ffffff;
  --accent: #146c5f;
  --accent-soft: #eaf7f3;
  --blue: #2d5c88;
  --blue-soft: #eef4fb;
  --warn: #94610b;
  --warn-soft: #fff6e3;
  --danger: #9f3434;
  --danger-soft: #fff1f1;
  --good: #186b3b;
  --good-soft: #eef8f1;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  min-height: 100vh;
  background: var(--bg);
  color: var(--ink);
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 14px;
  letter-spacing: 0;
}
a { color: inherit; }
.topbar {
  min-height: 58px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 20px;
  border-bottom: 1px solid var(--line);
  background: #ffffff;
}
.brand { display: grid; gap: 2px; min-width: 0; }
.brand strong { font-size: 17px; }
.brand span { color: var(--muted); font-size: 12px; overflow-wrap: anywhere; }
.actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
.btn {
  min-height: 34px;
  display: inline-flex;
  align-items: center;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  color: var(--ink);
  padding: 0 12px;
  text-decoration: none;
  cursor: pointer;
}
.btn.primary { background: var(--accent); border-color: var(--accent); color: #ffffff; }
.btn.text-expand-button {
  justify-self: start;
  min-height: 30px;
  padding: 0 10px;
  border-color: #b8cce6;
  background: var(--blue-soft);
  color: var(--blue);
  font-size: 12px;
}
.modal-enhanced .clampable {
  position: relative;
  max-height: 9.7em;
  overflow: hidden;
}
.modal-enhanced pre.clampable,
.modal-enhanced .evidence-text.clampable {
  max-height: 13.2em;
}
.modal-enhanced .clampable.is-clamped::after {
  content: "...";
  position: absolute;
  right: 0;
  bottom: 0;
  min-width: 58px;
  padding-left: 34px;
  background: linear-gradient(to right, rgba(255,255,255,0), #ffffff 42%);
  color: var(--muted);
  text-align: right;
}
.text-modal {
  width: min(1040px, calc(100vw - 28px));
  max-height: calc(100vh - 28px);
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #ffffff;
  color: var(--ink);
}
.text-modal::backdrop { background: rgba(32,36,43,0.56); }
.text-modal-frame {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  max-height: calc(100vh - 28px);
}
.text-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px;
  border-bottom: 1px solid var(--line);
}
.text-modal-header h2 {
  margin: 0;
  font-size: 16px;
  line-height: 1.35;
  overflow-wrap: anywhere;
}
.text-modal-body {
  margin: 0;
  max-height: min(72vh, 720px);
  border-radius: 0;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.report-shell {
  width: calc(100% - 32px);
  margin: 0 auto;
  padding: 22px 0 38px;
  display: grid;
  gap: 22px;
}
.hero {
  display: grid;
  gap: 12px;
  padding: 8px 0 20px;
  border-bottom: 1px solid var(--line);
}
.hero h1 { margin: 0; font-size: 34px; line-height: 1.08; letter-spacing: 0; }
.hero p { margin: 0; color: #3b4048; line-height: 1.65; max-width: 920px; }
.status-strip {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}
.metric {
  min-height: 92px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  padding: 12px;
  display: grid;
  align-content: start;
  gap: 8px;
}
.metric strong { font-size: 20px; }
.metric span { color: var(--muted); font-size: 12px; }
.metric p { margin: 0; color: #3b4048; line-height: 1.5; overflow-wrap: anywhere; }
.section { display: grid; gap: 10px; }
.section h2 { margin: 0; font-size: 18px; }
.sticky-nav {
  position: sticky;
  top: 0;
  z-index: 5;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: rgba(255,255,255,0.96);
}
.sticky-nav a {
  min-height: 30px;
  display: inline-flex;
  align-items: center;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 6px;
  text-decoration: none;
  background: #ffffff;
  color: var(--ink);
}
.section-intro { margin: 0; color: var(--muted); line-height: 1.55; }
.grid-two { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.grid-many { display: grid; grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); gap: 10px; }
.record-list { display: grid; gap: 10px; width: 100%; }
.report-card,
.flow-card,
.role-card {
  min-height: 124px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--panel);
  padding: 12px;
  display: grid;
  align-content: start;
  gap: 8px;
}
.report-card span,
.flow-card span,
.role-card span { color: var(--muted); font-size: 12px; }
.report-card strong,
.flow-card strong,
.role-card strong { color: var(--ink); line-height: 1.35; }
.report-card p,
.flow-card p,
.role-card p { margin: 0; color: #3b4048; line-height: 1.58; overflow-wrap: anywhere; white-space: pre-wrap; }
.user-request-card {
  min-height: 0;
  width: 100%;
}
.warning-banner {
  border: 1px solid #e0b36a;
  border-radius: 6px;
  background: var(--warn-soft);
  color: #6f4700;
  padding: 10px 12px;
  line-height: 1.55;
}
.warning-banner strong,
.warning-banner p { margin: 0; }
.warning-banner p + p,
.warning-banner details { margin-top: 8px; }
.derived-label {
  color: var(--warn);
  font-size: 12px;
  font-weight: 700;
}
.derived-card {
  display: grid;
  gap: 6px;
  padding: 10px;
  border: 1px solid #edd9b5;
  border-radius: 6px;
  background: var(--warn-soft);
}
.evidence-details {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #fbfcfd;
  padding: 10px;
}
.evidence-details summary {
  cursor: pointer;
  font-weight: 700;
}
.evidence-details pre {
  margin-top: 10px;
}
.verbatim-evidence {
  display: grid;
  gap: 10px;
  min-width: 0;
}
.evidence-text {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  word-break: break-word;
}
.evidence-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}
.metadata-table {
  width: 100%;
  margin-top: 10px;
  border-collapse: collapse;
  table-layout: fixed;
}
.metadata-table th,
.metadata-table td {
  border-top: 1px solid var(--line);
  padding: 8px;
  text-align: left;
  vertical-align: top;
  overflow-wrap: anywhere;
}
.metadata-table th {
  width: 180px;
  color: var(--muted);
  font-weight: 700;
}
.technical-details {
  width: 100%;
}
.audit-panel {
  display: grid;
  gap: 10px;
}
.audit-readable {
  border: 1px solid #e0b36a;
  border-radius: 6px;
  background: var(--warn-soft);
  padding: 12px;
  display: grid;
  gap: 8px;
}
.audit-readable strong { font-size: 18px; }
.audit-readable p { margin: 0; color: #3b4048; line-height: 1.58; overflow-wrap: anywhere; }
.audit-fields {
  display: grid;
  gap: 10px;
}
.audit-diagnostic-details {
  border-color: #b8cce6;
  background: var(--blue-soft);
}
.audit-diagnostic-details summary {
  color: var(--blue);
}
.audit-diagnostic-details p {
  margin: 8px 0 0;
  color: #3b4048;
  line-height: 1.58;
  overflow-wrap: anywhere;
}
.delegation-list { display: grid; gap: 10px; }
.delegation-card {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  padding: 12px;
  display: grid;
  gap: 10px;
}
.delegation-card h3 { margin: 0; font-size: 15px; }
.diff-file {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  overflow: hidden;
}
.diff-file + .diff-file { margin-top: 12px; }
.diff-file-header {
  display: grid;
  gap: 8px;
  padding: 12px;
  border-bottom: 1px solid var(--line);
  background: #fbfcfd;
}
.diff-file-title { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.diff-file-title strong { overflow-wrap: anywhere; }
.diff-meta { display: flex; gap: 6px; flex-wrap: wrap; }
.diff-pill {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  border: 1px solid var(--line);
  border-radius: 999px;
  padding: 0 8px;
  color: var(--muted);
  background: #ffffff;
  font-size: 12px;
}
.diff-summary { margin: 0; color: #3b4048; line-height: 1.55; overflow-wrap: anywhere; }
.diff-hunk { display: grid; gap: 8px; padding: 12px; border-top: 1px solid var(--line); }
.diff-hunk:first-of-type { border-top: 0; }
.diff-hunk-heading {
  display: flex;
  gap: 8px;
  align-items: baseline;
  flex-wrap: wrap;
  color: var(--blue);
}
.diff-hunk-heading code { color: var(--blue); }
.diff-hunk-summary,
.diff-explanations { margin: 0; color: #3b4048; line-height: 1.55; overflow-wrap: anywhere; }
.diff-explanations { display: grid; gap: 6px; padding: 8px 10px; background: var(--blue-soft); border-radius: 6px; }
.diff-table { display: grid; border: 1px solid var(--line); border-radius: 6px; overflow: hidden; }
.diff-line {
  display: grid;
  grid-template-columns: 64px 64px minmax(0, 1fr);
  min-height: 28px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  line-height: 1.5;
}
.diff-line span { padding: 4px 8px; border-right: 1px solid rgba(0,0,0,0.06); }
.diff-line code { padding: 4px 8px; white-space: pre-wrap; overflow-wrap: anywhere; background: transparent; }
.diff-line.add { background: var(--good-soft); }
.diff-line.remove { background: var(--danger-soft); }
.diff-line.context { background: #ffffff; }
.diff-line .line-no { color: var(--muted); text-align: right; user-select: none; }
.status-success { border-color: #cfe2d5; background: var(--good-soft); }
.status-failed { border-color: #e7c6c6; background: var(--danger-soft); }
.status-blocked,
.status-stopped { border-color: #edd9b5; background: var(--warn-soft); }
details {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  padding: 12px;
}
summary { cursor: pointer; font-weight: 700; }
pre {
  margin: 12px 0 0;
  overflow: auto;
  white-space: pre;
  background: #f8f9fa;
  border-radius: 6px;
  padding: 12px;
  line-height: 1.55;
}
pre.evidence-text {
  margin: 0;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.missing { color: var(--muted); }
@media (max-width: 860px) {
  .topbar { align-items: flex-start; flex-direction: column; padding: 12px 14px; }
  .actions { justify-content: flex-start; }
  .status-strip,
  .grid-two { grid-template-columns: 1fr; }
}
</style>
</head>
<body>
<header class="topbar">
  <div class="brand">
    <strong>Xavi cycle report</strong>
    <span>__CYCLE_ID__ · __CYCLE_ALIAS__ · cycle-report artifact · polling 없음</span>
  </div>
  <nav class="actions" aria-label="보고서 이동">
    <a class="btn" href="/reports">보고서 목록</a>
    <a class="btn" href="/">개발 콘솔</a>
    <a class="btn primary" href="/api/reports/__CYCLE_ID_URL__/report.json">report.json</a>
  </nav>
</header>
<main class="report-shell">
  <section class="hero">
    <h1>사이클 전체 evidence 보고서</h1>
    <p>cycle 종료 직후 생성된 공개 artifact의 사용자 요청 원문, 보고서 신뢰성 검증 상태, 역할별 지시/반환, 명령 evidence를 기본 화면에 그대로 표시합니다.</p>
    <p>전체 diff hunk는 이 화면에 렌더링하지 않고 전용 prebuilt artifact인 diff.html에서 확인합니다.</p>
    <p>요약과 changed_files는 derived/display 보조 정보이며, 원문이 없으면 복원하지 않고 원문 증거 상태를 그대로 보여줍니다.</p>
    __EVIDENCE_STATUS_WARNING__
  </section>

  <nav class="sticky-nav" aria-label="보고서 anchor navigation">
    <a href="#status-title">상태</a>
    <a href="#evidence-title">사용자 요청</a>
    <a href="#workflow-title">1회 작업 사이클 과정</a>
    <a href="#delegations-title">역할 지시 원문</a>
    <a href="#roles-title">역할 반환 원문</a>
    <a href="#verification-title">command evidence</a>
    <a href="#files-title">changed_files</a>
    <a href="#diff-title">diff.html</a>
    <a href="#raw-title">raw/context</a>
  </nav>

  <section class="section" aria-labelledby="status-title">
    <h2 id="status-title">사이클 상태</h2>
    <div class="status-strip">
      <article class="metric __STATUS_CLASS__"><span>상태</span><strong>__STATUS_LABEL__</strong></article>
      <article class="metric"><span>cycle_id</span><strong>__CYCLE_ID__</strong></article>
      <article class="metric"><span>cycle_alias</span><strong>__CYCLE_ALIAS__</strong></article>
      <article class="metric">
        <span>보고서 신뢰성 검증</span>
        <strong>__AUDIT_RELIABILITY_LABEL__</strong>
        <p>__AUDIT_RELIABILITY_DESCRIPTION__</p>
      </article>
    </div>
    <div class="grid-many">
      <article class="report-card"><span>cycle_title</span><strong>__CYCLE_TITLE__</strong></article>
      <article class="report-card"><span>cycle_category</span><strong>__CYCLE_CATEGORY__</strong></article>
      <article class="report-card"><span>cycle_category_key</span><strong>__CYCLE_CATEGORY_KEY__</strong></article>
      <article class="report-card"><span>cycle_sequence</span><strong>__CYCLE_SEQUENCE__</strong></article>
      <article class="report-card"><span>evidence_status</span><strong>__EVIDENCE_STATUS__</strong></article>
    </div>
  </section>

  <section class="section" aria-labelledby="evidence-title">
    <h2 id="evidence-title">사용자 요청 원문</h2>
    <p class="section-intro">사용자가 실제로 요청한 원문을 먼저 전체 폭으로 보여줍니다. source_ref, hash, timestamp 같은 검증 메타데이터는 본문 읽기를 방해하지 않도록 아래 접힘 표에 보존합니다.</p>
    __USER_REQUEST_EVIDENCE_WARNING__
    <div class="record-list">
      <article class="report-card user-request-card">
        <span>원문 evidence</span>
        <strong>user_request_verbatim</strong>
        __USER_REQUEST_VERBATIM__
      </article>
      <article class="report-card">
        <span class="derived-label">derived summary, not verbatim</span>
        <strong>user_request_display_summary_ko</strong>
        <p>__USER_REQUEST_DISPLAY_SUMMARY_KO__</p>
      </article>
      <article class="report-card">
        <span>결과 요약</span>
        <strong>result</strong>
        <p>__RESULT_SUMMARY__</p>
      </article>
    </div>
    __FULL_VERBATIM_EVIDENCE__
  </section>

  <section class="section" aria-labelledby="workflow-title">
    <h2 id="workflow-title">1회 작업 사이클 과정</h2>
    <p class="section-intro">이 구역은 derived/display 보조 정보입니다. 원문 지시와 반환은 위 evidence 구역과 아래 역할 반환 원문에서 확인합니다.</p>
    <div class="grid-many">
      __WORKFLOW_MAP__
    </div>
  </section>

  <section class="section" aria-labelledby="delegations-title">
    <h2 id="delegations-title">역할 지시 원문</h2>
    <p class="section-intro">역할별 지시는 prompt_verbatim 원문 evidence와 표시용 파생 요약을 분리합니다. 원문이 없으면 복원하지 않고 missing evidence로 표시합니다.</p>
    <div class="delegation-list">
      __ORCHESTRA_DELEGATIONS__
    </div>
  </section>

  <section class="section" aria-labelledby="roles-title">
    <h2 id="roles-title">역할 반환 원문</h2>
    <div class="grid-many">
      __ROLE_BOARD__
    </div>
  </section>

  <section class="section" aria-labelledby="failure-title">
    <h2 id="failure-title">실패 분석</h2>
    <div class="grid-two">
      <article class="report-card status-failed">
        <span>실패 지점</span>
        <strong>failure_point</strong>
        <p>__FAILURE_POINT__</p>
      </article>
      <article class="report-card">
        <span class="derived-label">derived summary, not verbatim</span>
        <strong>orchestra_instruction</strong>
        <p>__ORCHESTRA_INSTRUCTION__</p>
      </article>
    </div>
  </section>

  <section class="section" aria-labelledby="verification-title">
    <h2 id="verification-title">테스트 명령/결과 원문 evidence</h2>
    __COMMAND_EVIDENCE__
    <article class="report-card">
      <span class="derived-label">derived/display summary, not command output</span>
      <strong>검증 결과 요약</strong>
      <p>__VERIFICATION_RESULT__</p>
    </article>
  </section>

  <section class="section" aria-labelledby="files-title">
    <h2 id="files-title">전체 변경 기록</h2>
    <article class="report-card">
      <span class="derived-label">derived/display index, not diff hunk</span>
      <strong>changed_files 보조 색인</strong>
      <p>__CHANGED_FILES__</p>
    </article>
  </section>

  <section class="section" aria-labelledby="diff-title">
    <h2 id="diff-title">Diff 전용 artifact 색인</h2>
    __CODE_CHANGE_INDEX__
  </section>

  <section class="section" aria-labelledby="derived-title">
    <h2 id="derived-title">파생 요약</h2>
    __DERIVED_SUMMARIES__
  </section>

  <section class="section" aria-labelledby="evidence-audit-title">
    <h2 id="evidence-audit-title">보고서 신뢰성 검증</h2>
    __EVIDENCE_AUDIT__
  </section>

  <section class="section" aria-labelledby="raw-title">
    <h2 id="raw-title">원본 raw/audit/context</h2>
    __RAW_DETAILS__
  </section>
</main>
<dialog id="report-text-modal" class="text-modal" aria-labelledby="report-text-modal-title">
  <form method="dialog" class="text-modal-frame">
    <header class="text-modal-header">
      <h2 id="report-text-modal-title">원문 크게 보기</h2>
      <button class="btn" type="submit" data-modal-close>닫기</button>
    </header>
    <pre id="report-text-modal-body" class="text-modal-body"></pre>
  </form>
</dialog>
<script>
(() => {
  const dialog = document.getElementById('report-text-modal');
  const title = document.getElementById('report-text-modal-title');
  const body = document.getElementById('report-text-modal-body');
  if (!dialog || !title || !body || typeof dialog.showModal !== 'function') return;

  document.documentElement.classList.add('modal-enhanced');
  let returnFocus = null;
  const selectors = [
    '.user-request-card .evidence-text',
    '.verbatim-evidence > .evidence-text',
    '.card > p',
    '.card > pre',
    '.card .evidence-details pre',
    '.flow-card > p',
    '.report-card > p',
    '.report-card > pre',
    '.report-card .derived-card p',
    '.report-card .evidence-details pre',
    '.role-card > p',
    '.role-card > pre',
    '.role-card .derived-card p',
    '.role-card .evidence-details pre',
    '.delegation-card > p',
    '.delegation-card > pre',
    '.delegation-card .derived-card p',
    '.delegation-card .evidence-details pre',
    '.evidence-field .evidence-text',
    '.technical-details pre',
    'details > pre'
  ];

  const clampTargets = Array.from(document.querySelectorAll(selectors.join(',')))
    .filter((target, index, all) => all.indexOf(target) === index)
    .filter((target) => !target.closest('.diff-file, .diff-table, .diff-line'));

  const labelFor = (target) => {
    const owner = target.closest('.card, .report-card, .role-card, .delegation-card, .evidence-field, details, article, section');
    const label = owner?.querySelector('h3, strong, summary, span')?.textContent?.trim();
    return label || '원문 크게 보기';
  };

  const openModal = (target, trigger) => {
    returnFocus = trigger;
    title.textContent = labelFor(target);
    body.textContent = target.textContent || '';
    dialog.showModal();
  };

  dialog.addEventListener('click', (event) => {
    if (event.target === dialog) dialog.close();
  });
  dialog.addEventListener('close', () => {
    if (returnFocus) returnFocus.focus({ preventScroll: true });
    returnFocus = null;
  });

  clampTargets.forEach((target) => target.classList.add('clampable'));
  requestAnimationFrame(() => {
    clampTargets.forEach((target) => {
      const text = target.textContent || '';
      const looksLong = text.length > 280 || text.split('\n').length > 7;
      const overflows = looksLong || target.scrollHeight > target.clientHeight + 2 || target.scrollWidth > target.clientWidth + 2;
      if (!overflows) return;
      target.classList.add('is-clamped');
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'btn text-expand-button';
      button.textContent = '원문 크게 보기';
      button.setAttribute('aria-label', `${labelFor(target)} 원문 크게 보기`);
      button.addEventListener('click', () => openModal(target, button));
      target.insertAdjacentElement('afterend', button);
    });
  });
})();
</script>
</body>
</html>"##
        .to_owned();
    html = html.replace("__CYCLE_ID__", &html_escape(&artifact.cycle_id));
    html = html.replace("__CYCLE_ID_URL__", &url_component_encode(&artifact.cycle_id));
    html = html.replace("__CYCLE_ALIAS__", &html_escape(&cycle_alias_label));
    html = html.replace("__CYCLE_TITLE__", &html_escape(&cycle_title_label));
    html = html.replace("__CYCLE_CATEGORY__", &html_escape(&cycle_category_label));
    html = html.replace("__CYCLE_CATEGORY_KEY__", &html_escape(&cycle_category_key_label));
    html = html.replace("__CYCLE_SEQUENCE__", &html_escape(&cycle_sequence_label));
    html = html.replace("__STATUS_CLASS__", cycle_report_status_class(&summary.status));
    html = html.replace("__STATUS_LABEL__", &html_escape(&status_label));
    html = html.replace("__EVIDENCE_STATUS__", &html_escape(&summary.evidence_status));
    html = html.replace("__AUDIT_RELIABILITY_LABEL__", &html_escape(audit_reliability_label));
    html = html
        .replace("__AUDIT_RELIABILITY_DESCRIPTION__", &html_escape(audit_reliability_description));
    html = html.replace("__EVIDENCE_STATUS_WARNING__", &evidence_warning);
    html = html.replace("__USER_REQUEST_EVIDENCE_WARNING__", &user_request_evidence_warning);
    html = html.replace("__USER_REQUEST_VERBATIM__", &user_request_verbatim);
    html = html.replace(
        "__USER_REQUEST_DISPLAY_SUMMARY_KO__",
        &html_escape(&summary.user_request_display_summary_ko),
    );
    html = html.replace("__RESULT_SUMMARY__", &html_escape(&summary.result_summary));
    html = html.replace("__WORKFLOW_MAP__", &workflow_map);
    html = html.replace("__FULL_VERBATIM_EVIDENCE__", &full_verbatim_evidence);
    html = html.replace("__ORCHESTRA_DELEGATIONS__", &orchestra_delegations);
    html = html.replace("__ROLE_BOARD__", &role_board);
    html = html.replace("__COMMAND_EVIDENCE__", &command_evidence);
    html = html.replace("__FAILURE_POINT__", &html_escape(&summary.failure_point));
    html = html.replace("__ORCHESTRA_INSTRUCTION__", &html_escape(&summary.orchestra_instruction));
    html = html.replace("__VERIFICATION_RESULT__", &html_escape(&summary.verification_result));
    html = html.replace("__CHANGED_FILES__", &html_escape(&summary.changed_files));
    html = html.replace("__CODE_CHANGE_INDEX__", &code_change_index);
    html = html.replace("__DERIVED_SUMMARIES__", &derived_summaries);
    html = html.replace("__EVIDENCE_AUDIT__", &evidence_audit);
    html.replace("__RAW_DETAILS__", &raw_details)
}

/// Renders the prebuilt diff-only cycle-report artifact.
///
/// The report server serves this file directly from `diff.html`; it must not synthesize it from
/// `report.json` on request.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render_cycle_report_diff_html(artifact: &CycleReportArtifact) -> String {
    let summary = CycleReportArtifactSummary::from_artifact(artifact);
    let evidence_warning = render_evidence_status_warning(&summary);
    let file_nav = render_code_changes_diff_nav(&artifact.report_json);
    let code_changes = render_code_changes_diff_view(&artifact.report_json);
    let evidence_audit = render_evidence_audit(artifact);
    let mut html = r##"<!doctype html>
<html lang="ko">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Xavi Cycle Report Diff</title>
<style>
:root {
  color-scheme: light;
  --bg: #f5f6f8;
  --ink: #20242b;
  --muted: #646b76;
  --line: #d9dee6;
  --panel: #ffffff;
  --accent: #146c5f;
  --warn: #94610b;
  --warn-soft: #fff6e3;
  --danger-soft: #fff1f1;
  --good-soft: #eef8f1;
  --blue-soft: #eef4fb;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  min-height: 100vh;
  background: var(--bg);
  color: var(--ink);
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 14px;
  letter-spacing: 0;
}
a { color: inherit; }
.wrap-toggle {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
  pointer-events: none;
}
.topbar {
  min-height: 58px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 20px;
  border-bottom: 1px solid var(--line);
  background: #ffffff;
}
.brand { display: grid; gap: 2px; min-width: 0; }
.brand strong { font-size: 17px; }
.brand span { color: var(--muted); font-size: 12px; overflow-wrap: anywhere; }
.actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
.btn {
  min-height: 34px;
  display: inline-flex;
  align-items: center;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  color: var(--ink);
  padding: 0 12px;
  text-decoration: none;
  cursor: pointer;
}
.btn.primary { background: var(--accent); border-color: var(--accent); color: #ffffff; }
#wrap-toggle:checked ~ .topbar label[for="wrap-toggle"] {
  background: var(--accent);
  border-color: var(--accent);
  color: #ffffff;
}
.shell {
  width: calc(100vw - 24px);
  margin: 0 auto;
  padding: 18px 0 34px;
  display: grid;
  gap: 18px;
}
.hero {
  display: grid;
  gap: 10px;
  padding: 8px 0 16px;
  border-bottom: 1px solid var(--line);
}
.hero h1 { margin: 0; font-size: 30px; line-height: 1.12; letter-spacing: 0; }
.hero p { margin: 0; color: #3b4048; line-height: 1.6; max-width: 960px; }
.warning-banner {
  border: 1px solid #e0b36a;
  border-radius: 6px;
  background: var(--warn-soft);
  color: #6f4700;
  padding: 10px 12px;
  line-height: 1.55;
}
.sticky-nav {
  position: sticky;
  top: 0;
  z-index: 4;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: rgba(255,255,255,0.96);
}
.sticky-nav a {
  min-height: 30px;
  display: inline-flex;
  align-items: center;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 6px;
  text-decoration: none;
  background: #ffffff;
}
.section { display: grid; gap: 10px; }
.section h2 { margin: 0; font-size: 18px; }
.report-card {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--panel);
  padding: 12px;
  display: grid;
  gap: 8px;
}
.report-card span { color: var(--muted); font-size: 12px; }
.report-card p { margin: 0; color: #3b4048; line-height: 1.58; overflow-wrap: anywhere; }
.grid-two { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.diff-file {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  overflow: hidden;
}
.diff-file + .diff-file { margin-top: 12px; }
.diff-file-header {
  display: grid;
  gap: 8px;
  padding: 12px;
  border-bottom: 1px solid var(--line);
  background: #fbfcfd;
}
.diff-file-title { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.diff-file-title strong { overflow-wrap: anywhere; }
.diff-meta { display: flex; gap: 6px; flex-wrap: wrap; }
.diff-pill {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  border: 1px solid var(--line);
  border-radius: 999px;
  padding: 0 8px;
  color: var(--muted);
  background: #ffffff;
  font-size: 12px;
}
.diff-summary { margin: 0; color: #3b4048; line-height: 1.55; overflow-wrap: anywhere; }
.derived-label { color: var(--warn); font-size: 12px; font-weight: 700; }
.diff-hunk { display: grid; gap: 8px; padding: 12px; border-top: 1px solid var(--line); }
.diff-hunk:first-of-type { border-top: 0; }
.diff-hunk-heading {
  display: flex;
  gap: 8px;
  align-items: baseline;
  flex-wrap: wrap;
  color: #2d5c88;
}
.diff-hunk-summary,
.diff-explanations { margin: 0; color: #3b4048; line-height: 1.55; overflow-wrap: anywhere; }
.diff-explanations { display: grid; gap: 6px; padding: 8px 10px; background: var(--blue-soft); border-radius: 6px; }
.code-viewport {
  width: 100%;
  overflow-x: auto;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
}
.diff-table {
  display: grid;
  min-width: max-content;
}
.diff-line {
  display: grid;
  grid-template-columns: 72px 72px minmax(720px, max-content);
  min-height: 28px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  line-height: 1.5;
}
.diff-line span { padding: 4px 8px; border-right: 1px solid rgba(0,0,0,0.06); }
.diff-line code {
  padding: 4px 8px;
  white-space: pre;
  overflow-wrap: normal;
  background: transparent;
}
#wrap-toggle:checked ~ .shell .diff-line code {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.diff-line.add { background: var(--good-soft); }
.diff-line.remove { background: var(--danger-soft); }
.diff-line.context { background: #ffffff; }
.diff-line .line-no { color: var(--muted); text-align: right; user-select: none; }
pre {
  margin: 12px 0 0;
  overflow: auto;
  white-space: pre;
  background: #f8f9fa;
  border-radius: 6px;
  padding: 12px;
  line-height: 1.55;
}
@media (max-width: 860px) {
  .topbar { align-items: flex-start; flex-direction: column; padding: 12px 14px; }
  .actions { justify-content: flex-start; }
  .grid-two { grid-template-columns: 1fr; }
  .hero h1 { font-size: 25px; }
}
</style>
</head>
<body>
<input id="wrap-toggle" class="wrap-toggle" type="checkbox">
<header class="topbar">
  <div class="brand">
    <strong>Xavi cycle diff</strong>
    <span>__CYCLE_ID__ · diff.html · prebuilt artifact</span>
  </div>
  <nav class="actions" aria-label="diff artifact navigation">
    <a class="btn" href="/reports/__CYCLE_ID_URL__/">메인 report</a>
    <a class="btn" href="/api/reports/__CYCLE_ID_URL__/report.json">report.json</a>
    <label class="btn" for="wrap-toggle">줄바꿈</label>
    <a class="btn primary" href="#diff-files">diff hunk</a>
  </nav>
</header>
<main class="shell">
  <section class="hero">
    <h1>Diff 전용 artifact</h1>
    <p>report.json.code_changes[]의 파일, hunk, line evidence를 이 페이지에서만 전체 렌더링합니다. 메인 report는 이 페이지로 연결되는 색인만 제공합니다.</p>
    <p>기본은 <code>white-space: pre</code>와 horizontal scroll입니다. 줄바꿈 토글은 표시 방식만 바꾸며 원문 line content를 수정하지 않습니다.</p>
    __EVIDENCE_STATUS_WARNING__
  </section>
  <nav class="sticky-nav" aria-label="diff file and hunk navigation">
    __FILE_NAV__
  </nav>
  <section id="diff-files" class="section" aria-labelledby="diff-files-title">
    <h2 id="diff-files-title">파일별 diff hunk</h2>
    __CODE_CHANGES__
  </section>
  <section id="diff-audit" class="section" aria-labelledby="diff-audit-title">
    <h2 id="diff-audit-title">Evidence audit warning</h2>
    __EVIDENCE_AUDIT__
  </section>
</main>
</body>
</html>"##
        .to_owned();
    html = html.replace("__CYCLE_ID__", &html_escape(&artifact.cycle_id));
    html = html.replace("__CYCLE_ID_URL__", &url_component_encode(&artifact.cycle_id));
    html = html.replace("__EVIDENCE_STATUS_WARNING__", &evidence_warning);
    html = html.replace("__FILE_NAV__", &file_nav);
    html = html.replace("__CODE_CHANGES__", &code_changes);
    html.replace("__EVIDENCE_AUDIT__", &evidence_audit)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VerbatimEvidence {
    text: String,
    source_type: String,
    source_ref: String,
    role: String,
    agent_id: Option<String>,
    hash_sha256: String,
    timestamp: String,
    order: String,
}

impl VerbatimEvidence {
    fn from_object(object: &str) -> Result<Self, String> {
        let text = required_json_string_field(object, "text")?;
        let source_type = required_json_string_field(object, "source_type")?;
        let source_ref = required_json_string_field(object, "source_ref")?;
        let role = required_json_string_field(object, "role")?;
        let agent_id = required_json_nullable_string_field(object, "agent_id")?;
        let hash_sha256 = required_json_string_field(object, "hash_sha256")?;
        let expected_hash = trace_text_sha256_hex(&text);
        if hash_sha256 != expected_hash {
            return Err(format!(
                "hash_sha256 does not match sha256(text): expected {expected_hash}, got {hash_sha256}"
            ));
        }
        Ok(Self {
            text,
            source_type,
            source_ref,
            role,
            agent_id,
            hash_sha256,
            timestamp: required_json_string_field(object, "timestamp")?,
            order: required_json_non_negative_integer_display_field(object, "order")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VerbatimEvidenceStatus {
    Valid(VerbatimEvidence),
    Invalid(String),
    Missing,
}

impl VerbatimEvidenceStatus {
    fn valid(&self) -> Option<&VerbatimEvidence> {
        match self {
            Self::Valid(evidence) => Some(evidence),
            Self::Invalid(_) | Self::Missing => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CycleReportArtifactSummary {
    status: String,
    evidence_status: String,
    evidence_warning: Option<String>,
    audit_status: Option<String>,
    cycle_alias: Option<String>,
    cycle_category: Option<String>,
    cycle_category_key: Option<String>,
    cycle_sequence: Option<String>,
    cycle_title: Option<String>,
    user_request_verbatim: VerbatimEvidenceStatus,
    user_request_display_summary_ko: String,
    result_summary: String,
    orchestra_instruction: String,
    failure_point: String,
    verification_result: String,
    changed_files: String,
}

impl CycleReportArtifactSummary {
    fn from_artifact(artifact: &CycleReportArtifact) -> Self {
        let report = &artifact.report_json;
        let audit = artifact.audit_json.as_deref().unwrap_or("");
        Self {
            status: first_json_text_for_keys(
                report,
                &["cycle_status", "status", "outcome", "result"],
            )
            .or_else(|| first_json_text_for_keys(audit, &["status", "outcome", "result"]))
            .unwrap_or_else(|| "unknown".to_owned()),
            evidence_status: first_json_text_for_keys(report, &["evidence_status"])
                .or_else(|| first_json_text_for_keys(audit, &["evidence_status"]))
                .unwrap_or_else(|| "incomplete".to_owned()),
            evidence_warning: first_json_text_for_keys(report, &["evidence_warning", "warning"])
                .or_else(|| first_json_text_for_keys(audit, &["summary_ko", "warning"])),
            audit_status: json_value_field_snippet(report, "audit")
                .and_then(|audit| first_json_text_for_keys(&audit, &["status"]))
                .or_else(|| first_json_text_for_keys(audit, &["status"])),
            cycle_alias: json_text_field(report, "cycle_alias"),
            cycle_category: json_text_field(report, "cycle_category"),
            cycle_category_key: json_text_field(report, "cycle_category_key"),
            cycle_sequence: json_text_field(report, "cycle_sequence"),
            cycle_title: json_text_field(report, "cycle_title"),
            user_request_verbatim: verbatim_evidence_from_field(report, "user_request_verbatim"),
            user_request_display_summary_ko: first_json_text_for_keys(
                report,
                &["user_request_display_summary_ko"],
            )
            .or_else(|| {
                first_json_text_for_keys(
                    report,
                    &["user_request", "original_user_request", "request", "goal"],
                )
            })
            .unwrap_or_else(|| "report.json에 사용자 요청 파생 요약 필드가 없습니다.".to_owned()),
            result_summary: first_json_text_for_keys(
                report,
                &["result_summary", "summary", "result", "outcome_summary"],
            )
            .unwrap_or_else(|| "report.json에 결과 요약 필드가 없습니다.".to_owned()),
            orchestra_instruction: first_json_text_for_keys(
                report,
                &[
                    "orchestra_instruction",
                    "orchestra_directive",
                    "orchestra_instructions",
                    "orchestra_judgment",
                ],
            )
            .unwrap_or_else(|| "report.json에 orchestra 지시 필드가 없습니다.".to_owned()),
            failure_point: first_json_text_for_keys(
                report,
                &[
                    "failure_point",
                    "failed_at",
                    "failure",
                    "failure_analysis",
                    "blocked_reason",
                    "stop_reason",
                ],
            )
            .unwrap_or_else(|| "실패/중단/blocked 지점이 명시되지 않았습니다.".to_owned()),
            verification_result: first_json_text_for_keys(
                report,
                &["verification_result", "verification", "test_result", "test_results", "tests"],
            )
            .or_else(|| first_json_text_for_keys(audit, &["verification_result", "status"]))
            .unwrap_or_else(|| "검증 결과 artifact 필드가 없습니다.".to_owned()),
            changed_files: first_json_text_for_keys(
                report,
                &["changed_files", "changed_files_or_scope", "files", "file_changes"],
            )
            .unwrap_or_else(|| "변경 파일 보조 색인 artifact 필드가 없습니다.".to_owned()),
        }
    }
}

fn verbatim_evidence_from_field(input: &str, key: &str) -> VerbatimEvidenceStatus {
    let Some(value) = json_value_field_snippet(input, key) else {
        return VerbatimEvidenceStatus::Missing;
    };
    let value = value.trim();
    if value == "null" {
        return VerbatimEvidenceStatus::Missing;
    }
    match VerbatimEvidence::from_object(value) {
        Ok(evidence) => VerbatimEvidenceStatus::Valid(evidence),
        Err(reason) => VerbatimEvidenceStatus::Invalid(format!("{key}: {reason}")),
    }
}

fn required_json_string_field(object: &str, key: &str) -> Result<String, String> {
    let value =
        json_value_field_snippet(object, key).ok_or_else(|| format!("missing field {key}"))?;
    let parsed = parse_json_string_value(value.trim())
        .ok_or_else(|| format!("field {key} must be a JSON string"))?;
    if parsed.trim().is_empty() {
        return Err(format!("field {key} must not be empty"));
    }
    Ok(parsed)
}

fn required_json_nullable_string_field(object: &str, key: &str) -> Result<Option<String>, String> {
    let value =
        json_value_field_snippet(object, key).ok_or_else(|| format!("missing field {key}"))?;
    let value = value.trim();
    if value == "null" {
        return Ok(None);
    }
    let parsed = parse_json_string_value(value)
        .ok_or_else(|| format!("field {key} must be a JSON string or null"))?;
    if parsed.trim().is_empty() {
        return Err(format!("field {key} must not be empty"));
    }
    Ok(Some(parsed))
}

fn required_json_non_negative_integer_display_field(
    object: &str,
    key: &str,
) -> Result<String, String> {
    let value =
        json_value_field_snippet(object, key).ok_or_else(|| format!("missing field {key}"))?;
    let parsed =
        value.trim().parse::<i64>().map_err(|_| format!("field {key} must be a JSON integer"))?;
    if parsed < 0 {
        return Err(format!("field {key} must be non-negative"));
    }
    Ok(parsed.to_string())
}

fn first_json_text_for_keys(input: &str, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| json_value_field_snippet(input, key))
        .map(|value| json_value_to_display_text(&value))
}

fn json_value_to_display_text(value: &str) -> String {
    parse_json_string_value(value.trim()).unwrap_or_else(|| artifact_snippet(value.trim(), 2_400))
}

fn artifact_snippet(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_owned();
    }
    let mut clipped = trimmed.chars().take(limit.saturating_sub(3)).collect::<String>();
    clipped.push_str("...");
    clipped
}

fn json_value_field_snippet(input: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut search_start = 0;
    while let Some(relative_index) = input[search_start..].find(&needle) {
        let key_start = search_start + relative_index;
        let after_key = key_start + needle.len();
        let after_colon = input[after_key..].find(':').map(|index| after_key + index + 1)?;
        if let Some((value, _end)) = parse_json_value_slice(&input[after_colon..]) {
            return Some(value.trim().to_owned());
        }
        search_start = after_key;
    }
    None
}

fn json_value_field_snippets(input: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let mut snippets = Vec::new();
    let mut search_start = 0;
    while let Some(relative_index) = input[search_start..].find(&needle) {
        let key_start = search_start + relative_index;
        let after_key = key_start + needle.len();
        let Some(after_colon) = input[after_key..].find(':').map(|index| after_key + index + 1)
        else {
            break;
        };
        if let Some((value, consumed)) = parse_json_value_slice(&input[after_colon..]) {
            snippets.push(value.trim().to_owned());
            search_start = after_colon + consumed;
        } else {
            search_start = after_key;
        }
    }
    snippets
}

fn json_object_direct_entries(object: &str) -> Vec<(String, String)> {
    let object = object.trim();
    if !object.starts_with('{') || json_container_slice_end(object, '{') != Some(object.len()) {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let mut rest = &object[1..object.len().saturating_sub(1)];
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        if !rest.starts_with('"') {
            break;
        }
        let Some(key_end) = json_string_slice_end(rest) else {
            break;
        };
        let Some(key) = parse_json_string_value(&rest[..key_end]) else {
            break;
        };
        rest = rest[key_end..].trim_start();
        let Some(after_colon) = rest.strip_prefix(':') else {
            break;
        };
        let Some((value, consumed)) = parse_json_value_slice(after_colon) else {
            break;
        };
        entries.push((key, value.trim().to_owned()));
        rest = after_colon[consumed..].trim_start();
        if let Some(after_comma) = rest.strip_prefix(',') {
            rest = after_comma;
        } else {
            break;
        }
    }
    entries
}

fn parse_json_value_slice(value: &str) -> Option<(&str, usize)> {
    let start = value.find(|character: char| !character.is_whitespace())?;
    let rest = &value[start..];
    let first = rest.chars().next()?;
    match first {
        '"' => {
            let end = json_string_slice_end(rest)?;
            Some((&rest[..end], start + end))
        }
        '{' | '[' => {
            let end = json_container_slice_end(rest, first)?;
            Some((&rest[..end], start + end))
        }
        _ => {
            let end = rest
                .char_indices()
                .find_map(|(index, character)| {
                    matches!(character, ',' | '}' | ']' | '\r' | '\n').then_some(index)
                })
                .unwrap_or(rest.len());
            Some((&rest[..end], start + end))
        }
    }
}

fn json_string_slice_end(input: &str) -> Option<usize> {
    let mut escaped = false;
    for (index, character) in input.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(index + character.len_utf8());
        }
    }
    None
}

fn json_container_slice_end(input: &str, opener: char) -> Option<usize> {
    let closer = if opener == '{' { '}' } else { ']' };
    let mut in_string = false;
    let mut escaped = false;
    let mut depth = 0_usize;
    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            current if current == opener => depth += 1,
            current if current == closer => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index + character.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_cycle_ids_from_latest_json(latest_json: &str) -> Vec<String> {
    json_string_field_values(latest_json, "cycle_id")
        .into_iter()
        .chain(json_string_field_values(latest_json, "id"))
        .filter(|value| validate_cycle_report_id(value).is_ok())
        .fold(Vec::<String>::new(), |mut ids, value| {
            if !ids.contains(&value) {
                ids.push(value);
            }
            ids
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CycleAliasIndexEntry {
    id: String,
    alias: String,
    category: String,
    category_key: String,
    sequence: String,
    title: Option<String>,
}

fn extract_cycle_alias_entries(aliases_json: &str) -> Result<Vec<CycleAliasIndexEntry>, String> {
    let trimmed = aliases_json.trim();
    if trimmed.is_empty() {
        return Err("aliases.json malformed: file is empty".to_owned());
    }
    let Some((root, consumed)) = parse_json_value_slice(trimmed) else {
        return Err("aliases.json malformed: root JSON value is incomplete".to_owned());
    };
    if consumed != trimmed.len() {
        return Err("aliases.json malformed: trailing data after root object".to_owned());
    }
    if !root.trim_start().starts_with('{') {
        return Err("aliases.json malformed: root must be a JSON object".to_owned());
    }

    let version = required_json_non_negative_integer_display_field(root, "version")
        .map_err(|error| format!("aliases.json malformed: {error}"))?;
    if version != "1" {
        return Err(format!("aliases.json malformed: unsupported version {version}"));
    }
    let aliases_value = json_value_field_snippet(root, "aliases")
        .ok_or_else(|| "aliases.json malformed: missing field aliases".to_owned())?;
    let alias_objects = required_json_object_array_entries(&aliases_value, "aliases")?;
    alias_objects
        .iter()
        .enumerate()
        .map(|(index, object)| parse_cycle_alias_index_entry(object, index))
        .collect()
}

fn resolve_cycle_report_alias_from_json(
    aliases_json: &str,
    cycle_alias: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let matches = extract_cycle_alias_entries(aliases_json)?
        .into_iter()
        .filter(|entry| entry.alias == cycle_alias)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entry] => Ok(entry.id.clone()),
        [] => Err(format!("cycle alias not found in aliases.json: {cycle_alias}").into()),
        _ => Err(format!("cycle alias maps ambiguously in aliases.json: {cycle_alias}").into()),
    }
}

fn parse_cycle_alias_index_entry(
    object: &str,
    index: usize,
) -> Result<CycleAliasIndexEntry, String> {
    let cycle_id = required_alias_index_string_field(object, "cycle_id", index)?;
    let cycle_alias = required_alias_index_string_field(object, "cycle_alias", index)?;
    let cycle_category = required_alias_index_string_field(object, "cycle_category", index)?;
    let cycle_category_key =
        required_alias_index_string_field(object, "cycle_category_key", index)?;
    let cycle_sequence = required_alias_index_sequence_field(object, index)?;
    let cycle_title = required_alias_index_nullable_string_field(object, "cycle_title", index)?;
    let _created_at = required_alias_index_string_field(object, "created_at", index)?;

    validate_cycle_report_id(&cycle_id)
        .map_err(|error| alias_index_entry_error(index, &error.to_string()))?;
    let alias_parts = validate_development_cycle_alias(&cycle_alias)
        .map_err(|error| alias_index_entry_error(index, &error))?;
    if cycle_category != alias_parts.cycle_category {
        return Err(alias_index_entry_error(
            index,
            "field cycle_category must match cycle_alias category",
        ));
    }
    if cycle_category_key != alias_parts.cycle_category_key {
        return Err(alias_index_entry_error(
            index,
            "field cycle_category_key must match cycle_alias category key",
        ));
    }
    if cycle_sequence != alias_parts.cycle_sequence.to_string() {
        return Err(alias_index_entry_error(
            index,
            "field cycle_sequence must match cycle_alias sequence",
        ));
    }

    Ok(CycleAliasIndexEntry {
        id: cycle_id,
        alias: cycle_alias,
        category: cycle_category,
        category_key: cycle_category_key,
        sequence: cycle_sequence,
        title: cycle_title,
    })
}

fn required_json_object_array_entries(
    value: &str,
    field_name: &str,
) -> Result<Vec<String>, String> {
    let array = value.trim();
    if !array.starts_with('[') {
        return Err(format!("aliases.json malformed: field {field_name} must be a JSON array"));
    }
    if json_container_slice_end(array, '[') != Some(array.len()) {
        return Err(format!("aliases.json malformed: field {field_name} array is incomplete"));
    }
    let mut entries = Vec::new();
    let mut rest = &array[1..array.len().saturating_sub(1)];
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let index = entries.len();
        if !rest.starts_with('{') {
            return Err(format!(
                "aliases.json malformed: {field_name}[{index}] must be a JSON object"
            ));
        }
        let Some(end) = json_container_slice_end(rest, '{') else {
            return Err(format!(
                "aliases.json malformed: {field_name}[{index}] object is incomplete"
            ));
        };
        entries.push(rest[..end].to_owned());
        rest = rest[end..].trim_start();
        if rest.is_empty() {
            break;
        }
        let Some(after_comma) = rest.strip_prefix(',') else {
            return Err(format!(
                "aliases.json malformed: expected comma after {field_name}[{index}]"
            ));
        };
        if after_comma.trim_start().is_empty() {
            return Err(format!("aliases.json malformed: trailing comma in field {field_name}"));
        }
        rest = after_comma;
    }
    Ok(entries)
}

fn required_alias_index_string_field(
    object: &str,
    field_name: &str,
    index: usize,
) -> Result<String, String> {
    required_json_string_field(object, field_name)
        .map_err(|error| alias_index_entry_error(index, &error))
}

fn required_alias_index_nullable_string_field(
    object: &str,
    field_name: &str,
    index: usize,
) -> Result<Option<String>, String> {
    required_json_nullable_string_field(object, field_name)
        .map_err(|error| alias_index_entry_error(index, &error))
}

fn required_alias_index_sequence_field(object: &str, index: usize) -> Result<String, String> {
    let sequence = required_json_non_negative_integer_display_field(object, "cycle_sequence")
        .map_err(|error| alias_index_entry_error(index, &error))?;
    if sequence == "0" {
        return Err(alias_index_entry_error(index, "field cycle_sequence must be positive"));
    }
    Ok(sequence)
}

fn alias_index_entry_error(index: usize, message: &str) -> String {
    format!("aliases.json malformed: aliases[{index}] {message}")
}

fn json_string_field_values(input: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let mut values = Vec::new();
    let mut search_start = 0;
    while let Some(relative_index) = input[search_start..].find(&needle) {
        let key_start = search_start + relative_index;
        let after_key = key_start + needle.len();
        if let Some(after_colon) = input[after_key..].find(':').map(|index| after_key + index + 1)
            && let Some(value) = parse_json_string_value(&input[after_colon..])
        {
            values.push(value);
        }
        search_start = after_key;
    }
    values
}

fn cycle_report_status_label(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "success" | "succeeded" | "passed" | "complete" | "completed" => "성공".to_owned(),
        "failed" | "failure" | "error" => "실패".to_owned(),
        "stopped" | "interrupted" | "canceled" | "cancelled" => "중단".to_owned(),
        "blocked" => "blocked".to_owned(),
        "" => "unknown".to_owned(),
        other => other.to_owned(),
    }
}

fn cycle_report_status_class(status: &str) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "success" | "succeeded" | "passed" | "complete" | "completed" => "status-success",
        "failed" | "failure" | "error" => "status-failed",
        "stopped" | "interrupted" | "canceled" | "cancelled" => "status-stopped",
        "blocked" => "status-blocked",
        _ => "",
    }
}

fn render_cycle_report_workflow_map(summary: &CycleReportArtifactSummary) -> String {
    [
        (
            "사용자 요청 파생 요약",
            "user_request_display_summary_ko",
            summary.user_request_display_summary_ko.as_str(),
        ),
        ("오케스트라 지시", "orchestra_instruction", summary.orchestra_instruction.as_str()),
        ("역할 반환", "role_returns", "아래 역할 반환 evidence에서 원문/파생 값을 확인합니다."),
        ("검증", "verification", summary.verification_result.as_str()),
        ("결과", "result", summary.result_summary.as_str()),
    ]
    .iter()
    .map(|(title, key, value)| {
        format!(
            r#"<article class="flow-card">
        <span>{}</span>
        <strong>{}</strong>
        <p>{}</p>
      </article>"#,
            html_escape(key),
            html_escape(title),
            html_escape(value)
        )
    })
    .collect::<Vec<_>>()
    .join("\n      ")
}

fn render_evidence_status_warning(summary: &CycleReportArtifactSummary) -> String {
    let evidence_status = summary.evidence_status.trim().to_ascii_lowercase();
    let audit_status = summary.audit_status.as_deref().unwrap_or("").trim().to_ascii_lowercase();
    let needs_warning = !matches!(evidence_status.as_str(), "complete")
        || matches!(audit_status.as_str(), "fail" | "failed" | "warn" | "warning");
    if !needs_warning {
        return String::new();
    }

    let warning = summary.evidence_warning.as_deref().unwrap_or(
        "원문 evidence 또는 trace audit이 complete/pass 상태가 아닙니다. 누락/실패 상태를 성공처럼 보정하지 않습니다.",
    );
    let reliability_label =
        audit_reliability_label(summary.audit_status.as_deref(), &summary.evidence_status);
    let reliability_description =
        audit_reliability_description(summary.audit_status.as_deref(), &summary.evidence_status);
    let raw_audit_status = summary.audit_status.as_deref().unwrap_or("없음");
    format!(
        r#"<div class="warning-banner">
  <strong>보고서 신뢰성 검증: {}</strong>
  <p>{}</p>
  <p>{}</p>
  <details class="technical-details">
    <summary>기술 상세</summary>
    <table class="metadata-table">
      <tbody>
        <tr><th scope="row">audit.status</th><td><code>audit.status={}</code></td></tr>
        <tr><th scope="row">evidence_status</th><td><code>evidence_status={}</code></td></tr>
      </tbody>
    </table>
  </details>
</div>"#,
        html_escape(reliability_label),
        html_escape(reliability_description),
        html_escape(warning),
        html_escape(raw_audit_status),
        html_escape(&summary.evidence_status),
    )
}

fn audit_reliability_label<'a>(
    audit_status: Option<&'a str>,
    evidence_status: &'a str,
) -> &'static str {
    let audit_status = audit_status.unwrap_or("").trim().to_ascii_lowercase();
    let evidence_status = evidence_status.trim().to_ascii_lowercase();
    if matches!(audit_status.as_str(), "pass" | "passed" | "ok" | "success")
        && matches!(evidence_status.as_str(), "complete")
    {
        "원문 증거 확인됨"
    } else if matches!(audit_status.as_str(), "fail" | "failed")
        || matches!(evidence_status.as_str(), "fail" | "failed" | "incomplete")
    {
        "원문 증거 불완전"
    } else if matches!(audit_status.as_str(), "warn" | "warning")
        || matches!(evidence_status.as_str(), "warn" | "warning")
    {
        "원문 증거 확인 필요"
    } else {
        "검증 상태 확인 필요"
    }
}

fn audit_reliability_description<'a>(
    audit_status: Option<&'a str>,
    evidence_status: &'a str,
) -> &'static str {
    let audit_status = audit_status.unwrap_or("").trim().to_ascii_lowercase();
    let evidence_status = evidence_status.trim().to_ascii_lowercase();
    if matches!(audit_status.as_str(), "fail" | "failed")
        || matches!(evidence_status.as_str(), "fail" | "failed" | "incomplete")
    {
        "작업 결과가 실패했다는 뜻이 아니라, 보고서가 참조해야 할 일부 원문 증거가 누락되었거나 완전하게 확인되지 않았다는 뜻입니다."
    } else if matches!(audit_status.as_str(), "pass" | "passed" | "ok" | "success")
        && matches!(evidence_status.as_str(), "complete")
    {
        "보고서가 참조하는 필수 원문 증거가 확인된 상태입니다."
    } else {
        "Audit은 작업 성공/실패가 아니라 보고서 신뢰성 검증입니다. 원문 증거 상태를 기술 상세와 함께 확인해야 합니다."
    }
}

fn render_missing_user_request_evidence_warning(summary: &CycleReportArtifactSummary) -> String {
    match &summary.user_request_verbatim {
        VerbatimEvidenceStatus::Valid(_) => String::new(),
        VerbatimEvidenceStatus::Missing => {
            r#"<div class="warning-banner">원문 증거 없음: user_request_verbatim이 없습니다. 아래 사용자 요청 내용은 legacy/derived summary로만 표시하며 원문처럼 복원하지 않습니다.</div>"#
                .to_owned()
        }
        VerbatimEvidenceStatus::Invalid(reason) => format!(
            r#"<div class="warning-banner">원문 증거 무효: {}. 아래 사용자 요청 내용은 legacy/derived summary로만 표시하며 원문처럼 복원하지 않습니다.</div>"#,
            html_escape(reason)
        ),
    }
}

fn render_verbatim_evidence_block(
    evidence: &VerbatimEvidenceStatus,
    missing_label: &str,
) -> String {
    let Some(evidence) = evidence.valid() else {
        let warning = match evidence {
            VerbatimEvidenceStatus::Invalid(reason) => {
                format!("원문 증거 무효: {reason}. 원문 블록을 표시하지 않습니다.")
            }
            VerbatimEvidenceStatus::Missing => missing_label.to_owned(),
            VerbatimEvidenceStatus::Valid(_) => unreachable!("valid evidence handled above"),
        };
        return format!(r#"<div class="warning-banner">{}</div>"#, html_escape(&warning));
    };
    let agent_id = evidence.agent_id.as_deref().unwrap_or("null");
    format!(
        r#"<div class="verbatim-evidence">
          <pre class="evidence-text">{}</pre>
          <details class="evidence-details evidence-metadata">
            <summary>증거 메타데이터</summary>
            <table class="metadata-table">
              <tbody>
                <tr><th scope="row">source_type</th><td>{}</td></tr>
                <tr><th scope="row">source_ref</th><td>{}</td></tr>
                <tr><th scope="row">hash_sha256</th><td>{}</td></tr>
                <tr><th scope="row">role</th><td>{}</td></tr>
                <tr><th scope="row">agent_id</th><td>{}</td></tr>
                <tr><th scope="row">timestamp</th><td>{}</td></tr>
                <tr><th scope="row">order</th><td>{}</td></tr>
              </tbody>
            </table>
          </details>
        </div>"#,
        html_escape(&evidence.text),
        html_escape(&evidence.source_type),
        html_escape(&evidence.source_ref),
        html_escape(&evidence.hash_sha256),
        html_escape(&evidence.role),
        html_escape(agent_id),
        html_escape(&evidence.timestamp),
        html_escape(&evidence.order)
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LabeledVerbatimEvidence {
    artifact_name: &'static str,
    field_name: &'static str,
    status: VerbatimEvidenceStatus,
}

fn render_full_verbatim_evidence_index(artifact: &CycleReportArtifact) -> String {
    let mut entries = Vec::new();
    entries.extend(labeled_verbatim_evidence_from_text("report.json", &artifact.report_json));
    if let Some(raw_json) = artifact.raw_json.as_deref() {
        entries.extend(labeled_verbatim_evidence_from_text("raw.json", raw_json));
    }
    entries = dedupe_labeled_verbatim_evidence(entries);

    if entries.is_empty() {
        return r#"<article class="report-card status-failed">
      <span>full_verbatim_evidence</span>
      <strong>표시할 원문 evidence 없음</strong>
      <p>report.json/raw.json에서 user_request_verbatim, prompt_verbatim, response_verbatim, result_verbatim 객체를 찾지 못했습니다. 원문을 추정하거나 요약으로 대체하지 않습니다.</p>
    </article>"#
            .to_owned();
    }

    let cards = entries
        .iter()
        .map(|entry| {
            let body = render_verbatim_evidence_block(
                &entry.status,
                "원문 evidence 객체가 null 또는 missing 상태입니다.",
            );
            format!(
                r#"<article class="delegation-card">
      <h3>{} · {}</h3>
      {}
    </article>"#,
                html_escape(entry.artifact_name),
                html_escape(entry.field_name),
                body
            )
        })
        .collect::<Vec<_>>()
        .join("\n    ");
    format!(
        r#"<div class="delegation-list" aria-label="전체 원문 evidence 색인">
    {cards}
  </div>"#,
    )
}

fn labeled_verbatim_evidence_from_text(
    artifact_name: &'static str,
    text: &str,
) -> Vec<LabeledVerbatimEvidence> {
    [
        "user_request_verbatim",
        "prompt_verbatim",
        "response_verbatim",
        "result_verbatim",
        "command_verbatim",
        "output_verbatim",
    ]
    .iter()
    .flat_map(|field_name| {
        json_value_field_snippets(text, field_name).into_iter().map(move |value| {
            let value = value.trim().to_owned();
            let status = if value == "null" {
                VerbatimEvidenceStatus::Missing
            } else {
                VerbatimEvidence::from_object(&value)
                    .map_or_else(VerbatimEvidenceStatus::Invalid, VerbatimEvidenceStatus::Valid)
            };
            LabeledVerbatimEvidence { artifact_name, field_name, status }
        })
    })
    .collect()
}

fn dedupe_labeled_verbatim_evidence(
    entries: Vec<LabeledVerbatimEvidence>,
) -> Vec<LabeledVerbatimEvidence> {
    let mut seen = Vec::<String>::new();
    let mut deduped = Vec::new();
    for entry in entries {
        let key = match &entry.status {
            VerbatimEvidenceStatus::Valid(evidence) => {
                format!("{}:{}:{}", entry.field_name, evidence.source_ref, evidence.hash_sha256)
            }
            VerbatimEvidenceStatus::Invalid(reason) => {
                format!("{}:{}:{}", entry.artifact_name, entry.field_name, reason)
            }
            VerbatimEvidenceStatus::Missing => {
                format!("{}:{}:missing", entry.artifact_name, entry.field_name)
            }
        };
        if !seen.contains(&key) {
            seen.push(key);
            deduped.push(entry);
        }
    }
    deduped
}

fn render_orchestra_delegations(report_json: &str, summary: &CycleReportArtifactSummary) -> String {
    let delegations = json_objects_from_field(report_json, "orchestra_delegations");
    if delegations.is_empty() {
        return format!(
            r#"<article class="delegation-card">
      <div class="warning-banner">원문 증거 없음: orchestra_delegations[]가 없어서 역할별 prompt 원문을 표시할 수 없습니다.</div>
      <div class="derived-card">
        <span class="derived-label">derived summary, not verbatim</span>
        <strong>legacy orchestra_instruction</strong>
        <p>{}</p>
      </div>
    </article>"#,
            html_escape(&summary.orchestra_instruction)
        );
    }

    delegations
        .iter()
        .map(|delegation| {
            let role =
                json_text_field(delegation, "role").unwrap_or_else(|| "역할 없음".to_owned());
            let agent_id = json_text_field(delegation, "agent_id")
                .unwrap_or_else(|| "agent_id 없음".to_owned());
            let order = json_text_field(delegation, "order").unwrap_or_else(|| "-".to_owned());
            let source_ref = json_text_field(delegation, "dispatch_event_id")
                .or_else(|| json_text_field(delegation, "source_ref"))
                .unwrap_or_else(|| "source_ref 없음".to_owned());
            let prompt_evidence = verbatim_evidence_from_field(delegation, "prompt_verbatim");
            let prompt_summary = json_text_field(delegation, "prompt_derived_summary_ko")
                .unwrap_or_else(|| "prompt 파생 요약 없음".to_owned());
            format!(
                r#"<article class="delegation-card">
      <h3>{} · order {}</h3>
      <div class="evidence-meta">
        <span class="diff-pill">agent_id: {}</span>
        <span class="diff-pill">source_ref: {}</span>
      </div>
      {}
      <div class="derived-card">
        <span class="derived-label">derived summary, not verbatim</span>
        <strong>prompt_derived_summary_ko</strong>
        <p>{}</p>
      </div>
    </article>"#,
                html_escape(&role),
                html_escape(&order),
                html_escape(&agent_id),
                html_escape(&source_ref),
                render_verbatim_evidence_block(&prompt_evidence, "prompt_verbatim 원문 증거 없음"),
                html_escape(&prompt_summary)
            )
        })
        .collect::<Vec<_>>()
        .join("\n    ")
}

fn render_derived_summaries(report_json: &str) -> String {
    let derived = json_value_field_snippet(report_json, "derived_summaries").unwrap_or_else(|| {
        r#"{"legacy":"derived_summaries 필드 없음. legacy summary 필드는 원문으로 취급하지 않음."}"#
            .to_owned()
    });
    format!(
        r#"<article class="report-card">
      <span class="derived-label">derived summary, not verbatim</span>
      <strong>derived_summaries</strong>
      <pre>{}</pre>
    </article>"#,
        html_escape(&json_value_to_display_text(&derived))
    )
}

struct EvidenceAuditDisplay {
    audit_status: String,
    evidence_status: String,
    missing_required_inputs: String,
    warnings: String,
    missing_evidence: String,
    invalid_evidence: String,
    derived_not_verbatim: String,
    excluded_code_changes: String,
    trace_audit: String,
    trace_audit_command: String,
}

fn evidence_audit_display(artifact: &CycleReportArtifact) -> EvidenceAuditDisplay {
    EvidenceAuditDisplay {
        audit_status: evidence_audit_field(artifact, "status")
            .unwrap_or_else(|| "status 필드 없음".to_owned()),
        evidence_status: evidence_audit_field(artifact, "evidence_status")
            .or_else(|| json_text_field(&artifact.report_json, "evidence_status"))
            .unwrap_or_else(|| "evidence_status 필드 없음".to_owned()),
        missing_required_inputs: evidence_audit_field(artifact, "missing_required_inputs")
            .unwrap_or_else(|| "missing_required_inputs 필드 없음".to_owned()),
        warnings: evidence_audit_field(artifact, "warnings")
            .unwrap_or_else(|| "warnings 필드 없음".to_owned()),
        missing_evidence: evidence_audit_field(artifact, "missing_evidence")
            .unwrap_or_else(|| "missing_evidence 필드 없음".to_owned()),
        invalid_evidence: evidence_audit_field(artifact, "invalid_evidence")
            .unwrap_or_else(|| "invalid_evidence 필드 없음".to_owned()),
        derived_not_verbatim: evidence_audit_field(artifact, "derived_not_verbatim")
            .unwrap_or_else(|| "derived_not_verbatim 필드 없음".to_owned()),
        excluded_code_changes: evidence_audit_field(artifact, "excluded_code_changes")
            .unwrap_or_else(|| "excluded_code_changes 필드 없음".to_owned()),
        trace_audit: trace_audit_display_text(artifact),
        trace_audit_command: trace_audit_command_display_text(artifact),
    }
}

fn trace_audit_display_text(artifact: &CycleReportArtifact) -> String {
    artifact
        .audit_json
        .as_deref()
        .and_then(|audit| json_value_field_snippet(audit, "trace_audit"))
        .map(|value| json_value_to_display_text(&value))
        .or_else(|| {
            artifact
                .audit_json
                .as_deref()
                .and_then(|audit| json_value_field_snippet(audit, "findings"))
                .map(|value| json_value_to_display_text(&value))
        })
        .unwrap_or_else(|| "trace_audit/findings 필드 없음".to_owned())
}

fn trace_audit_command_display_text(artifact: &CycleReportArtifact) -> String {
    artifact
        .audit_json
        .as_deref()
        .and_then(|audit| json_value_field_snippet(audit, "trace_audit_command"))
        .map_or_else(
            || "trace_audit_command 필드 없음".to_owned(),
            |value| json_value_to_display_text(&value),
        )
}

fn render_evidence_audit(artifact: &CycleReportArtifact) -> String {
    let audit = evidence_audit_display(artifact);
    let reliability_label =
        audit_reliability_label(Some(&audit.audit_status), &audit.evidence_status);
    let reliability_description =
        audit_reliability_description(Some(&audit.audit_status), &audit.evidence_status);
    format!(
        r#"<div class="audit-panel">
      <article class="audit-readable">
        <span>Audit 뜻: 보고서 신뢰성 검증</span>
        <strong>{}</strong>
        <p>{}</p>
        <p>아래 목록은 작업 결과를 다시 판정하는 영역이 아니라, 보고서가 원문 evidence를 얼마나 완전하게 참조하는지 확인하는 영역입니다.</p>
      </article>
      <div class="audit-fields">
        <article class="report-card">
          <span>누락된 원문 evidence 목록</span>
          <strong>missing evidence</strong>
          <pre>{}</pre>
        </article>
        <article class="report-card">
          <span>무효 원문 evidence 목록</span>
          <strong>invalid evidence</strong>
          <pre>{}</pre>
        </article>
        <article class="report-card">
          <span>원문이 아니라 파생 표시인 필드</span>
          <strong>derived summary, not verbatim</strong>
          <pre>{}</pre>
        </article>
        <article class="report-card">
          <span>필수 입력 누락</span>
          <strong>missing required inputs</strong>
          <pre>{}</pre>
        </article>
        <article class="report-card">
          <span>검증 경고</span>
          <strong>warnings</strong>
          <pre>{}</pre>
        </article>
        <article class="report-card">
          <span>제외된 코드 변경 사유</span>
          <strong>excluded code changes reason</strong>
          <pre>{}</pre>
        </article>
      </div>
      <details class="technical-details audit-diagnostic-details">
        <summary>보고서 신뢰성 진단 기술 상세</summary>
        <p>아래 원문은 작업 실패 판정이 아니라 trace audit이 보고서 evidence를 점검한 기술 진단입니다.</p>
        <table class="metadata-table">
          <tbody>
            <tr><th scope="row">trace_audit/findings</th><td><pre>{}</pre></td></tr>
            <tr><th scope="row">trace_audit_command</th><td><pre>{}</pre></td></tr>
          </tbody>
        </table>
      </details>
      <details class="technical-details">
        <summary>기술 상세: 원문 audit 필드</summary>
        <table class="metadata-table">
          <tbody>
            <tr><th scope="row">audit.status</th><td><code>audit.status={}</code></td></tr>
            <tr><th scope="row">evidence_status</th><td><code>evidence_status={}</code></td></tr>
          </tbody>
        </table>
      </details>
    </div>"#,
        html_escape(reliability_label),
        html_escape(reliability_description),
        html_escape(&audit.missing_evidence),
        html_escape(&audit.invalid_evidence),
        html_escape(&audit.derived_not_verbatim),
        html_escape(&audit.missing_required_inputs),
        html_escape(&audit.warnings),
        html_escape(&audit.excluded_code_changes),
        html_escape(&audit.trace_audit),
        html_escape(&audit.trace_audit_command),
        html_escape(&audit.audit_status),
        html_escape(&audit.evidence_status)
    )
}

fn evidence_audit_field(artifact: &CycleReportArtifact, key: &str) -> Option<String> {
    json_value_field_snippet(&artifact.report_json, "audit")
        .and_then(|audit| json_value_field_snippet(&audit, key))
        .or_else(|| {
            artifact.audit_json.as_deref().and_then(|audit| json_value_field_snippet(audit, key))
        })
        .map(|value| json_value_to_display_text(&value))
}

fn render_cycle_report_role_board(report_json: &str, raw_json: Option<&str>) -> String {
    let role_returns = json_value_field_snippet(report_json, "role_returns");
    let mut cards = role_returns
        .as_deref()
        .map(json_object_direct_entries)
        .unwrap_or_default()
        .iter()
        .map(|(role, value)| render_role_return_card(role, value))
        .collect::<Vec<_>>();

    cards.extend(render_raw_role_return_evidence_cards(raw_json));

    if cards.is_empty() {
        return r#"<article class="role-card status-failed">
        <span>role_returns</span>
        <strong>역할 반환 evidence 없음</strong>
        <p>role_returns나 raw.json의 response_verbatim/result_verbatim을 찾지 못했습니다. 요약으로 원문을 복원하지 않습니다.</p>
      </article>"#
            .to_owned();
    }

    cards.join("\n      ")
}

fn render_role_return_card(role: &str, value: &str) -> String {
    if value.trim_start().starts_with('{') {
        let return_summary =
            json_text_field(value, "return_summary").unwrap_or_else(|| "반환 요약 없음".to_owned());
        let dispatch_summary = json_text_field(value, "dispatch_summary")
            .unwrap_or_else(|| "지시 요약 없음".to_owned());
        let context_report = json_text_field(value, "context_report")
            .unwrap_or_else(|| "Context Report 없음".to_owned());
        let verification_result = json_text_field(value, "verification_result")
            .unwrap_or_else(|| "검증 결과 없음".to_owned());
        let verbatim = first_verbatim_evidence_from_fields(
            value,
            &["return_verbatim", "response_verbatim", "result_verbatim"],
        );
        let evidence_block = verbatim.map_or_else(
            || {
                r#"<div class="warning-banner">role_returns 항목에 반환 원문 evidence가 없습니다. 이 값은 derived/display projection이며 raw.json의 response_verbatim/result_verbatim이 원문 source입니다.</div>"#
                    .to_owned()
            },
            |(field, evidence)| {
                format!(
                    r"<strong>{}</strong>{}",
                    html_escape(field),
                    render_verbatim_evidence_block(&evidence, "반환 원문 evidence 없음")
                )
            },
        );
        format!(
            r#"<article class="role-card">
        <span>{}</span>
        <strong>역할 반환</strong>
        {}
        <div class="derived-card">
          <span class="derived-label">derived/display summary, not verbatim</span>
          <p>dispatch_summary: {}</p>
          <p>return_summary: {}</p>
          <p>context_report: {}</p>
          <p>verification_result: {}</p>
        </div>
      </article>"#,
            html_escape(role),
            evidence_block,
            html_escape(&dispatch_summary),
            html_escape(&return_summary),
            html_escape(&context_report),
            html_escape(&verification_result)
        )
    } else {
        format!(
            r#"<article class="role-card">
        <span>{}</span>
        <strong>legacy role return</strong>
        <div class="warning-banner">문자열 role return은 원문 evidence가 아니라 legacy/derived 표시값입니다. 원문을 복원하지 않습니다.</div>
        <p>{}</p>
      </article>"#,
            html_escape(role),
            html_escape(&json_value_to_display_text(value))
        )
    }
}

fn first_verbatim_evidence_from_fields(
    object: &str,
    fields: &[&'static str],
) -> Option<(&'static str, VerbatimEvidenceStatus)> {
    fields.iter().find_map(|field| {
        let evidence = verbatim_evidence_from_field(object, field);
        matches!(evidence, VerbatimEvidenceStatus::Valid(_)).then_some((*field, evidence))
    })
}

fn render_raw_role_return_evidence_cards(raw_json: Option<&str>) -> Vec<String> {
    let Some(raw_json) = raw_json else {
        return Vec::new();
    };
    labeled_verbatim_evidence_from_text("raw.json", raw_json)
        .into_iter()
        .filter(|entry| matches!(entry.field_name, "response_verbatim" | "result_verbatim"))
        .map(|entry| {
            format!(
                r#"<article class="role-card">
        <span>{} · {}</span>
        <strong>raw 역할 반환 원문</strong>
        {}
      </article>"#,
                html_escape(entry.artifact_name),
                html_escape(entry.field_name),
                render_verbatim_evidence_block(&entry.status, "반환 원문 evidence 없음")
            )
        })
        .collect()
}

fn render_command_evidence_view(artifact: &CycleReportArtifact) -> String {
    let mut cards = json_objects_from_field(&artifact.report_json, "commands")
        .iter()
        .map(|command| render_command_evidence_card(command))
        .collect::<Vec<_>>();
    cards.extend(render_raw_test_trace_cards(artifact.raw_json.as_deref()));

    if cards.is_empty() {
        return r#"<article class="report-card status-failed">
      <span>commands</span>
      <strong>테스트 명령/결과 원문 evidence 없음</strong>
      <p>commands[]나 raw.json test trace를 찾지 못했습니다. 검증 결과를 추정하지 않습니다.</p>
    </article>"#
            .to_owned();
    }

    format!(
        r#"<div class="delegation-list" aria-label="테스트 명령과 결과 evidence">
    {}
  </div>"#,
        cards.join("\n    ")
    )
}

fn render_command_evidence_card(command: &str) -> String {
    let command_text =
        json_text_field(command, "command").unwrap_or_else(|| "command 필드 없음".to_owned());
    let actor = json_text_field(command, "actor").unwrap_or_else(|| "actor 없음".to_owned());
    let result = json_text_field(command, "result").unwrap_or_else(|| "result 없음".to_owned());
    let evidence = json_text_field(command, "evidence")
        .or_else(|| json_text_field(command, "evidence_ref"))
        .unwrap_or_else(|| "evidence 필드 없음".to_owned());
    let verbatim = ["command_verbatim", "result_verbatim", "output_verbatim"]
        .iter()
        .map(|field| {
            let evidence = verbatim_evidence_from_field(command, field);
            format!(
                r#"<section class="evidence-field"><strong>{}</strong>{}</section>"#,
                html_escape(field),
                render_verbatim_evidence_block(&evidence, &format!("{field} 원문 evidence 없음"))
            )
        })
        .collect::<Vec<_>>()
        .join("\n      ");
    format!(
        r#"<article class="delegation-card">
      <h3>{}</h3>
      <div class="evidence-meta">
        <span class="diff-pill">actor: {}</span>
        <span class="diff-pill">result: {}</span>
        <span class="diff-pill">evidence: {}</span>
      </div>
      {}
    </article>"#,
        html_escape(&command_text),
        html_escape(&actor),
        html_escape(&result),
        html_escape(&evidence),
        verbatim
    )
}

fn render_raw_test_trace_cards(raw_json: Option<&str>) -> Vec<String> {
    let Some(raw_json) = raw_json else {
        return Vec::new();
    };
    raw_trace_entry_objects(raw_json)
        .into_iter()
        .filter(|entry| {
            json_text_field(entry, "kind").as_deref() == Some("test_summary")
                || json_text_field(entry, "role_name").as_deref() == Some("test")
        })
        .map(|entry| {
            let event_id =
                json_text_field(&entry, "event_id").unwrap_or_else(|| "event_id 없음".to_owned());
            let kind = json_text_field(&entry, "kind").unwrap_or_else(|| "kind 없음".to_owned());
            let summary =
                json_text_field(&entry, "summary").unwrap_or_else(|| "summary 없음".to_owned());
            let body = json_text_field(&entry, "body").unwrap_or_else(|| "body 없음".to_owned());
            let metadata_json = json_text_field(&entry, "metadata_json")
                .unwrap_or_else(|| "metadata_json 없음".to_owned());
            format!(
                r#"<article class="delegation-card">
      <h3>raw.json test trace · {}</h3>
      <div class="evidence-meta">
        <span class="diff-pill">kind: {}</span>
        <span class="diff-pill">summary: {}</span>
      </div>
      <details class="evidence-details" open>
        <summary>body 원문</summary>
        <pre>{}</pre>
      </details>
      <details class="evidence-details" open>
        <summary>metadata_json 원문</summary>
        <pre>{}</pre>
      </details>
    </article>"#,
                html_escape(&event_id),
                html_escape(&kind),
                html_escape(&summary),
                html_escape(&body),
                html_escape(&metadata_json)
            )
        })
        .collect()
}

fn raw_trace_entry_objects(raw_json: &str) -> Vec<String> {
    let Some(trace_entries) = json_value_field_snippet(raw_json, "trace_entries") else {
        return json_object_slices(raw_json).into_iter().map(str::to_owned).collect();
    };
    json_object_slices(&trace_entries).into_iter().map(str::to_owned).collect()
}

fn render_code_changes_index_view(report_json: &str, cycle_id: &str) -> String {
    let changes = code_change_objects(report_json);
    let diff_href = format!("/reports/{}/diff.html", url_component_encode(cycle_id));
    if changes.is_empty() {
        return format!(
            r#"<article class="report-card">
      <span>code_changes</span>
      <strong>표시할 diff hunk 색인 없음</strong>
      <p>report.json에 code_changes 배열이 없거나 비어 있습니다. 메인 화면은 diff를 합성하지 않고 전용 artifact 링크만 유지합니다.</p>
      <p><a class="btn primary" href="{diff_href}">diff.html 열기</a></p>
    </article>"#
        );
    }

    let items = changes
        .iter()
        .enumerate()
        .map(|(file_index, change)| {
            let file_path =
                json_text_field(change, "file_path").unwrap_or_else(|| "파일 경로 없음".to_owned());
            let language =
                json_text_field(change, "language").unwrap_or_else(|| "언어 미지정".to_owned());
            let change_kind =
                json_text_field(change, "change_kind").unwrap_or_else(|| "unknown".to_owned());
            let summary_ko = json_text_field(change, "summary_ko")
                .unwrap_or_else(|| "한국어 변경 설명 없음".to_owned());
            let file_anchor = code_change_file_anchor(file_index, &file_path);
            let hunk_links = render_code_change_hunk_index_links(change, &diff_href, file_index);
            format!(
                r#"<article class="report-card">
      <span class="derived-label">derived/display file index, not diff hunk</span>
      <strong><a href="{}#{}">{}</a></strong>
      <p>언어: {} · 변경: {}</p>
      <p>{}</p>
      {}
    </article>"#,
                html_escape(&diff_href),
                html_escape(&file_anchor),
                html_escape(&file_path),
                html_escape(&language),
                html_escape(&change_kind),
                html_escape(&summary_ko),
                hunk_links
            )
        })
        .collect::<Vec<_>>()
        .join("\n      ");

    format!(
        r#"<div class="record-list">
      <article class="report-card">
        <span>canonical route</span>
        <strong><a href="{diff_href}">/reports/{}/diff.html</a></strong>
        <p>전체 code hunk line은 diff 전용 artifact에서만 렌더링합니다. 아래 항목은 파일과 hunk anchor 색인입니다.</p>
      </article>
      {items}
    </div>"#,
        html_escape(cycle_id),
    )
}

fn render_code_changes_diff_nav(report_json: &str) -> String {
    let changes = code_change_objects(report_json);
    if changes.is_empty() {
        return r##"<a href="#diff-files">diff hunk 없음</a><a href="#diff-audit">audit</a>"##
            .to_owned();
    }

    let mut links = Vec::new();
    links.push(r##"<a href="#diff-files">top</a>"##.to_owned());
    for (file_index, change) in changes.iter().enumerate() {
        let file_path =
            json_text_field(change, "file_path").unwrap_or_else(|| "파일 경로 없음".to_owned());
        let file_anchor = code_change_file_anchor(file_index, &file_path);
        links.push(format!(
            r##"<a href="#{}">{}</a>"##,
            html_escape(&file_anchor),
            html_escape(&file_path)
        ));
        for (hunk_index, hunk) in json_objects_from_field(change, "hunks").iter().enumerate() {
            let heading = json_text_field(hunk, "heading").unwrap_or_else(|| "@@".to_owned());
            let hunk_anchor = code_change_hunk_anchor(file_index, hunk_index);
            links.push(format!(
                r##"<a href="#{}">h{}: {}</a>"##,
                html_escape(&hunk_anchor),
                hunk_index + 1,
                html_escape(&heading)
            ));
        }
    }
    links.push(r##"<a href="#diff-audit">audit</a>"##.to_owned());
    links.join("\n    ")
}

fn render_code_change_hunk_index_links(change: &str, diff_href: &str, file_index: usize) -> String {
    let hunks = json_objects_from_field(change, "hunks");
    if hunks.is_empty() {
        return r#"<p>hunk anchor 없음</p>"#.to_owned();
    }
    let links = hunks
        .iter()
        .enumerate()
        .map(|(hunk_index, hunk)| {
            let heading = json_text_field(hunk, "heading").unwrap_or_else(|| "@@".to_owned());
            let hunk_anchor = code_change_hunk_anchor(file_index, hunk_index);
            format!(
                r#"<a class="btn" href="{}#{}">{}</a>"#,
                html_escape(diff_href),
                html_escape(&hunk_anchor),
                html_escape(&heading)
            )
        })
        .collect::<Vec<_>>()
        .join("\n        ");
    format!(r#"<div class="actions">{links}</div>"#)
}

fn render_code_changes_diff_view(report_json: &str) -> String {
    let changes = code_change_objects(report_json);
    if changes.is_empty() {
        return r#"<article class="report-card">
      <span>code_changes</span>
      <strong>표시할 전체 diff hunk 없음</strong>
      <p>report.json에 code_changes 배열이 없거나 비어 있습니다. artifact를 새로 만들거나 요약으로 대체하지 않고 빈 상태만 표시합니다.</p>
    </article>"#
            .to_owned();
    }

    changes
        .iter()
        .enumerate()
        .map(|(file_index, change)| {
            let file_path =
                json_text_field(change, "file_path").unwrap_or_else(|| "파일 경로 없음".to_owned());
            let language =
                json_text_field(change, "language").unwrap_or_else(|| "언어 미지정".to_owned());
            let change_kind =
                json_text_field(change, "change_kind").unwrap_or_else(|| "unknown".to_owned());
            let summary_ko = json_text_field(change, "summary_ko")
                .unwrap_or_else(|| "한국어 변경 설명 없음".to_owned());
            let raw_diff_ref = json_text_field(change, "raw_diff_ref")
                .unwrap_or_else(|| "raw_diff_ref 없음".to_owned());
            let author_roles = json_string_array_field_values(change, "author_roles").join(", ");
            let author_roles = if author_roles.is_empty() {
                "역할 기록 없음".to_owned()
            } else {
                author_roles
            };
            let cycle_alias = json_text_field(change, "cycle_alias")
                .or_else(|| json_text_field(report_json, "cycle_alias"));
            let cycle_category = json_text_field(change, "cycle_category")
                .or_else(|| json_text_field(report_json, "cycle_category"));
            let cycle_sequence = json_text_field(change, "cycle_sequence")
                .or_else(|| json_text_field(report_json, "cycle_sequence"));
            let alias_meta = render_code_change_alias_meta(
                cycle_alias.as_deref(),
                cycle_category.as_deref(),
                cycle_sequence.as_deref(),
            );
            let file_anchor = code_change_file_anchor(file_index, &file_path);
            let hunks = render_code_change_hunks(change, file_index);
            format!(
                r#"<article id="{}" class="diff-file">
      <header class="diff-file-header">
        <div class="diff-file-title">
          <strong>{}</strong>
        </div>
        <div class="diff-meta">
          <span class="diff-pill">언어: {}</span>
          <span class="diff-pill">변경: {}</span>
          <span class="diff-pill">역할: {}</span>
          <span class="diff-pill">raw_diff_ref: {}</span>
          {}
        </div>
        <p class="diff-summary"><span class="derived-label">derived/file summary</span> {}</p>
      </header>
      {}
    </article>"#,
                html_escape(&file_anchor),
                html_escape(&file_path),
                html_escape(&language),
                html_escape(&change_kind),
                html_escape(&author_roles),
                html_escape(&raw_diff_ref),
                alias_meta,
                html_escape(&summary_ko),
                hunks
            )
        })
        .collect::<Vec<_>>()
        .join("\n    ")
}

fn code_change_file_anchor(file_index: usize, file_path: &str) -> String {
    format!("file-{}-{}", file_index + 1, html_id_fragment(file_path))
}

fn code_change_hunk_anchor(file_index: usize, hunk_index: usize) -> String {
    format!("hunk-{}-{}", file_index + 1, hunk_index + 1)
}

fn html_id_fragment(value: &str) -> String {
    let mut fragment = String::with_capacity(value.len());
    let mut previous_dash = false;
    for character in value.chars() {
        let normalized = if character.is_ascii_alphanumeric() {
            previous_dash = false;
            Some(character.to_ascii_lowercase())
        } else if matches!(character, '_' | '-') {
            previous_dash = false;
            Some(character)
        } else if previous_dash {
            None
        } else {
            previous_dash = true;
            Some('-')
        };
        if let Some(character) = normalized {
            fragment.push(character);
        }
    }
    let fragment = fragment.trim_matches('-').to_owned();
    if fragment.is_empty() { "change".to_owned() } else { fragment }
}

fn render_code_change_alias_meta(
    cycle_alias: Option<&str>,
    cycle_category: Option<&str>,
    cycle_sequence: Option<&str>,
) -> String {
    [("별칭", cycle_alias), ("category", cycle_category), ("sequence", cycle_sequence)]
        .into_iter()
        .filter_map(|(label, value)| {
            value.map(|value| {
                format!(
                    r#"<span class="diff-pill">{}: {}</span>"#,
                    html_escape(label),
                    html_escape(value)
                )
            })
        })
        .collect::<Vec<_>>()
        .join("\n          ")
}

fn render_code_change_hunks(change: &str, file_index: usize) -> String {
    let hunks = json_objects_from_field(change, "hunks");
    if hunks.is_empty() {
        return r#"<div class="diff-hunk">
        <p class="diff-hunk-summary">이 파일의 hunk 정보가 비어 있습니다.</p>
      </div>"#
            .to_owned();
    }

    hunks
        .iter()
        .enumerate()
        .map(|(hunk_index, hunk)| {
            let heading = json_text_field(hunk, "heading").unwrap_or_else(|| "@@".to_owned());
            let summary_ko =
                json_text_field(hunk, "summary_ko").unwrap_or_else(|| "hunk 설명 없음".to_owned());
            let old_start = json_line_number_field(hunk, "old_start");
            let old_lines = json_text_field(hunk, "old_lines").unwrap_or_else(|| "0".to_owned());
            let new_start = json_line_number_field(hunk, "new_start");
            let new_lines = json_text_field(hunk, "new_lines").unwrap_or_else(|| "0".to_owned());
            let lines = render_code_change_lines(hunk);
            let explanations = render_code_change_explanations(hunk);
            let hunk_anchor = code_change_hunk_anchor(file_index, hunk_index);
            format!(
                r#"<section id="{}" class="diff-hunk">
        <div class="diff-hunk-heading">
          <code>{}</code>
          <span class="diff-pill">old {} / {}</span>
          <span class="diff-pill">new {} / {}</span>
        </div>
        <p class="diff-hunk-summary"><span class="derived-label">derived/hunk summary</span> {}</p>
        {}
        {}
      </section>"#,
                html_escape(&hunk_anchor),
                html_escape(&heading),
                html_escape(&old_start),
                html_escape(&old_lines),
                html_escape(&new_start),
                html_escape(&new_lines),
                html_escape(&summary_ko),
                lines,
                explanations
            )
        })
        .collect::<Vec<_>>()
        .join("\n      ")
}

fn render_code_change_lines(hunk: &str) -> String {
    let lines = json_objects_from_field(hunk, "lines");
    if lines.is_empty() {
        return r#"<div class="code-viewport"><div class="diff-table">
          <div class="diff-line context"><span class="line-no"></span><span class="line-no"></span><code>hunk lines가 비어 있습니다.</code></div>
        </div></div>"#
            .to_owned();
    }

    let rows = lines
        .iter()
        .map(|line| {
            let kind = normalized_diff_line_kind(
                json_text_field(line, "kind").unwrap_or_else(|| "context".to_owned()).as_str(),
            );
            let marker = match kind {
                "add" => "+",
                "remove" => "-",
                _ => " ",
            };
            let old_line = json_line_number_field(line, "old_line");
            let new_line = json_line_number_field(line, "new_line");
            let content = json_text_field(line, "content").unwrap_or_default();
            format!(
                r#"<div class="diff-line {}"><span class="line-no">{}</span><span class="line-no">{}</span><code>{}{}</code></div>"#,
                kind,
                html_escape(&old_line),
                html_escape(&new_line),
                marker,
                html_escape(&content)
            )
        })
        .collect::<Vec<_>>()
        .join("\n          ");
    format!(
        r#"<div class="code-viewport"><div class="diff-table">
          {rows}
        </div></div>"#,
    )
}

fn render_code_change_explanations(hunk: &str) -> String {
    let explanations = json_objects_from_field(hunk, "explanations");
    if explanations.is_empty() {
        return String::new();
    }

    let items = explanations
        .iter()
        .map(|explanation| {
            let target = json_text_field(explanation, "line_ref")
                .or_else(|| json_text_field(explanation, "range_ref"))
                .unwrap_or_else(|| "hunk".to_owned());
            let text_ko =
                json_text_field(explanation, "text_ko").unwrap_or_else(|| "설명 없음".to_owned());
            format!(
                r"<div><strong>{}</strong>: {}</div>",
                html_escape(&target),
                html_escape(&text_ko)
            )
        })
        .collect::<Vec<_>>()
        .join("\n          ");
    format!(
        r#"<div class="diff-explanations">
          {items}
        </div>"#,
    )
}

fn normalized_diff_line_kind(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "add" | "added" | "+" => "add",
        "remove" | "removed" | "delete" | "deleted" | "-" => "remove",
        _ => "context",
    }
}

fn code_change_objects(report_json: &str) -> Vec<String> {
    json_objects_from_field(report_json, "code_changes")
}

fn json_objects_from_field(input: &str, key: &str) -> Vec<String> {
    json_value_field_snippet(input, key)
        .map(|value| {
            if value.trim_start().starts_with('{') || value.trim_start().starts_with('[') {
                json_object_slices(&value).into_iter().map(str::to_owned).collect()
            } else {
                Vec::new()
            }
        })
        .unwrap_or_default()
}

fn json_text_field(object: &str, key: &str) -> Option<String> {
    let value = json_value_field_snippet(object, key)?;
    let trimmed = value.trim();
    if trimmed == "null" { None } else { Some(json_value_to_display_text(trimmed)) }
}

fn json_line_number_field(object: &str, key: &str) -> String {
    json_text_field(object, key).filter(|value| value != "null").unwrap_or_default()
}

fn json_string_array_field_values(object: &str, key: &str) -> Vec<String> {
    let Some(value) = json_value_field_snippet(object, key) else {
        return Vec::new();
    };
    let value = value.trim();
    if !value.starts_with('[') {
        return Vec::new();
    }

    let mut values = Vec::new();
    let mut rest = &value[1..];
    loop {
        let Some(start) = rest.find('"') else {
            break;
        };
        let slice = &rest[start..];
        let Some(end) = json_string_slice_end(slice) else {
            break;
        };
        if let Some(parsed) = parse_json_string_value(slice) {
            values.push(parsed);
        }
        rest = &slice[end..];
    }
    values
}

fn render_artifact_details(artifact: &CycleReportArtifact) -> String {
    let report_json_link = format!(
        r#"<details open>
      <summary>report.json · <a href="/api/reports/{}/report.json">원문 열기</a></summary>
      <p class="missing">메인 HTML은 report.json 안의 code_changes 전체 hunk를 중복 렌더링하지 않습니다. 구조화 report 원문은 링크로 열고, diff hunk는 diff.html에서 확인합니다.</p>
    </details>"#,
        url_component_encode(&artifact.cycle_id),
    );
    let other_artifacts = [
        ("raw.json", artifact.raw_json.as_deref()),
        ("audit.json", artifact.audit_json.as_deref()),
        ("context.md", artifact.context_markdown.as_deref()),
    ]
    .iter()
    .map(|(name, value)| {
        let body = value.map_or_else(
            || format!(r#"<p class="missing">{name} 파일이 없습니다.</p>"#),
            |content| format!("<pre>{}</pre>", html_escape(content)),
        );
        format!(
            r#"<details open>
      <summary>{} · <a href="/api/reports/{}/{}">원문 열기</a></summary>
      {}
    </details>"#,
            html_escape(name),
            url_component_encode(&artifact.cycle_id),
            url_component_encode(name),
            body
        )
    })
    .collect::<Vec<_>>()
    .join("\n    ");
    format!("{report_json_link}\n    {other_artifacts}")
}

fn html_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Renders the static HTML shell for the dev console.
#[must_use]
pub fn render_console_html(cycle_id: &str, initial_report_json: Option<&str>) -> String {
    render_console_html_ko(cycle_id, initial_report_json)
}

#[must_use]
#[allow(clippy::too_many_lines, dead_code)]
fn render_console_html_legacy(cycle_id: &str, initial_report_json: Option<&str>) -> String {
    let initial_report = javascript_safe_json_literal(initial_report_json.unwrap_or("null"));
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Xavi Dev Console</title>
<style>
:root {{
  color-scheme: light;
  --bg: #f6f7f9;
  --panel: #ffffff;
  --ink: #20242b;
  --muted: #646b76;
  --line: #d9dee6;
  --accent: #146c5f;
  --accent-2: #6f4e9b;
  --warn: #9a5b00;
  --good: #186b3b;
}}
* {{ box-sizing: border-box; }}
body {{
  margin: 0;
  min-height: 100vh;
  background: var(--bg);
  color: var(--ink);
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 14px;
  letter-spacing: 0;
}}
button, input, select, textarea {{ font: inherit; }}
.shell {{ min-height: 100vh; display: grid; grid-template-rows: auto 1fr; }}
.topbar {{
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 20px;
  border-bottom: 1px solid var(--line);
  background: #ffffff;
}}
.brand {{ display: flex; align-items: baseline; gap: 12px; min-width: 0; }}
.brand h1 {{ margin: 0; font-size: 18px; font-weight: 700; white-space: nowrap; }}
.brand span {{ color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
.top-actions {{ display: flex; gap: 8px; align-items: center; }}
.btn {{
  min-height: 34px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  color: var(--ink);
  padding: 0 12px;
  cursor: pointer;
}}
.btn.primary {{ background: var(--accent); border-color: var(--accent); color: #ffffff; }}
.mode-badge {{
  display: inline-flex;
  align-items: center;
  min-height: 26px;
  border: 1px solid #cfd9ec;
  border-radius: 999px;
  padding: 0 10px;
  background: #eef4ff;
  color: #25496f;
  font-size: 12px;
  white-space: nowrap;
}}
.live-status {{ color: var(--muted); font-size: 12px; white-space: nowrap; }}
.layout {{
  display: grid;
  grid-template-columns: minmax(280px, 360px) minmax(0, 1fr);
  gap: 0;
  min-height: 0;
}}
.side {{
  border-right: 1px solid var(--line);
  background: #ffffff;
  padding: 16px;
  overflow: auto;
}}
.main {{ padding: 16px 20px 24px; overflow: auto; }}
.section {{ margin-bottom: 18px; }}
.section h2 {{ margin: 0 0 10px; font-size: 14px; font-weight: 700; color: #30343a; }}
.metrics {{ display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }}
.metric {{
  min-height: 64px;
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 10px;
  background: #fbfcfd;
}}
.metric b {{ display: block; font-size: 20px; line-height: 1; }}
.metric span {{ color: var(--muted); font-size: 12px; }}
.input-panel {{
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #fbfcfd;
  padding: 12px;
}}
.input-panel label {{ display: block; color: var(--muted); font-size: 12px; margin-bottom: 6px; }}
.input-panel textarea {{
  width: 100%;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  color: var(--ink);
  padding: 8px;
}}
.input-panel textarea {{ min-height: 190px; resize: vertical; margin-top: 8px; line-height: 1.5; }}
.input-row {{ display: flex; gap: 8px; margin-top: 10px; align-items: center; }}
.status {{ color: var(--muted); font-size: 12px; min-height: 18px; }}
.timeline {{ display: grid; gap: 8px; }}
.entry {{
  display: grid;
  grid-template-columns: 128px minmax(0, 1fr);
  gap: 12px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--panel);
  padding: 12px;
}}
.entry-meta {{ color: var(--muted); font-size: 12px; overflow-wrap: anywhere; }}
.entry-main h3 {{ margin: 0 0 6px; font-size: 14px; }}
.entry-main p {{ margin: 0; color: #3b4048; overflow-wrap: anywhere; }}
.tag {{
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  border-radius: 999px;
  padding: 0 8px;
  border: 1px solid var(--line);
  background: #f4f6f8;
  color: #343941;
  font-size: 12px;
}}
.tag.agent_dispatch {{ border-color: #cfd9ec; background: #eef4ff; color: #25496f; }}
.tag.agent_return {{ border-color: #cfe2d5; background: #eef8f1; color: var(--good); }}
.tag.orchestra_judgment {{ border-color: #ded4ef; background: #f6f0ff; color: var(--accent-2); }}
.tag.test_summary {{ border-color: #edd9b5; background: #fff6e8; color: var(--warn); }}
.role-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(178px, 1fr)); gap: 8px; }}
.role-lane {{
  min-height: 112px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  padding: 10px;
  display: grid;
  align-content: start;
  gap: 6px;
}}
.role-lane strong {{ font-size: 13px; }}
.role-lane .lane-summary {{ color: #3b4048; font-size: 12px; overflow-wrap: anywhere; }}
.role-lane.idle {{ background: #f9fafb; color: var(--muted); }}
.role-lane.running {{ border-color: #bdd7d0; background: #eef9f6; }}
.role-lane.returned {{ border-color: #cfe2d5; background: #f2fbf5; }}
.role-lane.queued {{ border-color: #d9cfe8; background: #f8f2ff; }}
.decision-grid {{ display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }}
.decision-panel {{
  min-height: 128px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  padding: 10px;
}}
.decision-panel h3 {{ margin: 0 0 8px; font-size: 13px; }}
.decision-list {{ display: grid; gap: 8px; }}
.decision-item {{ color: #3b4048; font-size: 12px; overflow-wrap: anywhere; }}
.decision-item time {{ display: block; color: var(--muted); margin-bottom: 2px; }}
.empty {{
  min-height: 160px;
  display: grid;
  place-items: center;
  border: 1px dashed var(--line);
  border-radius: 6px;
  color: var(--muted);
  background: #ffffff;
}}
@media (max-width: 860px) {{
  .topbar {{ height: auto; min-height: 56px; align-items: flex-start; flex-direction: column; padding: 12px 14px; }}
  .top-actions {{ width: 100%; }}
  .top-actions .btn {{ flex: 1; }}
  .layout {{ grid-template-columns: 1fr; }}
  .side {{ border-right: 0; border-bottom: 1px solid var(--line); }}
  .entry {{ grid-template-columns: 1fr; }}
  .decision-grid {{ grid-template-columns: 1fr; }}
}}
</style>
</head>
<body>
<div class="shell">
  <header class="topbar">
    <div class="brand">
      <h1>Xavi Dev Console</h1>
      <span id="cycle-label"></span>
    </div>
    <div class="top-actions">
      <span class="mode-badge">trace/readback mode</span>
      <span class="live-status" id="live-status">SSE disconnected</span>
      <button class="btn" type="button" id="live-button">Start SSE</button>
      <button class="btn" type="button" id="refresh-button">Refresh</button>
      <button class="btn primary" type="button" id="export-button">JSON</button>
    </div>
  </header>
  <div class="layout">
    <aside class="side">
      <section class="section">
        <h2>Cycle</h2>
        <div class="metrics" id="metrics"></div>
      </section>
      <section class="section">
        <h2>Terminal Prompt Draft</h2>
        <div class="input-panel" id="prompt-panel">
          <label for="message">Copy-only prompt</label>
          <textarea id="message" maxlength="6000" spellcheck="false"></textarea>
          <div class="input-row">
            <button class="btn primary" type="button" id="copy-prompt-button">Copy Prompt</button>
            <button class="btn" type="button" id="select-prompt-button">Select All</button>
            <span class="status" id="input-status"></span>
          </div>
        </div>
      </section>
      <section class="section">
        <h2>Realtime</h2>
        <div class="input-panel">
          <label>Visible Logs</label>
          <p class="status">Shows public trace judgments, role instructions, review notes, and state-transition reasons.</p>
        </div>
      </section>
      <section class="section">
        <h2>Console Input Records</h2>
        <div class="timeline" id="pending"></div>
      </section>
    </aside>
    <main class="main">
      <section class="section">
        <h2>Role Lanes</h2>
        <div class="role-grid" id="role-lanes"></div>
      </section>
      <section class="section">
        <h2>Realtime State</h2>
        <div class="decision-grid">
          <div class="decision-panel">
            <h3>판단 로그</h3>
            <div class="decision-list" id="judgment-log"></div>
          </div>
          <div class="decision-panel">
            <h3>역할 지시</h3>
            <div class="decision-list" id="dispatch-log"></div>
          </div>
          <div class="decision-panel">
            <h3>검수 메모</h3>
            <div class="decision-list" id="review-log"></div>
          </div>
          <div class="decision-panel">
            <h3>상태 전이 이유</h3>
            <div class="decision-list" id="state-log"></div>
          </div>
        </div>
      </section>
      <section class="section">
        <h2>Timeline</h2>
        <div class="timeline" id="timeline"></div>
      </section>
    </main>
  </div>
</div>
<script>
const DEFAULT_CYCLE = {cycle_json};
window.__INITIAL_REPORT__ = {initial_report};
const params = new URLSearchParams(window.location.search);
let cycleId = params.get('cycle') || DEFAULT_CYCLE;
let latestReport = window.__INITIAL_REPORT__;
let traceRefreshInFlight = false;
let traceRefreshPending = false;
let traceRefreshDebounceTimer = null;
let liveEventSource = null;
const TRACE_REFRESH_DEBOUNCE_MS = 250;

function text(value) {{
  return value == null ? '' : String(value);
}}

function setStatus(message) {{
  document.getElementById('input-status').textContent = message;
}}

function metric(label, value) {{
  return `<div class="metric"><b>${{value}}</b><span>${{label}}</span></div>`;
}}

function roleLane(lane) {{
  const status = lane.status || 'idle';
  const summary = lane.latest_summary ? escapeHtml(lane.latest_summary) : 'No public trace events yet';
  return `<div class="role-lane ${{escapeHtml(status)}}">
    <strong>${{escapeHtml(lane.role)}}</strong>
    <span class="tag">${{escapeHtml(status)}}</span>
    <div class="lane-summary">${{summary}}</div>
    <div class="entry-meta">${{lane.entry_count || 0}} events</div>
  </div>`;
}}

function renderEntry(entry) {{
  const role = entry.role_name || entry.role || '-';
  return `<article class="entry">
    <div class="entry-meta">
      <span class="tag ${{entry.kind}}">${{entry.kind}}</span><br>
      #${{entry.id}}<br>${{entry.created_at}}<br>${{role}}
    </div>
    <div class="entry-main">
      <h3>${{escapeHtml(entry.summary)}}</h3>
      <p>${{escapeHtml(entry.public_excerpt || entry.summary || '')}}</p>
    </div>
  </article>`;
}}

function escapeHtml(value) {{
  return text(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}}

function decisionItem(entry) {{
  return `<div class="decision-item">
    <time>${{escapeHtml(entry.created_at || '')}} · ${{escapeHtml(entry.role_name || entry.kind || '')}}</time>
    ${{escapeHtml(entry.summary || '')}}
  </div>`;
}}

function renderDecisionList(id, entries) {{
  document.getElementById(id).innerHTML = entries.length
    ? entries.slice(-4).reverse().map(decisionItem).join('')
    : '<div class="decision-item">No public trace events</div>';
}}

function roleIs(entry, role) {{
  return text(entry.role_name).toLowerCase().replaceAll('_', '-') === role;
}}

function renderDecisionSections(entries) {{
  renderDecisionList('judgment-log', entries.filter(entry => entry.kind === 'orchestra_judgment'));
  renderDecisionList('dispatch-log', entries.filter(entry => entry.kind === 'agent_dispatch'));
  renderDecisionList('review-log', entries.filter(entry => roleIs(entry, 'review') || text(entry.summary).toLowerCase().includes('review')));
  renderDecisionList('state-log', entries.filter(entry => entry.kind === 'test_summary' || entry.kind === 'agent_return' || entry.kind === 'orchestra_judgment'));
}}

function render(report) {{
  latestReport = report;
  document.getElementById('cycle-label').textContent = report.cycle_id;
  const counts = report.counts || {{}};
  document.getElementById('metrics').innerHTML =
    metric('Shown', report.displayed_entry_count ?? report.entry_count ?? 0) +
    metric('Total', report.total_entry_count ?? report.entry_count ?? 0) +
    metric('User', counts.user_query || 0) +
    metric('Dispatch', counts.agent_dispatch || 0) +
    metric('Return', counts.agent_return || 0) +
    metric('Tests', counts.test_summary || 0) +
    metric('Console Input', (report.pending_inputs || []).length);
  const entries = report.entries || [];
  const lanes = report.role_lanes || [];
  document.getElementById('role-lanes').innerHTML = lanes.length
    ? lanes.map(roleLane).join('')
    : '<div class="empty">No role lanes</div>';
  renderDecisionSections(entries);
  document.getElementById('timeline').innerHTML = entries.length
    ? entries.map(renderEntry).join('')
    : '<div class="empty">No trace entries</div>';
  const pending = report.pending_inputs || [];
  document.getElementById('pending').innerHTML = pending.length
    ? pending.map(renderEntry).join('')
    : '<div class="empty">No console input records</div>';
}}

function setLiveStatus(message) {{
  document.getElementById('live-status').textContent = message;
}}

async function refresh() {{
  const response = await fetch(`/api/cycles/${{encodeURIComponent(cycleId)}}`);
  if (!response.ok) throw new Error(await response.text());
  render(await response.json());
}}

function requestTraceRefresh() {{
  traceRefreshPending = true;
  if (traceRefreshInFlight || traceRefreshDebounceTimer !== null) return;
  traceRefreshDebounceTimer = setTimeout(runTraceRefresh, TRACE_REFRESH_DEBOUNCE_MS);
}}

function runTraceRefresh() {{
  traceRefreshDebounceTimer = null;
  if (!traceRefreshPending || traceRefreshInFlight) return;
  traceRefreshPending = false;
  traceRefreshInFlight = true;
  refresh()
    .catch(error => setLiveStatus(error.message))
    .finally(() => {{
      traceRefreshInFlight = false;
      if (traceRefreshPending) requestTraceRefresh();
    }});
}}

function connectEvents() {{
  if (liveEventSource) {{
    setLiveStatus('SSE already connected');
    return;
  }}
  if (!('EventSource' in window)) {{
    setLiveStatus('SSE unavailable');
    return;
  }}
  liveEventSource = new EventSource(`/api/cycles/${{encodeURIComponent(cycleId)}}/events`);
  liveEventSource.onopen = () => setLiveStatus('SSE connected');
  liveEventSource.addEventListener('trace', event => {{
    const trace = JSON.parse(event.data);
    setLiveStatus(`SSE trace #${{trace.sequence}} · ${{trace.role}}`);
    requestTraceRefresh();
  }});
  liveEventSource.onerror = () => setLiveStatus('SSE reconnecting');
}}

document.getElementById('live-button').addEventListener('click', connectEvents);

document.getElementById('refresh-button').addEventListener('click', () => {{
  refresh().catch(error => setStatus(error.message));
}});

document.getElementById('export-button').addEventListener('click', () => {{
  const blob = new Blob([JSON.stringify(latestReport, null, 2)], {{ type: 'application/json' }});
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `${{cycleId}}.json`;
  anchor.click();
  URL.revokeObjectURL(url);
}});

function selectPromptText() {{
  const textarea = document.getElementById('message');
  textarea.focus();
  textarea.select();
  setStatus('Prompt text selected');
}}

async function copyPromptText() {{
  const textarea = document.getElementById('message');
  const value = textarea.value.trim();
  if (!value) {{
    setStatus('Select a trace item first');
    return;
  }}
  try {{
    await navigator.clipboard.writeText(value);
    setStatus('Prompt copied');
  }} catch (_error) {{
    selectPromptText();
    setStatus('Clipboard copy failed; text selected');
  }}
}}

document.getElementById('copy-prompt-button').addEventListener('click', () => {{
  copyPromptText().catch(error => setStatus(error.message));
}});

document.getElementById('select-prompt-button').addEventListener('click', selectPromptText);

if (latestReport) {{
  render(latestReport);
}} else {{
  refresh().catch(error => {{
    document.getElementById('cycle-label').textContent = cycleId;
    setStatus(error.message);
  }});
}}
setLiveStatus('Manual refresh mode');
</script>
</body>
</html>"#,
        cycle_json = json_string(cycle_id),
        initial_report = initial_report
    )
}

#[allow(clippy::too_many_lines)]
fn render_console_html_ko(cycle_id: &str, initial_report_json: Option<&str>) -> String {
    let initial_report = javascript_safe_json_literal(initial_report_json.unwrap_or("null"));
    let mut html = r#"<!doctype html>
<html lang="ko">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Xavi 개발 콘솔</title>
<style>
:root {
  color-scheme: light;
  --bg: #f4f5f7;
  --panel: #ffffff;
  --ink: #20242b;
  --muted: #646b76;
  --line: #d9dee6;
  --accent: #146c5f;
  --accent-soft: #eaf7f3;
  --blue: #2d5c88;
  --blue-soft: #eef4fb;
  --violet: #7250a3;
  --violet-soft: #f4effb;
  --warn: #94610b;
  --warn-soft: #fff6e3;
  --good: #186b3b;
  --good-soft: #eef8f1;
  --danger: #9f3434;
  --danger-soft: #fff1f1;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  min-height: 100vh;
  background: var(--bg);
  color: var(--ink);
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 14px;
  letter-spacing: 0;
}
button, input, select, textarea { font: inherit; }
button { text-align: left; }
.shell { min-height: 100vh; display: grid; grid-template-rows: auto 1fr; }
.topbar {
  min-height: 58px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 20px;
  border-bottom: 1px solid var(--line);
  background: #ffffff;
}
.brand { display: flex; align-items: baseline; gap: 12px; min-width: 0; }
.brand h1 { margin: 0; font-size: 18px; font-weight: 700; white-space: nowrap; }
.brand span { color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.top-actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
.btn {
  display: inline-flex;
  align-items: center;
  min-height: 34px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  color: var(--ink);
  padding: 0 12px;
  cursor: pointer;
  text-decoration: none;
}
.btn.primary { background: var(--accent); border-color: var(--accent); color: #ffffff; justify-content: center; text-align: center; }
.mode-badge {
  display: inline-flex;
  align-items: center;
  min-height: 26px;
  border: 1px solid #cfd9ec;
  border-radius: 999px;
  padding: 0 10px;
  background: var(--blue-soft);
  color: var(--blue);
  font-size: 12px;
  white-space: nowrap;
}
.live-status { color: var(--muted); font-size: 12px; white-space: nowrap; }
.workspace {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(320px, 390px);
  gap: 20px;
  padding: 18px 20px 28px;
  min-height: 0;
}
.main { min-width: 0; display: grid; gap: 20px; align-content: start; }
.request-pane { min-width: 0; display: grid; gap: 18px; align-content: start; position: sticky; top: 12px; }
.section { min-width: 0; }
.section h2 {
  margin: 0 0 10px;
  font-size: 16px;
  font-weight: 750;
  color: #30343a;
}
.metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; }
.metric {
  min-height: 64px;
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 10px;
  background: #fbfcfd;
}
.metric b { display: block; font-size: 20px; line-height: 1; }
.metric span { color: var(--muted); font-size: 12px; }
.request-panel,
.info-panel {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #fbfcfd;
  padding: 12px;
}
.request-panel label,
.info-panel label { display: block; color: var(--muted); font-size: 12px; margin: 10px 0 6px; }
.request-panel label:first-child,
.info-panel label:first-child { margin-top: 0; }
.request-panel textarea {
  width: 100%;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  color: var(--ink);
  padding: 8px;
}
.request-panel textarea { min-height: 420px; resize: vertical; line-height: 1.5; }
.request-row { display: flex; gap: 8px; margin-top: 10px; align-items: center; flex-wrap: wrap; }
.status { color: var(--muted); font-size: 12px; min-height: 18px; }
.selected-log {
  min-height: 40px;
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 8px;
  background: #ffffff;
  color: #343941;
  font-size: 12px;
  overflow-wrap: anywhere;
}
.selected-log.active { border-color: var(--accent); background: var(--accent-soft); }
.quality-note { color: var(--warn); font-size: 12px; overflow-wrap: anywhere; }
.process-map { display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 8px; }
.map-node,
.entry,
.role-lane {
  cursor: pointer;
  appearance: none;
}
.map-node {
  min-height: 112px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--panel);
  padding: 10px;
  display: grid;
  gap: 6px;
  color: var(--ink);
}
.map-node strong { font-size: 13px; }
.map-node .node-time { color: var(--muted); font-size: 12px; overflow-wrap: anywhere; }
.map-node .node-summary { color: #3b4048; font-size: 12px; overflow-wrap: anywhere; }
.timeline { display: grid; gap: 8px; }
.entry {
  display: grid;
  grid-template-columns: 128px minmax(0, 1fr);
  gap: 12px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--panel);
  padding: 12px;
  color: var(--ink);
  width: 100%;
}
.entry-meta { color: var(--muted); font-size: 12px; overflow-wrap: anywhere; }
.entry-main h3 { margin: 0 0 6px; font-size: 14px; }
.entry-main p { margin: 0; color: #3b4048; overflow-wrap: anywhere; }
.entry.compact { grid-template-columns: 1fr; min-height: 98px; }
.tag {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  border-radius: 999px;
  padding: 0 8px;
  border: 1px solid var(--line);
  background: #f4f6f8;
  color: #343941;
  font-size: 12px;
}
.tag.agent_dispatch { border-color: #cfd9ec; background: var(--blue-soft); color: var(--blue); }
.tag.agent_return { border-color: #cfe2d5; background: var(--good-soft); color: var(--good); }
.tag.orchestra_judgment { border-color: #ded4ef; background: var(--violet-soft); color: var(--violet); }
.tag.test_summary { border-color: #edd9b5; background: var(--warn-soft); color: var(--warn); }
.tag.user_query { border-color: #dfc8c8; background: var(--danger-soft); color: var(--danger); }
.role-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(178px, 1fr)); gap: 8px; }
.role-lane {
  min-height: 112px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  padding: 10px;
  display: grid;
  align-content: start;
  gap: 6px;
  color: var(--ink);
  width: 100%;
}
.role-lane strong { font-size: 13px; }
.role-lane .lane-summary { color: #3b4048; font-size: 12px; overflow-wrap: anywhere; }
.role-lane.idle { background: #f9fafb; color: var(--muted); }
.role-lane.running { border-color: #bdd7d0; background: #eef9f6; }
.role-lane.returned { border-color: #cfe2d5; background: #f2fbf5; }
.role-lane.queued { border-color: #d9cfe8; background: #f8f2ff; }
.role-lane.accepted { border-color: #ded4ef; background: var(--violet-soft); }
.role-lane.complete { border-color: #cfe2d5; background: var(--good-soft); }
.role-lane:disabled { cursor: default; opacity: 0.72; }
.decision-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.decision-panel {
  min-height: 128px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  padding: 10px;
}
.decision-panel h3 { margin: 0 0 8px; font-size: 13px; }
.decision-list { display: grid; gap: 8px; }
.decision-item {
  width: 100%;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #ffffff;
  padding: 8px;
  color: #3b4048;
  font-size: 12px;
  overflow-wrap: anywhere;
}
.decision-item time { display: block; color: var(--muted); margin-bottom: 2px; }
.trace-select.selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(20, 108, 95, 0.16);
  background: var(--accent-soft);
}
.raw-trace {
  border-top: 1px solid var(--line);
  padding-top: 12px;
}
.raw-trace summary {
  cursor: pointer;
  font-weight: 750;
  font-size: 16px;
  color: #30343a;
  margin-bottom: 10px;
}
.empty {
  min-height: 160px;
  display: grid;
  place-items: center;
  border: 1px dashed var(--line);
  border-radius: 6px;
  color: var(--muted);
  background: #ffffff;
}
.empty.small { min-height: 72px; }
@media (max-width: 860px) {
  .topbar { height: auto; min-height: 56px; align-items: flex-start; flex-direction: column; padding: 12px 14px; }
  .top-actions { width: 100%; }
  .top-actions .btn { flex: 1; }
  .workspace { grid-template-columns: 1fr; padding: 14px; }
  .request-pane { position: static; }
  .metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .entry { grid-template-columns: 1fr; }
  .decision-grid { grid-template-columns: 1fr; }
}
</style>
</head>
<body>
<div class="shell">
  <header class="topbar">
    <div class="brand">
      <h1>Xavi 개발 콘솔</h1>
      <span id="cycle-label"></span>
    </div>
    <div class="top-actions">
      <span class="mode-badge">공개 trace 확인</span>
      <span class="live-status" id="live-status">실시간 연결 대기</span>
      <a class="btn" href="/reports">사이클 보고서</a>
      <button class="btn" type="button" id="live-button">실시간 시작</button>
      <button class="btn" type="button" id="refresh-button">새로고침</button>
      <button class="btn primary" type="button" id="export-button">JSON 내보내기</button>
    </div>
  </header>
  <div class="workspace">
    <main class="main">
      <section class="section">
        <h2>사이클 개요</h2>
        <div class="metrics" id="metrics"></div>
      </section>
      <section class="section">
        <h2>Trace 무결성</h2>
        <div class="decision-list" id="trace-integrity"></div>
      </section>
      <section class="section">
        <h2>작업 사이클 지도</h2>
        <div class="process-map" id="cycle-map"></div>
      </section>
      <section class="section">
        <h2>오케스트라 조율</h2>
        <div class="decision-grid">
          <div class="decision-panel">
            <h3>판단 로그</h3>
            <div class="decision-list" id="judgment-log"></div>
          </div>
          <div class="decision-panel">
            <h3>역할 지시</h3>
            <div class="decision-list" id="dispatch-log"></div>
          </div>
        </div>
      </section>
      <section class="section">
        <h2>역할별 작업 보드</h2>
        <div class="role-grid" id="role-lanes"></div>
      </section>
      <section class="section">
        <h2>검수와 재시도 흐름</h2>
        <div class="timeline" id="review-flow"></div>
      </section>
      <section class="section">
        <h2>검증 결과</h2>
        <div class="timeline" id="test-results"></div>
      </section>
      <section class="section">
        <h2>현재 제한 / 다음 단계</h2>
        <div class="decision-grid">
          <div class="decision-panel">
            <h3>기존 콘솔 입력 기록</h3>
            <div class="decision-list" id="pending"></div>
          </div>
        </div>
      </section>
      <details class="section raw-trace">
        <summary>원본 이벤트 흐름</summary>
        <div class="timeline" id="timeline"></div>
      </details>
    </main>
    <aside class="request-pane">
      <section class="section">
        <h2>선택 로그 복사용 정보</h2>
        <div class="request-panel" id="prompt-panel">
          <label for="selected-log">선택한 공개 로그</label>
          <div class="selected-log" id="selected-log">선택된 공개 로그 없음</div>
          <label for="message">복사할 정보</label>
          <textarea id="message" maxlength="6000" spellcheck="false"></textarea>
          <div class="request-row">
            <button class="btn primary" type="button" id="copy-prompt-button">정보 복사</button>
            <button class="btn" type="button" id="select-prompt-button">전체 선택</button>
            <span class="status" id="input-status"></span>
          </div>
        </div>
      </section>
      <section class="section">
        <h2>공개 반환 요약</h2>
        <div class="info-panel">
          <label>검수 메모</label>
          <div class="decision-list" id="review-log"></div>
          <label>상태 전이 이유</label>
          <div class="decision-list" id="state-log"></div>
        </div>
      </section>
    </aside>
  </div>
</div>
<script>
const DEFAULT_CYCLE = __CYCLE_JSON__;
window.__INITIAL_REPORT__ = __INITIAL_REPORT_JSON__;
const params = new URLSearchParams(window.location.search);
let cycleId = params.get('cycle') || DEFAULT_CYCLE;
let latestReport = window.__INITIAL_REPORT__;
let selectedEntryRowId = null;
let traceRefreshInFlight = false;
let traceRefreshPending = false;
let traceRefreshDebounceTimer = null;
let liveEventSource = null;
const TRACE_REFRESH_DEBOUNCE_MS = 250;

const ROLE_LABELS = {
  orchestra: '오케스트라',
  planning: '기획',
  codegen: '구현',
  review: '검수',
  test: '검증',
  analysis: '원인 분석',
  'user-docs': '사용자 문서',
  'ai-docs': 'AI 문서',
  'cycle-report': '사이클 보고서',
  'dev-console': '개발 콘솔',
  user: '사용자'
};

const KIND_LABELS = {
  user_query: '사용자 입력',
  orchestra_judgment: '오케스트라 판단',
  agent_dispatch: '역할 지시',
  agent_return: '역할 반환',
  file_summary: '파일 변경 요약',
  test_summary: '검증 요약',
  project_knowledge_note: '프로젝트 지식'
};

const STATUS_LABELS = {
  idle: '공개 trace 없음',
  queued: '입력 기록',
  accepted: '수락됨',
  running: '진행 중',
  returned: '반환됨',
  complete: '완료',
  passed: '통과',
  failed: '실패',
  warning: '경고',
  invalid: '무효'
};

function text(value) {
  return value == null ? '' : String(value);
}

function setStatus(message) {
  document.getElementById('input-status').textContent = message;
}

function metric(label, value) {
  return `<div class="metric"><b>${value}</b><span>${label}</span></div>`;
}

function emptyState(message, compact = false) {
  return `<div class="empty ${compact ? 'small' : ''}">${escapeHtml(message)}</div>`;
}

function roleKey(value) {
  return text(value).trim().toLowerCase().replaceAll('_', '-');
}

function roleLabel(value) {
  const key = roleKey(value);
  return ROLE_LABELS[key] || text(value || '-');
}

function kindLabel(value) {
  return KIND_LABELS[text(value)] || text(value || '-');
}

function statusLabel(value) {
  return STATUS_LABELS[text(value)] || text(value || '-');
}

function isConsoleInput(entry) {
  return entry.is_console_input === true;
}

function roleForEntry(entry) {
  const explicitRole = roleKey(entry.role || entry.role_name);
  if (ROLE_LABELS[explicitRole]) return explicitRole;
  if (isConsoleInput(entry)) return 'dev-console';
  if (entry.kind === 'test_summary') return 'test';
  return 'orchestra';
}

function roleSourceLabel(value) {
  switch (text(value)) {
    case 'explicit': return '명시 역할';
    case 'inferred_from_event_id': return '추정: event_id prefix, 원본 role_name 없음';
    case 'inferred_from_summary': return '추정: summary prefix, 원본 role_name 없음';
    case 'console_input': return '개발 콘솔 입력';
    case 'kind_default': return 'kind 기준 기본 분류';
    case 'missing_role_metadata': return '역할 기록 없음';
    default: return text(value || '역할 출처 없음');
  }
}

function roleQuality(entry) {
  return roleSourceLabel(entry.role_source);
}

function roleDisplay(entry) {
  const role = roleLabel(roleForEntry(entry));
  const source = text(entry.role_source);
  if (source === 'inferred_from_event_id' || source === 'inferred_from_summary') {
    return `${role} (${roleQuality(entry)})`;
  }
  if (source === 'missing_role_metadata' && !entry.role_name) {
    return `${role} (${roleQuality(entry)})`;
  }
  return role;
}

function statusForEntry(entry) {
  if (entry.status) return entry.status;
  switch (entry.kind) {
    case 'user_query': return 'queued';
    case 'orchestra_judgment': return 'accepted';
    case 'agent_dispatch': return 'running';
    case 'agent_return': return 'returned';
    default: return 'complete';
  }
}

function roleLane(lane) {
  const status = lane.status || 'idle';
  const summary = lane.latest_summary
    ? escapeHtml(lane.latest_summary)
    : escapeHtml(lane.role === 'test' ? 'test_summary 없음 / test 역할 trace 없음' : '공개 trace 없음');
  const rowId = lane.latest_entry_id == null ? '' : text(lane.latest_entry_id);
  const selectAttrs = rowId
    ? `data-entry-id="${escapeHtml(rowId)}" data-source-label="역할별 작업 보드"`
    : 'disabled';
  const quality = lane.latest_role_source
    ? roleSourceLabel(lane.latest_role_source)
    : '공개 trace 없음';
  return `<button class="role-lane trace-select ${escapeHtml(status)}" type="button" ${selectAttrs}>
    <strong>${escapeHtml(roleLabel(lane.role))}</strong>
    <span class="tag">${escapeHtml(statusLabel(status))}</span>
    <div class="lane-summary">${summary}</div>
    <div class="entry-meta">공개 로그 ${lane.entry_count || 0}개 · ${escapeHtml(quality)}</div>
  </button>`;
}

function renderEntry(entry, sourceLabel = '원본 이벤트 흐름', compact = false) {
  const role = roleDisplay(entry);
  const status = statusLabel(statusForEntry(entry));
  return `<button class="entry trace-select ${compact ? 'compact' : ''}" type="button" data-entry-id="${escapeHtml(entry.id)}" data-source-label="${escapeHtml(sourceLabel)}">
    <div class="entry-meta">
      <span class="tag ${escapeHtml(entry.kind || '')}">${escapeHtml(kindLabel(entry.kind))}</span><br>
      #${escapeHtml(entry.id)}<br>${escapeHtml(entry.created_at || '-')}<br>${escapeHtml(role)}<br>${escapeHtml(status)}
    </div>
    <div class="entry-main">
      <h3>${escapeHtml(entry.summary)}</h3>
      <p>${escapeHtml(entry.public_excerpt || entry.summary || '')}</p>
    </div>
  </button>`;
}

function renderMapNode(entry) {
  return `<button class="map-node trace-select" type="button" data-entry-id="${escapeHtml(entry.id)}" data-source-label="작업 사이클 지도">
    <span class="tag ${escapeHtml(entry.kind || '')}">${escapeHtml(kindLabel(entry.kind))}</span>
    <strong>${escapeHtml(roleDisplay(entry))}</strong>
    <span class="node-time">${escapeHtml(entry.created_at || '-')}</span>
    <span class="node-summary">${escapeHtml(entry.summary || '')}</span>
  </button>`;
}

function escapeHtml(value) {
  return text(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function decisionItem(entry, sourceLabel) {
  return `<button class="decision-item trace-select" type="button" data-entry-id="${escapeHtml(entry.id)}" data-source-label="${escapeHtml(sourceLabel)}">
    <time>${escapeHtml(entry.created_at || '')} · ${escapeHtml(roleDisplay(entry))} · ${escapeHtml(kindLabel(entry.kind))}</time>
    ${escapeHtml(entry.summary || '')}
  </button>`;
}

function renderDecisionList(id, entries, sourceLabel) {
  document.getElementById(id).innerHTML = entries.length
    ? entries.slice(-4).reverse().map(entry => decisionItem(entry, sourceLabel)).join('')
    : emptyState('공개 trace 없음', true);
}

function roleIs(entry, role) {
  return roleForEntry(entry) === role;
}

function renderDecisionSections(entries) {
  renderDecisionList('judgment-log', entries.filter(entry => entry.kind === 'orchestra_judgment'), '오케스트라 조율');
  renderDecisionList('dispatch-log', entries.filter(entry => entry.kind === 'agent_dispatch'), '오케스트라 조율');
  renderDecisionList('review-log', entries.filter(entry => roleIs(entry, 'review') || text(entry.summary).includes('검수') || text(entry.summary).toLowerCase().includes('review')), '공개 반환 요약');
  renderDecisionList('state-log', entries.filter(entry => entry.kind === 'test_summary' || entry.kind === 'agent_return' || entry.kind === 'orchestra_judgment'), '공개 반환 요약');
}

function renderCycleMap(entries) {
  const visible = entries.slice(-10);
  document.getElementById('cycle-map').innerHTML = visible.length
    ? visible.map(renderMapNode).join('')
    : emptyState('공개 trace 없음');
}

function renderReviewFlow(entries) {
  const reviewEntries = entries.filter(entry =>
    roleIs(entry, 'review') ||
    entry.kind === 'test_summary' ||
    text(entry.summary).includes('검수') ||
    text(entry.summary).includes('재시도') ||
    text(entry.summary).toLowerCase().includes('review') ||
    text(entry.summary).toLowerCase().includes('rework')
  );
  document.getElementById('review-flow').innerHTML = reviewEntries.length
    ? reviewEntries.slice(-8).reverse().map(entry => renderEntry(entry, '검수와 재시도 흐름', true)).join('')
    : emptyState('검수 또는 재시도 공개 로그 없음');
}

function renderTestResults(entries) {
  const testEntries = entries.filter(entry => entry.kind === 'test_summary' || roleIs(entry, 'test'));
  document.getElementById('test-results').innerHTML = testEntries.length
    ? testEntries.slice(-6).reverse().map(entry => renderEntry(entry, '검증 결과', true)).join('')
    : emptyState('test_summary 없음 / test 역할 trace 없음');
}

function renderPendingInputs(pending) {
  document.getElementById('pending').innerHTML = pending.length
    ? pending.slice(-4).reverse().map(entry => decisionItem(entry, '기존 콘솔 입력 기록')).join('')
    : emptyState('기존 입력 기록 없음', true);
}

function renderTraceIntegrity(integrity) {
  const target = document.getElementById('trace-integrity');
  if (!target) return;
  if (!integrity) {
    target.innerHTML = emptyState('Trace 무결성 감사 결과 없음', true);
    return;
  }
  const findings = integrity.findings || [];
  const status = text(integrity.status || 'invalid');
  const header = `<div class="decision-item">
    <strong>상태: ${escapeHtml(statusLabel(status))}</strong><br>
    검사 로그 ${escapeHtml(integrity.checked_entry_count ?? 0)}개 · 실패 ${escapeHtml(integrity.failure_count ?? 0)}개 · 경고 ${escapeHtml(integrity.warning_count ?? 0)}개
  </div>`;
  const list = findings.length
    ? findings.map((finding, index) => `<button class="decision-item audit-finding" type="button" data-audit-finding-index="${index}">
        <time>${escapeHtml(finding.severity || '-')} · ${escapeHtml(finding.code || '-')} · ${escapeHtml(finding.event_id || 'cycle')}</time>
        ${escapeHtml(finding.message || '')}
      </button>`).join('')
    : '<div class="decision-item">필수 metadata/trace 누락 없음</div>';
  target.innerHTML = header + list;
}

function render(report) {
  latestReport = report;
  document.getElementById('cycle-label').textContent = report.cycle_id;
  const counts = report.counts || {};
  document.getElementById('metrics').innerHTML =
    metric('표시 로그', report.displayed_entry_count ?? report.entry_count ?? 0) +
    metric('전체 로그', report.total_entry_count ?? report.entry_count ?? 0) +
    metric('역할 지시', counts.agent_dispatch || 0) +
    metric('역할 반환', counts.agent_return || 0) +
    metric('검증 요약', counts.test_summary || 0) +
    metric('사용자 입력', counts.user_query || 0) +
    metric('오케스트라 판단', counts.orchestra_judgment || 0) +
    metric('콘솔 입력 기록', (report.pending_inputs || []).length);
  const entries = report.entries || [];
  const lanes = report.role_lanes || [];
  renderCycleMap(entries);
  document.getElementById('role-lanes').innerHTML = lanes.length
    ? lanes.map(roleLane).join('')
    : emptyState('역할 보드 없음');
  renderDecisionSections(entries);
  renderReviewFlow(entries);
  renderTestResults(entries);
  renderTraceIntegrity(report.trace_integrity);
  document.getElementById('timeline').innerHTML = entries.length
    ? entries.map(entry => renderEntry(entry)).join('')
    : emptyState('공개 trace 없음');
  renderPendingInputs(report.pending_inputs || []);
  markSelection();
}

function setLiveStatus(message) {
  document.getElementById('live-status').textContent = message;
}

async function refresh() {
  const response = await fetch(`/api/cycles/${encodeURIComponent(cycleId)}`);
  if (!response.ok) throw new Error(await response.text());
  render(await response.json());
}

function requestTraceRefresh() {
  traceRefreshPending = true;
  if (traceRefreshInFlight || traceRefreshDebounceTimer !== null) return;
  traceRefreshDebounceTimer = setTimeout(runTraceRefresh, TRACE_REFRESH_DEBOUNCE_MS);
}

function runTraceRefresh() {
  traceRefreshDebounceTimer = null;
  if (!traceRefreshPending || traceRefreshInFlight) return;
  traceRefreshPending = false;
  traceRefreshInFlight = true;
  refresh()
    .catch(error => setLiveStatus(error.message))
    .finally(() => {
      traceRefreshInFlight = false;
      if (traceRefreshPending) requestTraceRefresh();
    });
}

function connectEvents() {
  if (liveEventSource) {
    setLiveStatus('이미 실시간 연결 중');
    return;
  }
  if (!('EventSource' in window)) {
    setLiveStatus('실시간 연결 미지원');
    return;
  }
  liveEventSource = new EventSource(`/api/cycles/${encodeURIComponent(cycleId)}/events`);
  liveEventSource.onopen = () => setLiveStatus('실시간 연결됨');
  liveEventSource.addEventListener('trace', event => {
    const trace = JSON.parse(event.data);
    setLiveStatus(`공개 trace #${trace.sequence} · ${roleLabel(trace.role)}`);
    requestTraceRefresh();
  });
  liveEventSource.onerror = () => setLiveStatus('실시간 재연결 중');
}

document.getElementById('live-button').addEventListener('click', connectEvents);

document.getElementById('refresh-button').addEventListener('click', () => {
  refresh().catch(error => setStatus(error.message));
});

document.getElementById('export-button').addEventListener('click', () => {
  const blob = new Blob([JSON.stringify(latestReport, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `${cycleId}.json`;
  anchor.click();
  URL.revokeObjectURL(url);
});

function findEntryByRowId(rowId) {
  const entries = (latestReport && latestReport.entries) || [];
  return entries.find(entry => text(entry.id) === text(rowId)) || null;
}

function selectedLogLabel(entry, sourceLabel) {
  return `${sourceLabel} · ${roleDisplay(entry)} · ${kindLabel(entry.kind)} · event_id ${entry.event_id || entry.id}`;
}

function clipText(value, limit) {
  const current = text(value).trim();
  if (current.length <= limit) return current;
  return `${current.slice(0, Math.max(0, limit - 3))}...`;
}

function promptForEntry(entry, sourceLabel) {
  const eventId = entry.event_id || `row-${entry.id}`;
  const summary = clipText(entry.summary || '-', 360);
  const publicExcerpt = clipText(entry.public_excerpt || entry.summary || '-', 1400);
  return `선택한 공개 trace 정보
- cycle_id: ${entry.cycle_id || cycleId}
- event_id: ${eventId}
- 시간: ${entry.created_at || '-'}
- 역할: ${roleDisplay(entry)}
- 역할 출처: ${roleQuality(entry)}
- kind/단계: ${entry.kind || '-'} / ${kindLabel(entry.kind)}
- 상태: ${statusLabel(statusForEntry(entry))}
- 공개 요약: ${summary}
- 공개 본문 excerpt:
${publicExcerpt}

요청:

invalidates_after_event_id(참고용): ${eventId}
선택 위치: ${sourceLabel}`;
}

function promptForAuditFinding(finding) {
  return `선택한 Trace 무결성 감사 항목
- cycle_id: ${cycleId}
- 상태: ${finding.severity || '-'}
- code: ${finding.code || '-'}
- event_id: ${finding.event_id || '-'}
- kind: ${finding.kind || '-'}
- role: ${finding.role || '-'}
- 공개 메시지: ${finding.message || '-'}

요청:
`;
}

function selectTraceEntry(entry, sourceLabel) {
  selectedEntryRowId = text(entry.id);
  document.getElementById('message').value = promptForEntry(entry, sourceLabel);
  const selectedLog = document.getElementById('selected-log');
  selectedLog.textContent = selectedLogLabel(entry, sourceLabel);
  selectedLog.classList.add('active');
  setStatus('프롬프트 초안 준비됨');
  markSelection();
}

function selectAuditFinding(index) {
  const integrity = latestReport && latestReport.trace_integrity;
  const finding = integrity && (integrity.findings || [])[Number(index)];
  if (!finding) return;
  selectedEntryRowId = `audit-${index}`;
  document.getElementById('message').value = promptForAuditFinding(finding);
  const selectedLog = document.getElementById('selected-log');
  selectedLog.textContent = `Trace 무결성 · ${finding.severity || '-'} · ${finding.code || '-'}`;
  selectedLog.classList.add('active');
  setStatus('Trace 무결성 프롬프트 초안 준비됨');
  markSelection();
}

function markSelection() {
  document.querySelectorAll('.trace-select').forEach(node => {
    const isSelected = selectedEntryRowId && text(node.dataset.entryId) === selectedEntryRowId;
    node.classList.toggle('selected', Boolean(isSelected));
  });
  document.querySelectorAll('[data-audit-finding-index]').forEach(node => {
    const isSelected = selectedEntryRowId === `audit-${node.dataset.auditFindingIndex}`;
    node.classList.toggle('selected', Boolean(isSelected));
  });
}

document.addEventListener('click', event => {
  const auditTrigger = event.target.closest('[data-audit-finding-index]');
  if (auditTrigger) {
    selectAuditFinding(auditTrigger.dataset.auditFindingIndex);
    return;
  }
  const trigger = event.target.closest('[data-entry-id]');
  if (!trigger) return;
  const entry = findEntryByRowId(trigger.dataset.entryId);
  if (!entry) return;
  selectTraceEntry(entry, trigger.dataset.sourceLabel || '공개 trace');
});

function selectPromptText(statusMessage = '복사용 정보 전체 선택됨') {
  const textarea = document.getElementById('message');
  textarea.focus();
  textarea.select();
  setStatus(statusMessage);
}

async function copyPromptText() {
  const textarea = document.getElementById('message');
  const value = textarea.value.trim();
  if (!value) {
    setStatus('먼저 로그를 선택하세요');
    return;
  }
  try {
    await navigator.clipboard.writeText(value);
    setStatus('복사용 정보가 복사됨. 터미널에서 요청 문장을 직접 작성하세요.');
  } catch (_error) {
    selectPromptText('클립보드 복사 실패. 전체 선택했으니 직접 복사하세요.');
  }
}

document.getElementById('copy-prompt-button').addEventListener('click', () => {
  copyPromptText().catch(error => setStatus(error.message));
});

document.getElementById('select-prompt-button').addEventListener('click', () => {
  selectPromptText();
});

if (latestReport) {
  render(latestReport);
} else {
  refresh().catch(error => {
    document.getElementById('cycle-label').textContent = cycleId;
    setStatus(error.message);
  });
}
setLiveStatus('수동 새로고침 모드');
</script>
</body>
</html>"#
    .to_owned();
    html = html.replace("__CYCLE_JSON__", &json_string(cycle_id));
    html = html.replace("__INITIAL_REPORT_JSON__", &initial_report);
    html
}

fn handle_connection(
    stream: &mut TcpStream,
    config: &DevConsoleConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let request = match read_http_request(stream) {
        Ok(request) => request,
        Err(error) if error.is_payload_too_large() => {
            return write_response(
                stream,
                "413 Payload Too Large",
                "text/plain; charset=utf-8",
                "payload too large",
            );
        }
        Err(error) => return Err(Box::new(error)),
    };
    let Some((request_line, body)) = request.split_once("\r\n") else {
        return write_response(
            stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            "bad request",
        );
    };
    let parts = request_line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 {
        return write_response(
            stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            "bad request",
        );
    }
    let method = parts[0];
    let raw_path = parts[1];
    let (path, query) = split_path_query(raw_path);

    if method == "GET"
        && let Some(result) = handle_static_get_route(stream, config, &path, query)
    {
        return result;
    }

    match (method, path.as_str()) {
        // Live console projection remains trace-backed for in-progress visibility. It does not
        // replace the file-backed cycle-report artifact close path above.
        ("GET", _) if path.starts_with("/api/cycles/") && path.ends_with("/events") => {
            let Some(cycle_id) = cycle_id_from_suffixed_path(&path, "/events") else {
                return write_response(
                    stream,
                    "400 Bad Request",
                    "text/plain; charset=utf-8",
                    "bad cycle events path",
                );
            };
            let service =
                DevelopmentTraceService::new(SqliteDevelopmentTraceStore::open(&config.db_path)?);
            stream_trace_events(
                stream,
                &service,
                &cycle_id,
                config.report_limit,
                last_event_id_cursor(&request),
            )
        }
        ("GET", _) if path.starts_with("/api/cycles/") => {
            let cycle_id = percent_decode(path.trim_start_matches("/api/cycles/"));
            let service =
                DevelopmentTraceService::new(SqliteDevelopmentTraceStore::open(&config.db_path)?);
            let report = build_report(&service, &cycle_id, config.report_limit)?;
            write_response(
                stream,
                "200 OK",
                "application/json; charset=utf-8",
                &render_report_json(&report),
            )
        }
        ("POST", _) if path.starts_with("/api/cycles/") && path.ends_with("/input") => {
            let service =
                DevelopmentTraceService::new(SqliteDevelopmentTraceStore::open(&config.db_path)?);
            handle_console_input_submission(stream, &service, &path, body)
        }
        _ => write_response(stream, "404 Not Found", "text/plain; charset=utf-8", "not found"),
    }
}

fn handle_static_get_route(
    stream: &mut TcpStream,
    config: &DevConsoleConfig,
    path: &str,
    query: &str,
) -> Option<Result<(), Box<dyn Error + Send + Sync>>> {
    match path {
        "/" => {
            let cycle_id = query_value(query, "cycle").unwrap_or_else(|| config.cycle_id.clone());
            let html = render_console_html(&cycle_id, None);
            Some(write_response(stream, "200 OK", "text/html; charset=utf-8", &html))
        }
        "/reports" | "/reports/" => Some(handle_cycle_report_index_page(stream, config)),
        "/api/health" => Some(write_response(
            stream,
            "200 OK",
            "application/json; charset=utf-8",
            &render_health_json(config),
        )),
        "/api/reports" | "/api/reports/" => Some(handle_cycle_report_latest_api(stream, config)),
        "/api/reports/aliases.json" => Some(handle_cycle_report_aliases_api(stream, config)),
        _ if path.starts_with("/api/reports/by-alias/") => {
            Some(handle_cycle_report_alias_artifact_api(stream, &config.reports_dir, path))
        }
        _ if path.starts_with("/reports/by-alias/") => {
            Some(handle_cycle_report_alias_artifact_page(stream, &config.reports_dir, path))
        }
        _ if path.starts_with("/api/reports/")
            && path.trim_end_matches('/').ends_with("/ready") =>
        {
            Some(handle_cycle_report_ready_api(stream, config, path))
        }
        _ if path.starts_with("/api/reports/") => {
            Some(handle_cycle_report_artifact_api(stream, &config.reports_dir, path))
        }
        _ if path.starts_with("/reports/") || path.starts_with("/cycles/") => {
            Some(handle_cycle_report_artifact_page(stream, &config.reports_dir, path))
        }
        _ => None,
    }
}

fn handle_cycle_report_index_page(
    stream: &mut TcpStream,
    config: &DevConsoleConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let latest_json = match read_cycle_report_latest_json(&config.reports_dir) {
        Ok(latest_json) => Some(latest_json),
        Err(error) if is_text_file_size_limit_error(error.as_ref()) => {
            return write_response(
                stream,
                "413 Payload Too Large",
                "text/plain; charset=utf-8",
                &format!("report artifact too large: {error}"),
            );
        }
        Err(_) => None,
    };
    let aliases_json = match read_cycle_report_aliases_json(&config.reports_dir) {
        Ok(aliases_json) => Some(aliases_json),
        Err(error) if is_text_file_size_limit_error(error.as_ref()) => {
            return write_response(
                stream,
                "413 Payload Too Large",
                "text/plain; charset=utf-8",
                &format!("report alias index too large: {error}"),
            );
        }
        Err(_) => None,
    };
    write_response(
        stream,
        "200 OK",
        "text/html; charset=utf-8",
        &render_cycle_report_index_html_with_aliases(
            latest_json.as_deref(),
            aliases_json.as_deref(),
        ),
    )
}

fn handle_cycle_report_aliases_api(
    stream: &mut TcpStream,
    config: &DevConsoleConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match read_cycle_report_aliases_json(&config.reports_dir) {
        Ok(aliases_json) => match extract_cycle_alias_entries(&aliases_json) {
            Ok(_) => {
                write_response(stream, "200 OK", "application/json; charset=utf-8", &aliases_json)
            }
            Err(error) => write_response(
                stream,
                "422 Unprocessable Entity",
                "application/json; charset=utf-8",
                &json_error("report_alias_index_malformed", &error),
            ),
        },
        Err(error) if is_text_file_size_limit_error(error.as_ref()) => write_response(
            stream,
            "413 Payload Too Large",
            "application/json; charset=utf-8",
            &json_error("report_alias_index_too_large", &error.to_string()),
        ),
        Err(error) => write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("report_alias_index_not_found", &error.to_string()),
        ),
    }
}

fn handle_cycle_report_latest_api(
    stream: &mut TcpStream,
    config: &DevConsoleConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match read_cycle_report_latest_json(&config.reports_dir) {
        Ok(latest_json) => {
            write_response(stream, "200 OK", "application/json; charset=utf-8", &latest_json)
        }
        Err(error) if is_text_file_size_limit_error(error.as_ref()) => write_response(
            stream,
            "413 Payload Too Large",
            "application/json; charset=utf-8",
            &json_error("report_artifact_too_large", &error.to_string()),
        ),
        Err(error) => write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("latest_report_artifact_not_found", &error.to_string()),
        ),
    }
}

fn handle_console_input_submission(
    stream: &mut TcpStream,
    service: &DevelopmentTraceService,
    path: &str,
    body: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cycle_path =
        path.trim_start_matches("/api/cycles/").trim_end_matches("/input").trim_end_matches('/');
    let cycle_id = percent_decode(cycle_path);
    let form = body.split_once("\r\n\r\n").map_or("", |(_, body)| body);
    let input_type = form_value(form, "type").unwrap_or_else(|| "message".to_owned());
    let message = form_value(form, "message").unwrap_or_default();
    let invalidates_after_event_id = form_value(form, "invalidates_after_event_id");
    let stored = append_console_input_with_invalidation(
        service,
        &cycle_id,
        &input_type,
        &message,
        invalidates_after_event_id.as_deref(),
    )?;
    write_response(
        stream,
        "201 Created",
        "application/json; charset=utf-8",
        &public_trace_projection_json(&stored),
    )
}

fn handle_cycle_report_alias_artifact_page(
    stream: &mut TcpStream,
    reports_dir: &str,
    path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(cycle_alias) = cycle_alias_from_report_page_path(path) else {
        return write_response(
            stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            "report alias artifact not found",
        );
    };
    let cycle_id = match resolve_cycle_report_alias(reports_dir, &cycle_alias) {
        Ok(cycle_id) => cycle_id,
        Err(error) if is_text_file_size_limit_error(error.as_ref()) => {
            return write_response(
                stream,
                "413 Payload Too Large",
                "text/plain; charset=utf-8",
                &format!("report alias index too large: {error}"),
            );
        }
        Err(error) if is_cycle_report_alias_index_malformed_error(error.as_ref()) => {
            return write_response(
                stream,
                "422 Unprocessable Entity",
                "text/plain; charset=utf-8",
                &format!("report alias index malformed: {error}"),
            );
        }
        Err(error) => {
            return write_response(
                stream,
                "404 Not Found",
                "text/plain; charset=utf-8",
                &format!("report alias artifact not found: {error}"),
            );
        }
    };
    handle_cycle_report_artifact_page(stream, reports_dir, &format!("/reports/{cycle_id}/"))
}

fn handle_cycle_report_artifact_page(
    stream: &mut TcpStream,
    reports_dir: &str,
    path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some((cycle_id, file_name)) = cycle_report_page_file(path) else {
        return write_response(
            stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            "report artifact not found",
        );
    };
    match cycle_report_browser_artifact_file(reports_dir, &cycle_id, &file_name) {
        Ok((artifact_path, artifact_bytes)) => write_file_response(
            stream,
            "200 OK",
            artifact_content_type(&file_name),
            &artifact_path,
            artifact_bytes,
        ),
        Err(error) => write_response(
            stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            &format!("report artifact not found: {error}"),
        ),
    }
}

fn handle_cycle_report_ready_api(
    stream: &mut TcpStream,
    config: &DevConsoleConfig,
    path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(cycle_id) = cycle_id_from_report_ready_api_path(path) else {
        return write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("report_readiness_not_found", "unknown report readiness path"),
        );
    };
    if let Err(error) = validate_cycle_report_id(&cycle_id) {
        return write_response(
            stream,
            "400 Bad Request",
            "application/json; charset=utf-8",
            &json_error("unsafe_cycle_report_id", &error.to_string()),
        );
    }
    match cycle_report_browser_index_file(&config.reports_dir, &cycle_id) {
        Ok((_, index_html_bytes)) => {
            let diff_html_present =
                cycle_report_optional_diff_html_present(&config.reports_dir, &cycle_id);
            write_response(
                stream,
                "200 OK",
                "application/json; charset=utf-8",
                &render_cycle_report_ready_json(
                    config,
                    &cycle_id,
                    index_html_bytes,
                    diff_html_present,
                ),
            )
        }
        Err(error) => write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("report_artifact_not_ready", &error.to_string()),
        ),
    }
}

fn handle_cycle_report_alias_artifact_api(
    stream: &mut TcpStream,
    reports_dir: &str,
    path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some((cycle_alias, file_name)) = cycle_report_alias_api_file(path) else {
        return write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("report_alias_artifact_not_found", "unknown report alias artifact path"),
        );
    };
    if let Err(error) = validate_cycle_report_artifact_file_name(&file_name) {
        return write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("unsupported_report_artifact", &error.to_string()),
        );
    }
    let cycle_id = match resolve_cycle_report_alias(reports_dir, &cycle_alias) {
        Ok(cycle_id) => cycle_id,
        Err(error) if is_text_file_size_limit_error(error.as_ref()) => {
            return write_response(
                stream,
                "413 Payload Too Large",
                "application/json; charset=utf-8",
                &json_error("report_alias_index_too_large", &error.to_string()),
            );
        }
        Err(error) if is_cycle_report_alias_index_malformed_error(error.as_ref()) => {
            return write_response(
                stream,
                "422 Unprocessable Entity",
                "application/json; charset=utf-8",
                &json_error("report_alias_index_malformed", &error.to_string()),
            );
        }
        Err(error) => {
            return write_response(
                stream,
                "404 Not Found",
                "application/json; charset=utf-8",
                &json_error("report_alias_artifact_not_found", &error.to_string()),
            );
        }
    };
    if let Err(error) = validate_cycle_report_artifact_bundle(reports_dir, &cycle_id) {
        return write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("report_artifact_not_found", &error.to_string()),
        );
    }
    match read_cycle_report_artifact_file(reports_dir, &cycle_id, &file_name) {
        Ok(body) => write_response(stream, "200 OK", artifact_content_type(&file_name), &body),
        Err(error) if is_text_file_size_limit_error(error.as_ref()) => write_response(
            stream,
            "413 Payload Too Large",
            "application/json; charset=utf-8",
            &json_error("report_artifact_too_large", &error.to_string()),
        ),
        Err(error) => write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("report_artifact_not_found", &error.to_string()),
        ),
    }
}

fn handle_cycle_report_artifact_api(
    stream: &mut TcpStream,
    reports_dir: &str,
    path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some((cycle_id, file_name)) = cycle_report_api_file(path) else {
        return write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("report_artifact_not_found", "unknown report artifact path"),
        );
    };
    if let Err(error) = validate_cycle_report_id(&cycle_id) {
        return write_response(
            stream,
            "400 Bad Request",
            "application/json; charset=utf-8",
            &json_error("unsafe_cycle_report_id", &error.to_string()),
        );
    }
    if let Err(error) = validate_cycle_report_artifact_file_name(&file_name) {
        return write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("unsupported_report_artifact", &error.to_string()),
        );
    }
    if let Err(error) = validate_cycle_report_artifact_bundle(reports_dir, &cycle_id) {
        return write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("report_artifact_not_found", &error.to_string()),
        );
    }
    match read_cycle_report_artifact_file(reports_dir, &cycle_id, &file_name) {
        Ok(body) => write_response(stream, "200 OK", artifact_content_type(&file_name), &body),
        Err(error) if is_text_file_size_limit_error(error.as_ref()) => write_response(
            stream,
            "413 Payload Too Large",
            "application/json; charset=utf-8",
            &json_error("report_artifact_too_large", &error.to_string()),
        ),
        Err(error) => write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            &json_error("report_artifact_not_found", &error.to_string()),
        ),
    }
}

fn cycle_report_page_file(path: &str) -> Option<(String, String)> {
    let rest = path
        .strip_prefix("/reports/")
        .or_else(|| path.strip_prefix("/cycles/"))?
        .trim_end_matches('/');
    if rest.is_empty() {
        return None;
    }
    if let Some((cycle_id, file_name)) = rest.rsplit_once('/') {
        if cycle_id.is_empty() || file_name.is_empty() || cycle_id.contains('/') {
            return None;
        }
        let file_name = percent_decode(file_name);
        if !CYCLE_REPORT_BROWSER_ARTIFACT_FILES.contains(&file_name.as_str()) {
            return None;
        }
        return Some((percent_decode(cycle_id), file_name));
    }
    Some((percent_decode(rest), CYCLE_REPORT_INDEX_FILE.to_owned()))
}

fn cycle_alias_from_report_page_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/reports/by-alias/")?.trim_end_matches('/');
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some(percent_decode(rest))
}

fn cycle_report_api_file(path: &str) -> Option<(String, String)> {
    let rest = path.strip_prefix("/api/reports/")?.trim_end_matches('/');
    let (cycle_id, file_name) = rest.rsplit_once('/')?;
    if cycle_id.is_empty() || file_name.is_empty() || cycle_id.contains('/') {
        return None;
    }
    Some((percent_decode(cycle_id), percent_decode(file_name)))
}

fn cycle_report_alias_api_file(path: &str) -> Option<(String, String)> {
    let rest = path.strip_prefix("/api/reports/by-alias/")?.trim_end_matches('/');
    let (cycle_alias, file_name) = rest.rsplit_once('/')?;
    if cycle_alias.is_empty() || file_name.is_empty() || cycle_alias.contains('/') {
        return None;
    }
    Some((percent_decode(cycle_alias), percent_decode(file_name)))
}

fn cycle_id_from_report_ready_api_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/api/reports/")?.trim_end_matches('/');
    let cycle_id = rest.strip_suffix("/ready")?;
    if cycle_id.is_empty() || cycle_id.contains('/') {
        return None;
    }
    Some(percent_decode(cycle_id))
}

fn artifact_content_type(file_name: &str) -> &'static str {
    match file_name {
        "index.html" | "diff.html" => "text/html; charset=utf-8",
        "context.md" => "text/markdown; charset=utf-8",
        _ => "application/json; charset=utf-8",
    }
}

fn render_health_json(config: &DevConsoleConfig) -> String {
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "status", &json_string("ok"), true);
    push_json_field(&mut output, "service", &json_string("xavi-dev-console"), false);
    push_json_field(&mut output, "cycle_id", &json_string(&config.cycle_id), false);
    push_json_field(&mut output, "bind_addr", &json_string(&config.bind_addr), false);
    push_json_field(&mut output, "reports_dir", &json_string(&config.reports_dir), false);
    push_json_field(&mut output, "report_route", &json_string("/reports/{cycle_id}/"), false);
    output.push('}');
    output
}

fn render_cycle_report_ready_json(
    config: &DevConsoleConfig,
    cycle_id: &str,
    index_html_bytes: u64,
    diff_html_present: bool,
) -> String {
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "status", &json_string("ok"), true);
    push_json_field(&mut output, "service", &json_string("xavi-dev-console"), false);
    push_json_field(&mut output, "reports_dir", &json_string(&config.reports_dir), false);
    push_json_field(&mut output, "cycle_id", &json_string(cycle_id), false);
    push_json_field(&mut output, "index_html_bytes", &index_html_bytes.to_string(), false);
    push_json_field(&mut output, "diff_html_present", bool_json(diff_html_present), false);
    push_json_field(&mut output, "artifact_files_present", bool_json(true), false);
    output.push('}');
    output
}

fn json_error(code: &str, message: &str) -> String {
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "error", &json_string(code), true);
    push_json_field(&mut output, "message", &json_string(message), false);
    output.push('}');
    output
}

fn cycle_id_from_suffixed_path(path: &str, suffix: &str) -> Option<String> {
    let cycle_path =
        path.trim_start_matches("/api/cycles/").strip_suffix(suffix)?.trim_end_matches('/');
    Some(percent_decode(cycle_path))
}

fn stream_trace_events(
    stream: &mut TcpStream,
    service: &DevelopmentTraceService,
    cycle_id: &str,
    report_limit: usize,
    initial_last_seen_id: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    write_sse_headers(stream)?;
    stream.write_all(b": connected to xavi-dev-console trace/readback mode\nretry: 1000\n\n")?;
    let mut last_seen_id = initial_last_seen_id;
    let mut idle_polls = 0_usize;

    for poll_index in 0..SSE_MAX_POLLS {
        let entries = cycle_entries_since(service, cycle_id, last_seen_id, report_limit)?;
        if entries.is_empty() {
            idle_polls += 1;
        } else {
            idle_polls = 0;
        }
        for entry in entries {
            let frame = sse_trace_frame(&entry);
            if stream.write_all(frame.as_bytes()).is_err() {
                return Ok(());
            }
            last_seen_id = entry.id;
        }
        if stream.flush().is_err() {
            return Ok(());
        }
        if idle_polls >= SSE_MAX_IDLE_POLLS {
            break;
        }
        if poll_index + 1 < SSE_MAX_POLLS {
            thread::sleep(SSE_POLL_INTERVAL);
        }
    }

    let _ = stream.write_all(b": xavi-dev-console SSE stream closed by safety guard\n\n");
    let _ = stream.flush();
    Ok(())
}

fn sse_poll_entry_limit(report_limit: usize) -> usize {
    report_limit.clamp(1, SSE_MAX_POLL_ENTRIES)
}

fn sse_window_entry_limit(report_limit: usize) -> usize {
    let poll_limit = sse_poll_entry_limit(report_limit);
    poll_limit.saturating_mul(SSE_WINDOW_MULTIPLIER).min(SSE_MAX_WINDOW_ENTRIES)
}

fn cycle_entries_since(
    service: &DevelopmentTraceService,
    cycle_id: &str,
    last_seen_id: i64,
    report_limit: usize,
) -> Result<Vec<DevelopmentTraceEntry>, Box<dyn Error + Send + Sync>> {
    let poll_limit = sse_poll_entry_limit(report_limit);
    let window_limit = sse_window_entry_limit(report_limit);
    let window_entries = service.list_latest_entries(&DevelopmentTraceFilter {
        cycle_id: Some(cycle_id.to_owned()),
        kind: None,
        limit: Some(window_limit),
    })?;
    let mut entries =
        window_entries.into_iter().filter(|entry| entry.id > last_seen_id).collect::<Vec<_>>();

    if entries.len() > poll_limit {
        entries = entries.split_off(entries.len() - poll_limit);
    }

    Ok(entries)
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, HttpRequestReadError> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if buffer.len() > MAX_REQUEST_BYTES {
            return Err(HttpRequestReadError::PayloadTooLarge("request too large"));
        }
    }

    let header_end = buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map_or(buffer.len(), |index| index + 4);
    let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let content_length = headers
        .lines()
        .find_map(|line| line.split_once(':'))
        .and_then(|(name, value)| {
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    enforce_request_size_limits(&headers, header_end, content_length)?;
    let already_read_body = buffer.len().saturating_sub(header_end);
    if content_length > already_read_body {
        let mut rest = vec![0_u8; content_length - already_read_body];
        stream.read_exact(&mut rest)?;
        buffer.extend_from_slice(&rest);
    }
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

fn enforce_request_size_limits(
    headers: &str,
    header_len: usize,
    content_length: usize,
) -> Result<(), HttpRequestReadError> {
    if header_len.checked_add(content_length).is_none_or(|length| length > MAX_REQUEST_BYTES) {
        return Err(HttpRequestReadError::PayloadTooLarge("request too large"));
    }
    if request_method(headers) == Some("POST") && content_length > MAX_POST_BODY_BYTES {
        return Err(HttpRequestReadError::PayloadTooLarge("post body too large"));
    }
    Ok(())
}

fn request_method(headers: &str) -> Option<&str> {
    headers.lines().next()?.split_whitespace().next()
}

fn last_event_id_cursor(request: &str) -> i64 {
    header_value(request, "Last-Event-ID")
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(0)
}

fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    request
        .lines()
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once(':'))
        .find_map(|(candidate, value)| candidate.eq_ignore_ascii_case(name).then(|| value.trim()))
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn write_file_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    path: &Path,
    content_length: u64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut file = std::fs::File::open(path)?;
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(headers.as_bytes())?;
    std::io::copy(&mut file, stream)?;
    Ok(())
}

fn write_sse_headers(stream: &mut TcpStream) -> Result<(), Box<dyn Error + Send + Sync>> {
    stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nX-Accel-Buffering: no\r\n\r\n",
    )?;
    Ok(())
}

fn split_path_query(raw_path: &str) -> (String, &str) {
    raw_path
        .split_once('?')
        .map_or_else(|| (raw_path.to_owned(), ""), |(path, query)| (path.to_owned(), query))
}

fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (candidate, value) = pair.split_once('=')?;
        (candidate == key).then(|| percent_decode(value))
    })
}

fn form_value(form: &str, key: &str) -> Option<String> {
    form.split('&').find_map(|pair| {
        let (candidate, value) = pair.split_once('=')?;
        (percent_decode(candidate) == key).then(|| percent_decode(value))
    })
}

fn push_entries_array(output: &mut String, entries: &[DevelopmentTraceEntry]) {
    output.push('[');
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&public_trace_projection_json(entry));
    }
    output.push(']');
}

fn push_role_lanes_array(output: &mut String, lanes: &[RoleLaneSummary]) {
    output.push('[');
    for (index, lane) in lanes.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push('{');
        push_json_field(output, "role", &json_string(lane.role), true);
        push_json_field(output, "status", &json_string(lane.status), false);
        push_json_field(output, "entry_count", &lane.entry_count.to_string(), false);
        push_json_field(
            output,
            "latest_entry_id",
            &lane.latest_entry_id.map_or_else(|| "null".to_owned(), |id| id.to_string()),
            false,
        );
        push_json_field(output, "latest_kind", &json_optional_string(lane.latest_kind), false);
        push_json_field(
            output,
            "latest_summary",
            &json_optional_string(lane.latest_summary.as_deref()),
            false,
        );
        push_json_field(
            output,
            "latest_role_source",
            &json_optional_string(lane.latest_role_source),
            false,
        );
        output.push('}');
    }
    output.push(']');
}

fn public_trace_projection_json(entry: &DevelopmentTraceEntry) -> String {
    let role_resolution = role_resolution_for_entry(entry);
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "id", &entry.id.to_string(), true);
    push_json_field(&mut output, "event_id", &json_string(&entry.event_id), false);
    push_json_field(&mut output, "cycle_id", &json_string(&entry.cycle_id), false);
    push_json_field(&mut output, "kind", &json_string(entry.kind.as_str()), false);
    push_json_field(&mut output, "role", &json_string(role_resolution.role), false);
    push_json_field(&mut output, "role_source", &json_string(role_resolution.source), false);
    push_json_field(
        &mut output,
        "role_name",
        &json_optional_string(entry.role_name.as_deref()),
        false,
    );
    push_json_field(&mut output, "status", &json_string(status_for_entry(entry)), false);
    push_json_field(&mut output, "summary", &json_string(&public_text(&entry.summary)), false);
    push_json_field(
        &mut output,
        "public_excerpt",
        &json_string(&public_trace_excerpt(entry)),
        false,
    );
    push_json_field(
        &mut output,
        "redaction_status",
        &json_string(redaction_status_for_entry(entry)),
        false,
    );
    push_json_field(&mut output, "is_console_input", bool_json(is_console_input(entry)), false);
    push_json_field(
        &mut output,
        "input_type",
        &json_optional_string(console_input_type(entry).as_deref()),
        false,
    );
    push_json_field(
        &mut output,
        "invalidates_after_event_id",
        &json_optional_string(console_invalidates_after_event_id(entry).as_deref()),
        false,
    );
    push_json_field(&mut output, "created_at", &json_string(&entry.created_at), false);
    output.push('}');
    output
}

fn trace_ui_event_json(entry: &DevelopmentTraceEntry) -> String {
    let role_resolution = role_resolution_for_entry(entry);
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "event_id", &json_string(&entry.event_id), true);
    push_json_field(&mut output, "cycle_id", &json_string(&entry.cycle_id), false);
    push_json_field(&mut output, "sequence", &entry.id.to_string(), false);
    push_json_field(&mut output, "occurred_at", &json_string(&entry.created_at), false);
    push_json_field(&mut output, "source_type", &json_string(source_type_for_entry(entry)), false);
    push_json_field(&mut output, "role", &json_string(role_resolution.role), false);
    push_json_field(&mut output, "role_source", &json_string(role_resolution.source), false);
    push_json_field(&mut output, "kind", &json_string(entry.kind.as_str()), false);
    push_json_field(&mut output, "status", &json_string(status_for_entry(entry)), false);
    push_json_field(&mut output, "summary", &json_string(&public_text(&entry.summary)), false);
    push_json_field(&mut output, "visibility", &json_string("public_ui"), false);
    push_json_field(
        &mut output,
        "redaction_status",
        &json_string(redaction_status_for_entry(entry)),
        false,
    );
    push_json_field(
        &mut output,
        "public_excerpt",
        &json_string(&public_trace_excerpt(entry)),
        false,
    );
    push_json_field(&mut output, "entry", &public_trace_projection_json(entry), false);
    output.push('}');
    output
}

fn sse_trace_frame(entry: &DevelopmentTraceEntry) -> String {
    format!("id: {}\nevent: trace\ndata: {}\n\n", entry.id, trace_ui_event_json(entry))
}

fn push_json_field(output: &mut String, key: &str, value: &str, first: bool) {
    if !first {
        output.push(',');
    }
    let _ = write!(output, "{}:{value}", json_string(key));
}

fn json_optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), json_string)
}

fn bool_json(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            control if control.is_control() => {
                let _ = write!(escaped, "\\u{:04x}", u32::from(control));
            }
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

fn public_trace_excerpt(entry: &DevelopmentTraceEntry) -> String {
    let source = if entry.body.trim().is_empty() { &entry.summary } else { &entry.body };
    public_excerpt(source, redaction_status_for_entry(entry) == "redacted")
}

fn public_excerpt(value: &str, force_redaction: bool) -> String {
    if force_redaction || should_redact_public_text(value) {
        return PUBLIC_REDACTION_PLACEHOLDER.to_owned();
    }

    let trimmed = value.trim();
    if trimmed.chars().count() <= PUBLIC_EXCERPT_CHAR_LIMIT {
        trimmed.to_owned()
    } else {
        let mut clipped =
            trimmed.chars().take(PUBLIC_EXCERPT_CHAR_LIMIT.saturating_sub(3)).collect::<String>();
        clipped.push_str("...");
        clipped
    }
}

fn public_text(value: &str) -> String {
    if should_redact_public_text(value) {
        PUBLIC_REDACTION_PLACEHOLDER.to_owned()
    } else {
        value.to_owned()
    }
}

fn redaction_status_for_entry(entry: &DevelopmentTraceEntry) -> &'static str {
    if should_redact_public_text(&entry.summary)
        || should_redact_public_text(&entry.body)
        || should_redact_public_text(&entry.metadata_json)
    {
        "redacted"
    } else {
        "none"
    }
}

fn should_redact_public_text(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "password",
        "passwd",
        "token",
        "secret",
        "api_key",
        "apikey",
        "authorization",
        "bearer ",
        "system prompt",
        "developer prompt",
        "chain-of-thought",
        "raw reasoning",
        "scratchpad",
        concat!("터미널에 붙여넣을 ", "한국어 프롬프트 초안"),
        concat!("이 과정에서 ", "잘못됐어"),
        concat!("내가 ", "의도한 건"),
        concat!("이 부분 ", "수정 부탁해"),
        concat!("폐기", "해야해"),
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn console_input_type(entry: &DevelopmentTraceEntry) -> Option<String> {
    if !is_console_input(entry) {
        return None;
    }
    json_string_field(&entry.metadata_json, "input_type")
        .map(|value| normalized_input_type(&value).to_owned())
}

fn console_invalidates_after_event_id(entry: &DevelopmentTraceEntry) -> Option<String> {
    if !is_console_input(entry) {
        return None;
    }
    json_string_field(&entry.metadata_json, "invalidates_after_event_id")
        .map(|value| public_excerpt(&value, false))
}

fn percent_decode(value: &str) -> String {
    let mut bytes = Vec::with_capacity(value.len());
    let mut input = value.as_bytes().iter().copied();
    while let Some(byte) = input.next() {
        match byte {
            b'+' => bytes.push(b' '),
            b'%' => {
                let high = input.next();
                let low = input.next();
                if let (Some(high), Some(low)) = (high, low) {
                    if let Some(decoded) = decode_hex_pair(high, low) {
                        bytes.push(decoded);
                    }
                }
            }
            other => bytes.push(other),
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn url_component_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(*byte));
            }
            other => {
                let _ = write!(encoded, "%{other:02X}");
            }
        }
    }
    encoded
}

fn decode_hex_pair(high: u8, low: u8) -> Option<u8> {
    Some(hex_value(high)? * 16 + hex_value(low)?)
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn normalized_input_type(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "approval" | "approve" | "yes" => "approval",
        "revision" | "revise" | "change" => "revision",
        "question" | "ask" => "question",
        _ => "message",
    }
}

fn input_type_public_label(value: &str) -> &'static str {
    match value {
        "approval" => "승인",
        "revision" => "수정 요청",
        "question" => "질문",
        _ => "메시지",
    }
}

fn one_line_summary(message: &str) -> String {
    let mut summary = message.lines().next().unwrap_or("").trim().to_owned();
    if summary.chars().count() > 120 {
        summary = summary.chars().take(117).collect::<String>();
        summary.push_str("...");
    }
    summary
}

fn is_console_input(entry: &DevelopmentTraceEntry) -> bool {
    entry.kind == DevelopmentTraceKind::UserQuery
        && entry.metadata_json.contains("\"source\":\"xavi-dev-console\"")
}

fn generated_event_id(prefix: &str) -> String {
    let nanos = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    format!("{prefix}-{nanos}")
}

fn trace_sequence_no(event_id: &str) -> i64 {
    event_id
        .rsplit('-')
        .next()
        .and_then(|value| value.parse::<u128>().ok())
        .and_then(|value| i64::try_from((value % (i64::MAX as u128 - 1)) + 1).ok())
        .unwrap_or(1)
}

fn epoch_timestamp() -> String {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => format!("unix:{}", duration.as_secs()),
        Err(_) => "unix:0".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xavi_application::ports::development_trace_store::{
        DevelopmentTraceStore, DevelopmentTraceStoreResult,
    };
    use xavi_infrastructure::development_trace::sqlite_development_trace_store::SqliteDevelopmentTraceStore;

    #[test]
    fn report_json_groups_cycle_entries() {
        let service = test_service();
        append_entry(&service, DevelopmentTraceKind::AgentDispatch, "planning dispatch", "{}");
        append_entry(&service, DevelopmentTraceKind::AgentReturn, "planning returned", "{}");

        let report = build_report(&service, "cycle-dev-console-test", 20).unwrap();
        let json = render_report_json(&report);

        assert_eq!(report.agent_dispatch_count, 1);
        assert_eq!(report.agent_return_count, 1);
        assert!(json.contains("\"cycle_id\":\"cycle-dev-console-test\""));
        assert!(json.contains("\"agent_dispatch\":1"));
        assert!(json.contains("\"agent_return\":1"));
    }

    #[test]
    fn report_uses_latest_entries_and_names_displayed_count() {
        let service = test_service();
        let first = append_role_entry(
            &service,
            DevelopmentTraceKind::AgentDispatch,
            "planning",
            "old planning dispatch",
            "{}",
        );
        let second = append_role_entry(
            &service,
            DevelopmentTraceKind::AgentDispatch,
            "codegen",
            "new codegen dispatch",
            "{}",
        );
        let third = append_role_entry(
            &service,
            DevelopmentTraceKind::AgentReturn,
            "codegen",
            "new codegen return",
            "{}",
        );

        let report = build_report(&service, "cycle-dev-console-test", 2).unwrap();
        let json = render_report_json(&report);

        assert_eq!(report.total_entry_count, 3);
        assert_eq!(report.displayed_entry_count, 2);
        assert_eq!(
            report.entries.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            vec![second.id, third.id]
        );
        assert!(!report.entries.iter().any(|entry| entry.id == first.id));
        assert!(json.contains("\"entry_count\":2"));
        assert!(json.contains("\"displayed_entry_count\":2"));
        assert!(json.contains("\"total_entry_count\":3"));
        assert!(json.contains("new codegen return"));
        assert!(!json.contains("old planning dispatch"));
    }

    #[test]
    fn build_report_uses_bounded_latest_source_window_for_report_and_audit() {
        let service = DevelopmentTraceService::new(LatestWindowOnlyTraceStore::new(
            (1..=6).map(|id| report_store_entry(id, &format!("source entry {id}"))).collect(),
        ));

        let report = build_report(&service, "cycle-dev-console-test", 1).unwrap();
        let json = render_report_json(&report);

        assert_eq!(report_source_window_limit(1), 4);
        assert_eq!(report.total_entry_count, 4);
        assert_eq!(report.displayed_entry_count, 1);
        assert_eq!(report.trace_integrity.checked_entry_count, 4);
        assert_eq!(report.entries.iter().map(|entry| entry.id).collect::<Vec<_>>(), vec![6]);
        assert!(json.contains("source entry 6"));
        assert!(!json.contains("source entry 1"));
        assert!(!json.contains("source entry 2"));
    }

    #[test]
    fn console_input_is_stored_as_pending_user_query() {
        let service = test_service();
        let stored = append_console_input(
            &service,
            "cycle-dev-console-test",
            "approval",
            "Proceed with the implementation.",
        )
        .unwrap();
        let report = build_report(&service, "cycle-dev-console-test", 20).unwrap();

        assert_eq!(stored.kind, DevelopmentTraceKind::UserQuery);
        assert!(stored.metadata_json.contains("\"source\":\"xavi-dev-console\""));
        assert!(stored.metadata_json.contains("\"trace_contract_version\":2"));
        assert!(stored.metadata_json.contains("\"user_request_verbatim\""));
        assert!(stored.metadata_json.contains("\"text\":\"Proceed with the implementation.\""));
        assert!(stored.metadata_json.contains("\"source_type\":\"dev_console_input\""));
        assert!(
            stored
                .metadata_json
                .contains("\"source_ref\":\"development_trace://events/dev_console_input-")
        );
        assert!(stored.metadata_json.contains("\"role\":\"user\""));
        assert!(stored.metadata_json.contains("\"agent_id\":null"));
        assert!(stored.metadata_json.contains("\"timestamp\":\"unix:"));
        assert!(stored.metadata_json.contains("\"order\":"));
        assert!(stored.metadata_json.contains(&format!(
            "\"hash_sha256\":\"{}\"",
            trace_text_sha256_hex("Proceed with the implementation.")
        )));
        assert!(!stored.metadata_json.contains("\"trace_contract_version\":1"));
        assert_eq!(report.pending_inputs.len(), 1);
        assert!(render_report_json(&report).contains("Proceed with the implementation."));
    }

    #[test]
    fn console_revision_input_records_invalidation_reference() {
        let service = test_service();
        let stored = append_console_input_with_invalidation(
            &service,
            "cycle-dev-console-test",
            "revision",
            "선택한 로그 기준으로 수정 요청을 기록한다.",
            Some("event-123"),
        )
        .unwrap();

        assert!(stored.summary.contains("개발 콘솔 수정 요청"));
        assert!(stored.metadata_json.contains("\"input_type\":\"revision\""));
        assert!(stored.metadata_json.contains("\"invalidates_after_event_id\":\"event-123\""));
        let json =
            render_report_json(&build_report(&service, "cycle-dev-console-test", 20).unwrap());
        assert!(json.contains("\"input_type\":\"revision\""));
        assert!(json.contains("\"invalidates_after_event_id\":\"event-123\""));
        assert!(!json.contains("\"metadata_json\""));
    }

    #[test]
    fn html_shell_embeds_initial_report_when_provided() {
        let service = test_service();
        append_entry(&service, DevelopmentTraceKind::TestSummary, "tests passed", "{}");
        let report = build_report(&service, "cycle-dev-console-test", 20).unwrap();
        let json = render_report_json(&report);
        let html = render_console_html("cycle-dev-console-test", Some(&json));

        assert!(html.contains("<html lang=\"ko\">"));
        assert!(html.contains("Xavi 개발 콘솔"));
        assert!(html.contains("window.__INITIAL_REPORT__ = {"));
        assert!(html.contains("cycle-dev-console-test"));
    }

    #[test]
    fn html_shell_escapes_initial_report_script_breakout_text() {
        let service = test_service();
        service
            .append_entry(&NewDevelopmentTraceEntry {
                event_id: "event-script-breakout".to_owned(),
                cycle_id: "cycle-dev-console-test".to_owned(),
                user_turn_id: None,
                kind: DevelopmentTraceKind::AgentReturn,
                role_name: Some("codegen".to_owned()),
                summary: "script breakout check".to_owned(),
                body: "공개 로그 </script><script>alert(1)</script> & 앞\u{2028}\u{2029}뒤"
                    .to_owned(),
                metadata_json: test_metadata_json(
                    DevelopmentTraceKind::AgentReturn,
                    "codegen",
                    "{}",
                ),
                created_at: epoch_timestamp(),
            })
            .expect("trace entry should append");
        let report = build_report(&service, "cycle-dev-console-test", 20).unwrap();
        let json = render_report_json(&report);
        let html = render_console_html("cycle-dev-console-test", Some(&json));

        assert!(
            html.contains("\\u003c/script\\u003e\\u003cscript\\u003ealert(1)\\u003c/script\\u003e")
        );
        assert!(html.contains("\\u0026"));
        assert!(html.contains("앞\\u2028\\u2029뒤"));
        assert!(!html.contains("</script><script>alert(1)</script>"));
    }

    #[test]
    fn report_json_includes_role_lanes() {
        let service = test_service();
        append_role_entry(
            &service,
            DevelopmentTraceKind::AgentDispatch,
            "codegen",
            "codegen dispatch",
            "{}",
        );
        append_role_entry(
            &service,
            DevelopmentTraceKind::AgentReturn,
            "review",
            "review returned",
            "{}",
        );
        append_role_entry(
            &service,
            DevelopmentTraceKind::AgentReturn,
            "cycle-report",
            "cycle-report returned",
            "{}",
        );
        let report = build_report(&service, "cycle-dev-console-test", 20).unwrap();
        let json = render_report_json(&report);

        assert!(json.contains("\"role_lanes\""));
        assert!(json.contains("\"role\":\"codegen\""));
        assert!(json.contains("\"status\":\"running\""));
        assert!(json.contains("\"role\":\"review\""));
        assert!(json.contains("\"status\":\"returned\""));
        assert!(json.contains("\"role\":\"cycle-report\""));
        assert!(json.contains("cycle-report returned"));
    }

    #[test]
    fn role_lanes_infer_legacy_agent_returns_without_raw_trace_mutation() {
        let planning = legacy_roleless_entry(
            1,
            "planning-copy-prompt-console-return-001",
            "planning returned without role_name",
        );
        let codegen = legacy_roleless_entry(
            2,
            "legacy-codegen-return",
            "codegen: returned without role_name",
        );
        let review = legacy_roleless_entry(
            3,
            "review-copy-prompt-console-return-001",
            "review returned without role_name",
        );
        let cycle_report = legacy_roleless_entry(
            4,
            "cycle-report-copy-prompt-console-return-001",
            "cycle-report returned without role_name",
        );

        let entries = vec![planning.clone(), codegen.clone(), review.clone(), cycle_report.clone()];
        let report = DevelopmentCycleReport::from_entries(
            "cycle-dev-console-test",
            (entries, 4),
            audit_development_trace_cycle(
                "cycle-dev-console-test",
                &[planning.clone(), codegen.clone(), review.clone(), cycle_report.clone()],
            ),
        );
        let lanes = role_lane_summaries(&report.entries);
        let lane = |role: &str| {
            lanes.iter().find(|lane| lane.role == role).expect("role lane should exist")
        };

        assert_eq!(role_lane_for_entry(&planning), "planning");
        assert_eq!(role_lane_for_entry(&codegen), "codegen");
        assert_eq!(role_lane_for_entry(&review), "review");
        assert_eq!(role_lane_for_entry(&cycle_report), "cycle-report");
        assert_eq!(lane("planning").entry_count, 1);
        assert_eq!(lane("codegen").entry_count, 1);
        assert_eq!(lane("review").entry_count, 1);
        assert_eq!(lane("cycle-report").entry_count, 1);
        assert_eq!(lane("planning").latest_role_source, Some("inferred_from_event_id"));
        assert_eq!(lane("codegen").latest_role_source, Some("inferred_from_summary"));
        assert_eq!(lane("review").latest_role_source, Some("inferred_from_event_id"));
        assert_eq!(lane("cycle-report").latest_role_source, Some("inferred_from_event_id"));
        assert_eq!(planning.role_name, None);
        assert_eq!(codegen.role_name, None);
        assert_eq!(review.role_name, None);
        assert_eq!(cycle_report.role_name, None);

        let json = render_report_json(&report);
        assert!(json.contains("\"role\":\"planning\""));
        assert!(json.contains("\"role\":\"codegen\""));
        assert!(json.contains("\"role\":\"review\""));
        assert!(json.contains("\"role\":\"cycle-report\""));
        assert!(json.contains("\"role_source\":\"inferred_from_event_id\""));
        assert!(json.contains("\"role_source\":\"inferred_from_summary\""));
        assert!(json.contains("\"role_name\":null"));
        assert!(!json.contains("\"metadata_json\""));
        assert!(!json.contains("\"body\""));
        assert!(!json.contains("\"user_turn_id\""));
    }

    #[test]
    fn html_shell_names_visible_trace_sections_without_hidden_reasoning_claims() {
        let html = render_console_html("cycle-dev-console-test", None);

        assert!(html.contains("사이클 개요"));
        assert!(html.contains("작업 사이클 지도"));
        assert!(html.contains("오케스트라 조율"));
        assert!(html.contains("역할별 작업 보드"));
        assert!(html.contains("검수와 재시도 흐름"));
        assert!(html.contains("검증 결과"));
        assert!(html.contains("Trace 무결성"));
        assert!(html.contains("현재 제한 / 다음 단계"));
        assert!(html.contains("선택 로그 복사용 정보"));
        assert!(html.contains("기존 콘솔 입력 기록"));
        assert!(html.contains("원본 이벤트 흐름"));
        assert!(html.contains("판단 로그"));
        assert!(html.contains("역할 지시"));
        assert!(html.contains("검수 메모"));
        assert!(html.contains("상태 전이 이유"));
        assert!(html.contains("공개 반환 요약"));
        assert!(!html.contains("chain-of-thought"));
        assert!(!html.contains("raw reasoning"));
    }

    #[test]
    fn html_shell_includes_click_to_terminal_prompt_copy_controls() {
        let html = render_console_html("cycle-dev-console-test", None);

        assert!(html.contains("id=\"message\""));
        assert!(!html.contains("id=\"invalidates-after-event-id\""));
        assert!(!html.contains("name=\"invalidates_after_event_id\""));
        assert!(html.contains("trace-select"));
        assert!(html.contains("data-entry-id"));
        assert!(html.contains("promptForEntry"));
        assert!(html.contains("선택 로그 복사용 정보"));
        assert!(html.contains("정보 복사"));
        assert!(html.contains("전체 선택"));
        assert!(html.contains("navigator.clipboard.writeText"));
        assert!(html.contains("공개 본문 excerpt:"));
        assert!(html.contains("역할 출처:"));
        assert!(html.contains("추정: event_id prefix, 원본 role_name 없음"));
        assert!(html.contains("역할 기록 없음"));
        assert!(html.contains("function connectEvents()"));
        assert!(html.contains("id=\"live-button\""));
        assert!(html.contains("실시간 시작"));
        assert!(html.contains("수동 새로고침 모드"));
        assert!(!html.contains("connectEvents();"));
        assert!(html.contains("test_summary 없음 / test 역할 trace 없음"));
        assert!(html.contains("Trace 무결성 감사 항목"));
        assert!(html.contains("요청:"));
        assert!(!html.contains("폐기 기준:"));
        assert!(html.contains("invalidates_after_event_id(참고용):"));
        assert!(!html.contains("수정 요청 보내기"));
        assert!(!html.contains("id=\"input-form\""));
        assert!(!html.contains("type=\"submit\""));
        assert!(!html.contains("/api/cycles/${encodeURIComponent(cycleId)}/input"));
    }

    #[test]
    fn cycle_report_artifact_html_contains_required_korean_sections_and_failure_details() {
        let artifact = sample_cycle_report_artifact();
        let html = render_cycle_report_artifact_html(&artifact);

        for title in [
            "사이클 상태",
            "사용자 요청 원문",
            "1회 작업 사이클 과정",
            "역할 지시 원문",
            "역할 반환 원문",
            "실패 분석",
            "테스트 명령/결과 원문 evidence",
            "전체 변경 기록",
            "Diff 전용 artifact 색인",
            "파생 요약",
            "보고서 신뢰성 검증",
            "원본 raw/audit/context",
        ] {
            assert!(html.contains(title), "missing report section: {title}");
        }
        for phrase in [
            "실패",
            "feature-001",
            "별칭 라우트 테스트",
            "cycle_category_key",
            "사용자가 요청한 기능",
            "사용자 원문 요청 본문",
            "증거 메타데이터",
            "trace://cycle-report-route-test/user/1",
            "9d7c0293b90b4267d298eefa833d44beaf414f9a68abb6f4d2d0773573861214",
            "원문 증거 불완전",
            "작업 결과가 실패했다는 뜻이 아니라",
            "audit.status=fail",
            "Audit 뜻: 보고서 신뢰성 검증",
            "orchestra가 cycle-report를 생성하라고 지시함",
            "codegen 원문 지시",
            "prompt_derived_summary_ko",
            "derived summary, not verbatim",
            "legacy role return",
            "planning returned",
            "codegen returned",
            "review returned",
            "test returned",
            "검증 실패: cargo test",
            "cargo test -p xavi-dev-console failed",
            "apps/xavi-dev-console/src/lib.rs",
            "open-cycle 명령이 같은 서버를 재사용하도록 변경",
            "raw-event-1",
            "audit failure raw",
            "context markdown raw",
            "보고서 신뢰성 진단 기술 상세",
            "audit-diagnostic-details",
        ] {
            assert!(html.contains(phrase), "missing report phrase: {phrase}");
        }
        assert!(html.contains("<details"));
        assert!(!html.contains("EventSource"));
        assert!(!html.contains("setInterval"));
        assert!(!html.contains("connectEvents"));
    }

    #[test]
    fn cycle_report_artifact_html_renders_user_request_as_full_width_reading_block() {
        let artifact = sample_cycle_report_artifact();
        let html = render_cycle_report_artifact_html(&artifact);

        let evidence_start = html
            .find("<section class=\"section\" aria-labelledby=\"evidence-title\">")
            .expect("user request evidence section should exist");
        let workflow_start = html
            .find("<section class=\"section\" aria-labelledby=\"workflow-title\">")
            .expect("workflow section should follow user request evidence");
        let evidence_section = &html[evidence_start..workflow_start];

        for phrase in [
            "사용자 요청 원문",
            "전체 폭으로 보여줍니다",
            "user-request-card",
            "<pre class=\"evidence-text\">사용자 원문 요청 본문</pre>",
            "<details class=\"evidence-details evidence-metadata\">",
            "<th scope=\"row\">source_ref</th>",
            "trace://cycle-report-route-test/user/1",
        ] {
            assert!(
                evidence_section.contains(phrase),
                "missing full-width evidence phrase: {phrase}"
            );
        }
        assert!(
            !evidence_section.contains("grid-two"),
            "user request evidence section must not use two-column cards"
        );
        assert!(
            !evidence_section.contains("source_ref: trace://cycle-report-route-test/user/1"),
            "source_ref metadata should not be rendered as inline body text"
        );
        assert!(html.contains("white-space: pre-wrap;"));
        assert!(html.contains("overflow-wrap: anywhere;"));
        assert!(html.contains("width: calc(100% - 32px);"));
        assert!(!html.contains("width: min(1380px"));
    }

    #[test]
    fn cycle_report_artifact_html_localizes_dispatch_return_and_adds_text_modal() {
        let long_flow_text = long_flow_card_fixture_text();
        let artifact = sample_cycle_report_artifact_with_workflow_text(&long_flow_text);
        let html = render_cycle_report_artifact_html(&artifact);
        let diff_html = render_cycle_report_diff_html(&artifact);
        let escaped_long_flow_text = html_escape(&long_flow_text);

        for phrase in ["1회 작업 사이클 과정", "역할 지시 원문", "역할 반환 원문"]
        {
            assert!(html.contains(phrase), "missing modal/localized phrase: {phrase}");
        }
        assert_text_modal_scaffold_and_flow_selector(&html);
        assert!(
            long_flow_text.len() > 280 && long_flow_text.lines().count() > 7,
            "fixture should be long enough to satisfy the clamp heuristic"
        );
        assert!(
            html.contains(&format!("<p>{escaped_long_flow_text}</p>")),
            "rendered workflow map should preserve the long flow-card paragraph in the source DOM"
        );
        assert!(
            html.contains("const looksLong = text.length > 280 || text.split('\\n').length > 7;"),
            "modal enhancer should clamp long text by length or line count"
        );
        for old_label in [
            ">dispatch evidence</a>",
            ">return evidence</a>",
            "역할별 dispatch evidence",
            "역할별 return evidence",
            "Role Dispatch / Return",
        ] {
            assert!(!html.contains(old_label), "old label should be removed: {old_label}");
        }
        assert!(
            html.contains("body.textContent = target.textContent || '';"),
            "modal should reuse the original DOM text instead of replacing the source node"
        );
        assert!(
            !diff_html.contains("report-text-modal"),
            "diff.html should remain a diff-only artifact without the text modal UI"
        );
    }

    #[test]
    fn cycle_report_index_template_localizes_dispatch_return_and_contains_text_modal() {
        let template_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/agent/cycle-report/templates/index.html");
        let template = std::fs::read_to_string(template_path).expect("template should be readable");

        for phrase in ["1회 작업 사이클 과정", "역할 지시 원문", "역할 반환 원문"]
        {
            assert!(template.contains(phrase), "missing template phrase: {phrase}");
        }
        assert_text_modal_scaffold_and_flow_selector(&template);
        assert!(!template.contains(">dispatch evidence</a>"));
        assert!(!template.contains(">return evidence</a>"));
        assert!(!template.contains("Role Dispatch / Return"));
    }

    #[test]
    fn cycle_report_artifact_html_keeps_audit_diagnostics_collapsed_without_failure_style() {
        let artifact = sample_cycle_report_artifact();
        let html = render_cycle_report_artifact_html(&artifact);

        let audit_start = html
            .find("<section class=\"section\" aria-labelledby=\"evidence-audit-title\">")
            .expect("evidence audit section should exist");
        let raw_start = html
            .find("<section class=\"section\" aria-labelledby=\"raw-title\">")
            .expect("raw section should follow evidence audit section");
        let audit_section = &html[audit_start..raw_start];

        for phrase in [
            "누락된 원문 evidence 목록",
            "무효 원문 evidence 목록",
            "원문이 아니라 파생 표시인 필드",
            "필수 입력 누락",
            "검증 경고",
            "보고서 신뢰성 진단 기술 상세",
            "아래 원문은 작업 실패 판정이 아니라 trace audit이 보고서 evidence를 점검한 기술 진단입니다.",
            "<details class=\"technical-details audit-diagnostic-details\">",
            "<th scope=\"row\">trace_audit/findings</th>",
            "<th scope=\"row\">trace_audit_command</th>",
            "audit failure raw",
            "cargo audit raw command output",
        ] {
            assert!(
                audit_section.contains(phrase),
                "missing collapsed audit diagnostic phrase: {phrase}"
            );
        }
        assert!(
            !audit_section.contains("status-failed"),
            "audit diagnostics must not reuse failure visual styling"
        );
        assert!(
            !audit_section.contains("trace audit findings"),
            "audit diagnostics should use Korean reliability wording instead of raw failure-style headings"
        );
        assert!(
            !audit_section.contains("trace audit command"),
            "audit diagnostics should use Korean reliability wording instead of raw failure-style headings"
        );
    }

    #[test]
    fn cycle_report_artifact_html_links_to_diff_artifact_without_rendering_hunk_lines() {
        let artifact = sample_cycle_report_artifact();
        let html = render_cycle_report_artifact_html(&artifact);

        for phrase in [
            "Diff 전용 artifact 색인",
            "/reports/cycle-report-route-test/diff.html",
            "apps/xavi-dev-console/src/lib.rs",
            "언어: rust",
            "변경: modified",
            "@@ open-cycle readiness @@",
            "open-cycle 명령이 같은 서버를 재사용하도록 변경",
            "file-1-apps-xavi-dev-console-src-lib-rs",
            "hunk-1-1",
        ] {
            assert!(html.contains(phrase), "missing diff index phrase: {phrase}");
        }
        assert!(!html.contains("diff-line remove"));
        assert!(!html.contains("diff-line add"));
        assert!(!html.contains("diff-line context"));
        assert!(!html.contains("<span class=\"line-no\">122</span>"));
        assert!(!html.contains("+let probe = probe_existing_report_server(config)?;"));
        assert!(!html.contains("-let url = fallback_to_any_open_port();"));
    }

    #[test]
    fn cycle_report_diff_html_renders_code_changes_as_full_width_diff() {
        let artifact = sample_cycle_report_artifact();
        let html = render_cycle_report_diff_html(&artifact);

        for phrase in [
            "Diff 전용 artifact",
            "줄바꿈",
            "white-space: pre",
            "overflow-x: auto",
            "apps/xavi-dev-console/src/lib.rs",
            "언어: rust",
            "변경: modified",
            "역할: dev-console, codegen",
            "raw_diff_ref: raw.json#diff-1",
            "별칭: feature-001",
            "category: feature",
            "sequence: 1",
            "@@ open-cycle readiness @@",
            "old 120 / 3",
            "new 120 / 4",
            "기존 fallback 없는 서버 재사용 검사를 추가했다.",
            "line 122",
            "health 확인 뒤 같은 report server만 재사용한다.",
            "file-1-apps-xavi-dev-console-src-lib-rs",
            "hunk-1-1",
        ] {
            assert!(html.contains(phrase), "missing diff phrase: {phrase}");
        }
        assert!(html.contains("diff-line remove"));
        assert!(html.contains("diff-line add"));
        assert!(html.contains("diff-line context"));
        assert!(html.contains("<span class=\"line-no\">122</span>"));
        assert!(html.contains("+let probe = probe_existing_report_server(config)?;"));
        assert!(html.contains("-let url = fallback_to_any_open_port();"));
        assert!(!html.contains("1500px"));
        let shell_rule_start =
            html.find(".shell {").expect("diff html should include the shell CSS rule");
        let shell_rule_end =
            html[shell_rule_start..].find('}').expect("diff shell CSS rule should close");
        let shell_rule = &html[shell_rule_start..shell_rule_start + shell_rule_end];
        assert!(shell_rule.contains("width: calc(100vw - 24px);"));
        assert!(!shell_rule.contains("max-width"));
    }

    #[test]
    fn cycle_report_artifact_html_renders_verbatim_evidence_metadata() {
        let artifact = sample_cycle_report_artifact();
        let html = render_cycle_report_artifact_html(&artifact);

        for phrase in [
            "user_request_verbatim",
            "사용자 원문 요청 본문",
            "<th scope=\"row\">source_type</th>",
            "development_trace",
            "<th scope=\"row\">source_ref</th>",
            "trace://cycle-report-route-test/user/1",
            "<th scope=\"row\">hash_sha256</th>",
            "9d7c0293b90b4267d298eefa833d44beaf414f9a68abb6f4d2d0773573861214",
            "<th scope=\"row\">timestamp</th>",
            "unix:1",
            "<th scope=\"row\">order</th>",
            "<td>1</td>",
            "prompt_verbatim",
            "codegen 원문 지시",
            "trace://cycle-report-route-test/dispatch/codegen",
            "e61ad81c12930455e55cc9b5e35ce7fdc7a7b7ddd9d44132f16965480d9ecd62",
        ] {
            assert!(html.contains(phrase), "missing evidence phrase: {phrase}");
        }
    }

    #[test]
    fn cycle_report_artifact_html_renders_all_command_verbatim_evidence_fields() {
        let command_verbatim = test_verbatim_evidence_json(
            "cargo test -p xavi-dev-console",
            "trace://cycle-command-evidence-test/test/command",
            "test",
            Some("agent-test-command"),
            4,
        );
        let result_verbatim = test_verbatim_evidence_json(
            "exit status 0",
            "trace://cycle-command-evidence-test/test/result",
            "test",
            Some("agent-test-command"),
            5,
        );
        let output_verbatim = test_verbatim_evidence_json(
            "running 1 test\nok",
            "trace://cycle-command-evidence-test/test/output",
            "test",
            Some("agent-test-command"),
            6,
        );
        let report_json = format!(
            r#"{{
              "cycle_id":"cycle-command-evidence-test",
              "status":"success",
              "evidence_status":"complete",
              "user_request_verbatim":null,
              "user_request_display_summary_ko":"명령 evidence 검증",
              "result_summary":"명령 evidence 표시 검증",
              "orchestra_instruction":"명령 원문을 모두 표시한다",
              "orchestra_delegations":[],
              "role_returns":{{}},
              "failure_point":"해당 없음",
              "verification_result":"passed",
              "changed_files":[],
              "code_changes":[],
              "commands":[
                {{
                  "command":"cargo test -p xavi-dev-console",
                  "actor":"test",
                  "result":"passed",
                  "evidence":"test command raw evidence",
                  "command_verbatim":{command_verbatim},
                  "result_verbatim":{result_verbatim},
                  "output_verbatim":{output_verbatim}
                }}
              ],
              "audit":{{
                "status":"pass",
                "missing_required_inputs":[],
                "warnings":[],
                "missing_evidence":[],
                "derived_not_verbatim":[]
              }}
            }}"#
        );
        let artifact = CycleReportArtifact::from_parts(
            "cycle-command-evidence-test",
            report_json,
            Some("[]".to_owned()),
            Some(r#"{"status":"pass","missing_evidence":[]}"#.to_owned()),
            Some("# context\n".to_owned()),
        );

        let html = render_cycle_report_artifact_html(&artifact);

        for phrase in [
            "<strong>command_verbatim</strong>",
            "<strong>result_verbatim</strong>",
            "<strong>output_verbatim</strong>",
        ] {
            assert!(html.contains(phrase), "missing command evidence field: {phrase}");
        }
    }

    #[test]
    fn cycle_report_artifact_html_warns_when_verbatim_evidence_is_missing() {
        let artifact = CycleReportArtifact::from_parts(
            "cycle-missing-evidence",
            r#"{
              "cycle_id":"cycle-missing-evidence",
              "status":"success",
              "user_request":"legacy request summary only",
              "result_summary":"legacy report",
              "orchestra_instruction":"legacy orchestra summary only",
              "role_returns":{},
              "failure_point":"해당 없음",
              "verification_result":"not run",
              "changed_files":[],
              "audit":{
                "status":"warn",
                "missing_required_inputs":[],
                "warnings":[],
                "missing_evidence":["user_request_verbatim"],
                "derived_not_verbatim":["user_request"]
              }
            }"#
            .to_owned(),
            Some("[]".to_owned()),
            Some(
                r#"{"status":"warn","missing_evidence":["user_request_verbatim"],"derived_not_verbatim":["user_request"]}"#
                    .to_owned(),
            ),
            Some("# context\n".to_owned()),
        );

        let html = render_cycle_report_artifact_html(&artifact);

        assert!(html.contains("원문 증거 없음"));
        assert!(html.contains("legacy/derived summary"));
        assert!(html.contains("legacy request summary only"));
        assert!(html.contains("derived summary, not verbatim"));
        assert!(!html.contains("legacy request summary only</pre>"));
    }

    #[test]
    fn cycle_report_artifact_html_warns_when_verbatim_evidence_hash_is_invalid() {
        let artifact = CycleReportArtifact::from_parts(
            "cycle-invalid-evidence",
            r#"{
              "cycle_id":"cycle-invalid-evidence",
              "status":"failure",
              "user_request":"derived request summary",
              "user_request_verbatim":{
                "text":"forged original text",
                "source_type":"development_trace",
                "source_ref":"trace://cycle-invalid-evidence/user/1",
                "role":"user",
                "agent_id":null,
                "hash_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "timestamp":"unix:1",
                "order":1
              },
              "user_request_display_summary_ko":"derived request summary",
              "result_summary":"invalid evidence is rejected",
              "orchestra_instruction":"do not trust forged evidence",
              "role_returns":{},
              "failure_point":"invalid evidence",
              "verification_result":"not run",
              "changed_files":[],
              "audit":{
                "status":"fail",
                "missing_required_inputs":[],
                "warnings":["invalid user_request_verbatim hash"],
                "missing_evidence":["user_request_verbatim"],
                "derived_not_verbatim":["user_request"]
              }
            }"#
            .to_owned(),
            Some("[]".to_owned()),
            Some(
                r#"{"status":"fail","missing_evidence":["user_request_verbatim"],"derived_not_verbatim":["user_request"]}"#
                    .to_owned(),
            ),
            Some("# context\n".to_owned()),
        );

        let html = render_cycle_report_artifact_html(&artifact);

        assert!(html.contains("원문 증거 무효"));
        assert!(html.contains("hash_sha256 does not match sha256(text)"));
        assert!(html.contains("derived request summary"));
        assert!(!html.contains("<pre>forged original text</pre>"));
    }

    #[test]
    fn cycle_report_artifact_html_shows_empty_code_changes_without_generating_artifact() {
        let artifact = CycleReportArtifact::from_parts(
            "cycle-empty-code-changes",
            r#"{
              "cycle_id":"cycle-empty-code-changes",
              "status":"success",
              "user_request":"empty diff test",
              "result_summary":"code_changes omitted",
              "orchestra_instruction":"do not synthesize",
              "role_returns":{},
              "failure_point":"해당 없음",
              "verification_result":"not run",
              "changed_files":[]
            }"#
            .to_owned(),
            Some("[]".to_owned()),
            Some(r#"{"status":"pass"}"#.to_owned()),
            Some("# context\n".to_owned()),
        );

        let html = render_cycle_report_artifact_html(&artifact);

        assert!(html.contains("표시할 diff hunk 색인 없음"));
        assert!(html.contains("메인 화면은 diff를 합성하지 않고 전용 artifact 링크만 유지합니다."));
        assert!(!html.contains("trace DB"));
    }

    #[test]
    fn cycle_report_schema_template_is_valid_json() {
        let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/agent/cycle-report/templates/report.schema.json");
        let output = Command::new("python3")
            .arg("-m")
            .arg("json.tool")
            .arg(&schema_path)
            .output()
            .expect("python3 json.tool should run for schema validation");

        assert!(
            output.status.success(),
            "schema JSON should parse: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let schema = std::fs::read_to_string(schema_path).expect("schema should be readable");
        assert!(schema.contains("\"user_request_verbatim\""));
        assert!(schema.contains("\"orchestra_delegations\""));
        assert!(schema.contains("\"derived_summaries\""));
        assert!(schema.contains("\"missing_evidence\""));
        assert!(schema.contains("\"invalid_evidence\""));
        assert!(schema.contains("\"derived_not_verbatim\""));
        assert!(schema.contains("\"timestamp\""));
        assert!(schema.contains("\"order\""));
        assert!(schema.contains("\"cycle_alias\""));
        assert!(schema.contains("\"cycle_category\""));
        assert!(schema.contains("\"cycle_category_key\""));
        assert!(schema.contains("\"cycle_sequence\""));
        assert!(schema.contains("\"cycle_title\""));
        assert!(schema.contains("\"aliases_json\""));
    }

    #[test]
    fn sample_cycle_report_top_level_keys_match_schema_properties() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let schema_path = repo_root.join("docs/agent/cycle-report/templates/report.schema.json");

        let schema = std::fs::read_to_string(schema_path).expect("schema should be readable");
        let report = SAMPLE_CYCLE_REPORT_JSON;

        let report_keys = json_object_direct_keys(report);
        let schema_properties =
            json_value_field_snippet(&schema, "properties").expect("schema should have properties");
        let schema_property_keys = json_object_direct_keys(&schema_properties);
        let unknown_report_keys = report_keys
            .iter()
            .filter(|key| !schema_property_keys.contains(key))
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            unknown_report_keys.is_empty(),
            "report top-level keys missing from schema.properties: {unknown_report_keys:?}"
        );

        let schema_required = json_value_field_snippet(&schema, "required")
            .expect("schema should have required keys");
        let required_keys = json_string_array_direct_values(&schema_required);
        let missing_required_keys = required_keys
            .iter()
            .filter(|key| !report_keys.contains(key))
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            missing_required_keys.is_empty(),
            "schema required top-level keys missing from sample report: {missing_required_keys:?}"
        );

        let evidence_status = json_text_field(report, "evidence_status")
            .expect("report should include evidence_status");
        assert!(
            matches!(evidence_status.as_str(), "complete" | "incomplete" | "fail"),
            "unexpected evidence_status: {evidence_status}"
        );
    }

    #[test]
    fn cycle_report_index_html_uses_latest_json_without_polling() {
        let latest_json = r#"[{"cycle_id":"cycle-report-route-test","status":"failed"}]"#;
        let aliases_json = r#"{"version":1,"aliases":[{"cycle_id":"cycle-report-route-test","cycle_alias":"feature-001","cycle_category":"feature","cycle_category_key":"feature","cycle_sequence":1,"cycle_title":"별칭 라우트 테스트","created_at":"unix:1"}]}"#;
        let html =
            render_cycle_report_index_html_with_aliases(Some(latest_json), Some(aliases_json));

        assert!(html.contains("latest.json 목록"));
        assert!(html.contains("aliases.json 별칭 목록"));
        assert!(html.contains("/reports/cycle-report-route-test/"));
        assert!(html.contains("/reports/by-alias/feature-001/"));
        assert!(html.contains("별칭 라우트 테스트"));
        assert!(html.contains("직접 URL/수동 새로고침"));
        assert!(html.contains("수동 새로고침"));
        assert!(!html.contains("EventSource"));
        assert!(!html.contains("setInterval"));
        assert!(!html.contains("setTimeout"));
    }

    #[test]
    fn cycle_report_artifact_routes_read_temp_reports_dir() {
        let reports_dir = test_reports_dir("artifact-routes");
        write_sample_report_artifact(&reports_dir, "cycle-report-route-test");

        let index_response = handle_single_request_with_reports_dir(
            "GET /reports HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let html_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let diff_html_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/diff.html HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let latest_response = handle_single_request_with_reports_dir(
            "GET /api/reports HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let aliases_response = handle_single_request_with_reports_dir(
            "GET /api/reports/aliases.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let report_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/report.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let alias_report_response = handle_single_request_with_reports_dir(
            "GET /api/reports/by-alias/feature-001/report.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let raw_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/raw.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let audit_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/audit.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let context_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/context.md HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let alias_response = handle_single_request_with_reports_dir(
            "GET /cycles/cycle-report-route-test/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let by_alias_response = handle_single_request_with_reports_dir(
            "GET /reports/by-alias/feature-001/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let artifact_index_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/index.html HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let artifact_diff_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/diff.html HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let ready_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/ready HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(index_response.starts_with("HTTP/1.1 200 OK"));
        assert!(index_response.contains("cycle-report-route-test"));
        assert!(html_response.starts_with("HTTP/1.1 200 OK"));
        assert!(html_response.contains("Content-Type: text/html; charset=utf-8"));
        assert!(html_response.contains("Xavi cycle report"));
        assert!(html_response.contains("diff 전용 artifact 링크"));
        assert!(html_response.contains("prebuilt cycle-report index"));
        assert!(!html_response.contains("+let probe = probe_existing_report_server(config)?;"));
        assert!(diff_html_response.starts_with("HTTP/1.1 200 OK"));
        assert!(diff_html_response.contains("Content-Type: text/html; charset=utf-8"));
        assert!(diff_html_response.contains("Xavi cycle diff"));
        assert!(diff_html_response.contains("전체 diff hunk"));
        assert!(diff_html_response.contains("+let probe = probe_existing_report_server(config)?;"));
        assert!(latest_response.starts_with("HTTP/1.1 200 OK"));
        assert!(latest_response.contains("Content-Type: application/json; charset=utf-8"));
        assert!(latest_response.contains("\"cycle_id\":\"cycle-report-route-test\""));
        assert!(aliases_response.starts_with("HTTP/1.1 200 OK"));
        assert!(aliases_response.contains("\"cycle_alias\":\"feature-001\""));
        assert!(report_response.starts_with("HTTP/1.1 200 OK"));
        assert!(report_response.contains("\"failure_point\""));
        assert!(alias_report_response.starts_with("HTTP/1.1 200 OK"));
        assert!(alias_report_response.contains("\"cycle_id\":\"cycle-report-route-test\""));
        assert!(alias_report_response.contains("\"cycle_alias\":\"feature-001\""));
        assert!(raw_response.starts_with("HTTP/1.1 200 OK"));
        assert!(raw_response.contains("raw-event-1"));
        assert!(audit_response.starts_with("HTTP/1.1 200 OK"));
        assert!(audit_response.contains("audit failure raw"));
        assert!(context_response.starts_with("HTTP/1.1 200 OK"));
        assert!(context_response.contains("Content-Type: text/markdown; charset=utf-8"));
        assert!(context_response.contains("context markdown raw"));
        assert!(alias_response.starts_with("HTTP/1.1 200 OK"));
        assert!(alias_response.contains("diff 전용 artifact 링크"));
        assert!(by_alias_response.starts_with("HTTP/1.1 200 OK"));
        assert!(by_alias_response.contains("cycle-report-route-test"));
        assert!(by_alias_response.contains("feature-001"));
        assert!(artifact_index_response.starts_with("HTTP/1.1 200 OK"));
        assert!(artifact_index_response.contains("Content-Type: text/html; charset=utf-8"));
        assert!(artifact_index_response.contains("prebuilt cycle-report index"));
        assert!(artifact_diff_response.starts_with("HTTP/1.1 200 OK"));
        assert!(artifact_diff_response.contains("Content-Type: text/html; charset=utf-8"));
        assert!(artifact_diff_response.contains("prebuilt cycle-report diff"));
        assert!(
            artifact_diff_response.contains("+let probe = probe_existing_report_server(config)?;")
        );
        assert!(ready_response.starts_with("HTTP/1.1 200 OK"));
        assert!(ready_response.contains("\"service\":\"xavi-dev-console\""));
        assert!(ready_response.contains("\"reports_dir\":"));
        assert!(ready_response.contains("\"cycle_id\":\"cycle-report-route-test\""));
        assert!(ready_response.contains("\"index_html_bytes\":"));
        assert!(ready_response.contains("\"diff_html_present\":true"));
        assert!(ready_response.contains("\"artifact_files_present\":true"));
    }

    #[test]
    fn cycle_report_readiness_keeps_old_core_artifact_ready_without_diff_html() {
        let reports_dir = test_reports_dir("artifact-routes-no-diff");
        let cycle_id = "cycle-report-route-test";
        write_sample_report_artifact(&reports_dir, cycle_id);
        std::fs::remove_file(reports_dir.join(cycle_id).join("diff.html"))
            .expect("diff artifact should be removed for old artifact fixture");

        let index_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let ready_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/ready HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let diff_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/diff.html HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(index_response.starts_with("HTTP/1.1 200 OK"));
        assert!(index_response.contains("prebuilt cycle-report index"));
        assert!(ready_response.starts_with("HTTP/1.1 200 OK"));
        assert!(ready_response.contains("\"artifact_files_present\":true"));
        assert!(ready_response.contains("\"diff_html_present\":false"));
        assert!(diff_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(diff_response.contains("missing cycle-report artifact file"));
    }

    #[cfg(unix)]
    #[test]
    fn cycle_report_artifact_routes_reject_cycle_dir_symlink_escape() {
        let reports_dir = test_reports_dir("cycle-dir-symlink-escape");
        let outside_reports_dir = test_reports_dir("cycle-dir-symlink-outside");
        let cycle_id = "cycle-report-route-test";
        write_sample_report_artifact(&outside_reports_dir, cycle_id);
        std::os::unix::fs::symlink(outside_reports_dir.join(cycle_id), reports_dir.join(cycle_id))
            .expect("cycle dir escape symlink should be created");

        let html_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let ready_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/ready HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(html_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(html_response.contains("escapes reports root"));
        assert!(!html_response.contains("prebuilt cycle-report index"));
        assert!(ready_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(ready_response.contains("\"error\":\"report_artifact_not_ready\""));
        assert!(ready_response.contains("escapes reports root"));
    }

    #[cfg(unix)]
    #[test]
    fn cycle_report_artifact_routes_reject_index_html_symlink() {
        let reports_dir = test_reports_dir("index-symlink");
        let cycle_id = "cycle-report-route-test";
        write_sample_report_artifact(&reports_dir, cycle_id);
        let cycle_dir = reports_dir.join(cycle_id);
        let real_index_path = cycle_dir.join("real-index.html");
        std::fs::rename(cycle_dir.join("index.html"), &real_index_path)
            .expect("index artifact should be renamed");
        std::os::unix::fs::symlink(&real_index_path, cycle_dir.join("index.html"))
            .expect("index symlink should be created");

        let html_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let index_api_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/index.html HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let ready_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/ready HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(html_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(html_response.contains("index.html must not be a symlink"));
        assert!(!html_response.contains("prebuilt cycle-report index"));
        assert!(index_api_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(index_api_response.contains("\"error\":\"report_artifact_not_found\""));
        assert!(index_api_response.contains("index.html must not be a symlink"));
        assert!(ready_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(ready_response.contains("\"error\":\"report_artifact_not_ready\""));
        assert!(ready_response.contains("index.html must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn cycle_report_artifact_routes_reject_diff_html_symlink_without_exposing_target() {
        let reports_dir = test_reports_dir("diff-symlink");
        let outside_dir = test_reports_dir("diff-symlink-outside");
        let cycle_id = "cycle-report-route-test";
        write_sample_report_artifact(&reports_dir, cycle_id);
        let escaped_diff_body = "escaped diff should not be served";
        let escaped_diff_path = outside_dir.join("escaped-diff.html");
        std::fs::write(&escaped_diff_path, escaped_diff_body)
            .expect("escaped diff fixture should write");
        let diff_path = reports_dir.join(cycle_id).join("diff.html");
        std::fs::remove_file(&diff_path).expect("sample diff should be removed");
        std::os::unix::fs::symlink(&escaped_diff_path, &diff_path)
            .expect("diff symlink should be created");

        let diff_page_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/diff.html HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let diff_api_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/diff.html HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let ready_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/ready HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(diff_page_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(diff_page_response.contains("diff.html must not be a symlink"));
        assert!(!diff_page_response.contains(escaped_diff_body));
        assert!(diff_api_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(diff_api_response.contains("\"error\":\"report_artifact_not_found\""));
        assert!(diff_api_response.contains("diff.html must not be a symlink"));
        assert!(!diff_api_response.contains(escaped_diff_body));
        assert!(ready_response.starts_with("HTTP/1.1 200 OK"));
        assert!(ready_response.contains("\"artifact_files_present\":true"));
        assert!(ready_response.contains("\"diff_html_present\":false"));
        assert!(!ready_response.contains("diff.html must not be a symlink"));
        assert!(!ready_response.contains(escaped_diff_body));
    }

    #[cfg(unix)]
    #[test]
    fn cycle_report_artifact_routes_reject_required_artifact_symlink_escape() {
        let reports_dir = test_reports_dir("artifact-file-symlink-escape");
        let outside_dir = test_reports_dir("artifact-file-symlink-outside");
        let cycle_id = "cycle-report-route-test";
        write_sample_report_artifact(&reports_dir, cycle_id);
        let escaped_report_path = outside_dir.join("escaped-report.json");
        std::fs::write(
            &escaped_report_path,
            r#"{"cycle_id":"escaped-cycle","secret":"escaped report should not be served"}"#,
        )
        .expect("escaped report fixture should write");
        let report_path = reports_dir.join(cycle_id).join("report.json");
        std::fs::remove_file(&report_path).expect("sample report should be removed");
        std::os::unix::fs::symlink(&escaped_report_path, &report_path)
            .expect("artifact symlink escape should be created");

        let html_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let report_api_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/report.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let ready_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/ready HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(html_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(html_response.contains("escapes cycle-report artifact directory"));
        assert!(!html_response.contains("escaped report should not be served"));
        assert!(report_api_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(report_api_response.contains("\"error\":\"report_artifact_not_found\""));
        assert!(report_api_response.contains("escapes cycle-report artifact directory"));
        assert!(!report_api_response.contains("escaped report should not be served"));
        assert!(ready_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(ready_response.contains("\"error\":\"report_artifact_not_ready\""));
        assert!(ready_response.contains("escapes cycle-report artifact directory"));
    }

    #[test]
    fn cycle_report_alias_routes_fail_closed_for_traversal_inputs() {
        let reports_dir = test_reports_dir("alias-route-traversal");
        write_sample_report_artifact(&reports_dir, "cycle-report-route-test");

        let encoded_alias_slash_response = handle_single_request_with_reports_dir(
            "GET /reports/by-alias/feature%2F001/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let encoded_api_alias_slash_response = handle_single_request_with_reports_dir(
            "GET /api/reports/by-alias/feature%2F001/report.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let encoded_file_traversal_response = handle_single_request_with_reports_dir(
            "GET /api/reports/by-alias/feature-001/..%2Freport.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(encoded_alias_slash_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(encoded_alias_slash_response.contains("invalid cycle report alias"));
        assert!(!encoded_alias_slash_response.contains("전체 diff hunk"));
        assert!(encoded_api_alias_slash_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(encoded_api_alias_slash_response.contains("invalid cycle report alias"));
        assert!(
            !encoded_api_alias_slash_response.contains("\"cycle_id\":\"cycle-report-route-test\"")
        );
        assert!(encoded_file_traversal_response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(
            encoded_file_traversal_response.contains("\"error\":\"unsupported_report_artifact\"")
        );
        assert!(
            !encoded_file_traversal_response.contains("\"cycle_id\":\"cycle-report-route-test\"")
        );
    }

    #[test]
    fn cycle_report_alias_routes_fail_closed_for_ambiguous_alias_index() {
        let reports_dir = test_reports_dir("alias-route-ambiguous");
        write_sample_report_artifact(&reports_dir, "cycle-report-route-test");
        std::fs::write(
            reports_dir.join("aliases.json"),
            r#"{"version":1,"aliases":[{"cycle_id":"cycle-report-route-test","cycle_alias":"feature-001","cycle_category":"feature","cycle_category_key":"feature","cycle_sequence":1,"cycle_title":"first","created_at":"unix:1"},{"cycle_id":"cycle-report-route-other","cycle_alias":"feature-001","cycle_category":"feature","cycle_category_key":"feature","cycle_sequence":1,"cycle_title":"second","created_at":"unix:2"}]}"#,
        )
        .expect("ambiguous alias index should write");

        let response = handle_single_request_with_reports_dir(
            "GET /api/reports/by-alias/feature-001/report.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(response.contains("maps ambiguously"));
        assert!(!response.contains("\"cycle_id\":\"cycle-report-route-test\""));
    }

    #[test]
    fn cycle_report_alias_routes_fail_closed_for_malformed_alias_entry() {
        let reports_dir = test_reports_dir("alias-route-malformed-entry");
        write_sample_report_artifact(&reports_dir, "cycle-report-route-test");
        std::fs::write(
            reports_dir.join("aliases.json"),
            r#"{"version":1,"aliases":[{"cycle_alias":"feature-001","cycle_category":"feature","cycle_category_key":"feature","cycle_sequence":1,"cycle_title":"missing cycle id","created_at":"unix:1"}]}"#,
        )
        .expect("malformed alias index should write");

        let aliases_response = handle_single_request_with_reports_dir(
            "GET /api/reports/aliases.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let by_alias_response = handle_single_request_with_reports_dir(
            "GET /api/reports/by-alias/feature-001/report.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let page_response = handle_single_request_with_reports_dir(
            "GET /reports/by-alias/feature-001/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let canonical_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(aliases_response.starts_with("HTTP/1.1 422 Unprocessable Entity"));
        assert!(aliases_response.contains("\"error\":\"report_alias_index_malformed\""));
        assert!(aliases_response.contains("aliases[0] missing field cycle_id"));
        assert!(by_alias_response.starts_with("HTTP/1.1 422 Unprocessable Entity"));
        assert!(by_alias_response.contains("\"error\":\"report_alias_index_malformed\""));
        assert!(!by_alias_response.contains("\"failure_point\""));
        assert!(page_response.starts_with("HTTP/1.1 422 Unprocessable Entity"));
        assert!(page_response.contains("report alias index malformed"));
        assert!(!page_response.contains("전체 diff hunk"));
        assert!(canonical_response.starts_with("HTTP/1.1 200 OK"));
        assert!(canonical_response.contains("diff 전용 artifact 링크"));

        std::fs::write(
            reports_dir.join("aliases.json"),
            r#"{"version":1,"aliases":[{"cycle_id":"cycle-report-route-test","cycle_alias":"feature-001","cycle_category":"feature","cycle_category_key":"feature","cycle_sequence":"1","cycle_title":"bad sequence type","created_at":"unix:1"}]}"#,
        )
        .expect("malformed alias field index should write");

        let malformed_field_response = handle_single_request_with_reports_dir(
            "GET /api/reports/aliases.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(malformed_field_response.starts_with("HTTP/1.1 422 Unprocessable Entity"));
        assert!(malformed_field_response.contains("\"error\":\"report_alias_index_malformed\""));
        assert!(malformed_field_response.contains("field cycle_sequence must be a JSON integer"));
    }

    #[test]
    fn cycle_report_alias_index_page_surfaces_damaged_aliases_json() {
        let reports_dir = test_reports_dir("alias-route-damaged-index");
        write_sample_report_artifact(&reports_dir, "cycle-report-route-test");
        std::fs::write(reports_dir.join("aliases.json"), r#"{"version":1,"aliases":["#)
            .expect("damaged alias index should write");

        let index_response = handle_single_request_with_reports_dir(
            "GET /reports HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let aliases_response = handle_single_request_with_reports_dir(
            "GET /api/reports/aliases.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(index_response.starts_with("HTTP/1.1 200 OK"));
        assert!(index_response.contains("aliases.json 오류"));
        assert!(index_response.contains("aliases.json malformed"));
        assert!(!index_response.contains("/reports/by-alias/feature-001/"));
        assert!(aliases_response.starts_with("HTTP/1.1 422 Unprocessable Entity"));
        assert!(aliases_response.contains("\"error\":\"report_alias_index_malformed\""));
        assert!(aliases_response.contains("root JSON value is incomplete"));
    }

    #[test]
    fn bounded_text_file_read_rejects_oversize_file() {
        let reports_dir = test_reports_dir("bounded-read");
        let file_path = reports_dir.join("small-limit.txt");
        std::fs::write(&file_path, "123456789").expect("bounded read fixture should write");

        let error =
            read_text_file_bounded(&file_path, 8).expect_err("oversize file should fail closed");

        assert!(is_text_file_size_limit_error(error.as_ref()));
        assert!(error.to_string().contains("exceeded size limit"));
        assert!(error.to_string().contains("8 bytes"));
    }

    #[test]
    fn cycle_report_artifact_bundle_total_uses_same_size_limit() {
        let artifact = CycleReportArtifact::from_parts(
            "cycle-bundle-limit",
            "12345".to_owned(),
            Some("6789".to_owned()),
            Some("0".to_owned()),
            None,
        );
        let reports_dir = test_reports_dir("bundle-limit");

        let error =
            ensure_cycle_report_artifact_text_total_within_limit(&artifact, &reports_dir, 9)
                .expect_err("bundle source total should fail closed");

        assert!(is_text_file_size_limit_error(error.as_ref()));
        assert!(error.to_string().contains("10 bytes > 9 bytes"));
    }

    #[test]
    fn cycle_report_artifact_api_returns_413_while_html_uses_index_for_oversize_artifact_file() {
        let reports_dir = test_reports_dir("oversize-artifact");
        let cycle_id = "cycle-report-route-test";
        write_sample_report_artifact(&reports_dir, cycle_id);
        std::fs::write(
            reports_dir.join(cycle_id).join("report.json"),
            vec![b'a'; MAX_REPORT_ARTIFACT_BYTES + 1],
        )
        .expect("oversize report artifact should write");

        let api_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/report.json HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let html_response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );
        let ready_response = handle_single_request_with_reports_dir(
            "GET /api/reports/cycle-report-route-test/ready HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(api_response.starts_with("HTTP/1.1 413 Payload Too Large"));
        assert!(api_response.contains("\"error\":\"report_artifact_too_large\""));
        assert!(api_response.contains(&MAX_REPORT_ARTIFACT_BYTES.to_string()));
        assert!(html_response.starts_with("HTTP/1.1 200 OK"));
        assert!(html_response.contains("Content-Type: text/html; charset=utf-8"));
        assert!(html_response.contains("prebuilt cycle-report index"));
        assert!(!html_response.contains("trace DB"));
        assert!(ready_response.starts_with("HTTP/1.1 200 OK"));
        assert!(ready_response.contains("\"artifact_files_present\":true"));
        assert!(ready_response.contains("\"index_html_bytes\":"));
    }

    #[test]
    fn missing_cycle_report_artifact_does_not_fallback_to_trace_db() {
        let reports_dir = test_reports_dir("missing-artifact");
        let response = handle_single_request_with_reports_dir(
            "GET /reports/missing-cycle/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(response.contains("report artifact not found"));
    }

    #[test]
    fn missing_cycle_report_index_does_not_render_from_report_json() {
        let reports_dir = test_reports_dir("missing-index");
        let cycle_dir = reports_dir.join("cycle-report-route-test");
        std::fs::create_dir_all(&cycle_dir).expect("cycle artifact dir should be created");
        std::fs::write(cycle_dir.join("report.json"), sample_cycle_report_artifact().report_json)
            .expect("report artifact should write");

        let response = handle_single_request_with_reports_dir(
            "GET /reports/cycle-report-route-test/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            &reports_dir,
        );

        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(response.contains("report artifact not found"));
        assert!(!response.contains("실패 분석"));
    }

    #[test]
    fn copy_cycle_report_artifact_bundle_copies_existing_files_without_trace_db() {
        let reports_dir = test_reports_dir("copy-source");
        write_sample_report_artifact(&reports_dir, "cycle-report-route-test");
        let output_dir = test_reports_dir("copy-output").join("cycle-export-test");

        copy_cycle_report_artifact_bundle(&reports_dir, "cycle-report-route-test", &output_dir)
            .expect("existing artifact bundle should copy into temp dir");

        for file_name in
            ["index.html", "diff.html", "report.json", "raw.json", "audit.json", "context.md"]
        {
            assert!(
                output_dir.join(file_name).exists(),
                "missing copied artifact file: {file_name}"
            );
        }
        let html = std::fs::read_to_string(output_dir.join("index.html"))
            .expect("copied html should read");
        let raw = std::fs::read_to_string(output_dir.join("raw.json"))
            .expect("copied raw json should read");
        assert!(html.contains("prebuilt cycle-report index"));
        let diff = std::fs::read_to_string(output_dir.join("diff.html"))
            .expect("copied diff html should read");
        assert!(diff.contains("prebuilt cycle-report diff"));
        assert!(raw.contains("raw-event-1"));
    }

    #[test]
    fn validate_cycle_report_artifact_bundle_requires_cycle_report_files() {
        let reports_dir = test_reports_dir("incomplete-artifact");
        let cycle_dir = reports_dir.join("cycle-report-route-test");
        std::fs::create_dir_all(&cycle_dir).expect("cycle artifact dir should be created");
        std::fs::write(cycle_dir.join("index.html"), "partial")
            .expect("partial artifact should write");

        let error = validate_cycle_report_artifact_bundle(&reports_dir, "cycle-report-route-test")
            .expect_err("incomplete artifact should fail validation");

        assert!(error.to_string().contains("missing cycle-report artifact file"));
    }

    #[test]
    fn open_cycle_plans_report_route_server_command_and_browser_open() {
        let config = OpenCycleConfig {
            cycle_id: "cycle with space".to_owned(),
            reports_dir: ".xavi/reports/development_cycles".to_owned(),
            host: "127.0.0.1".to_owned(),
            port: 4200,
            browser_mode: OpenCycleBrowserMode::PrintUrlOnly,
        };

        let url = cycle_report_url(&config.host, config.port, &config.cycle_id);
        let server_plan = open_cycle_server_start_plan("/tmp/xavi-dev-console", &config);
        let browser_plan = browser_open_plan(&url);

        assert_eq!(url, "http://127.0.0.1:4200/reports/cycle%20with%20space/");
        assert_eq!(server_plan.program, PathBuf::from("/tmp/xavi-dev-console"));
        assert_eq!(
            server_plan.args,
            vec![
                "serve",
                "--cycle",
                "cycle with space",
                "--addr",
                "127.0.0.1:4200",
                "--reports-dir",
                ".xavi/reports/development_cycles"
            ]
        );
        assert_eq!(server_plan.spawn_mode, OpenCycleServerSpawnMode::DetachedPersistent);
        assert!(!server_plan.args.iter().any(|arg| arg == "open-cycle"));
        assert_eq!(browser_plan.program, PathBuf::from("/usr/bin/open"));
        assert_eq!(browser_plan.args, vec![url]);
        assert_eq!(config.browser_mode, OpenCycleBrowserMode::PrintUrlOnly);
    }

    #[test]
    fn open_cycle_report_server_spawn_mode_keeps_child_alive_after_handle_drop() {
        let plan = OpenCycleServerPlan {
            program: PathBuf::from("/bin/sleep"),
            args: vec!["5".to_owned()],
            spawn_mode: OpenCycleServerSpawnMode::DetachedPersistent,
        };
        let child = spawn_report_server(&plan).expect("detached child should spawn");
        let process_id = child.id();

        drop(child);
        thread::sleep(Duration::from_millis(100));

        assert!(
            process_is_running(process_id),
            "dropping the short-lived open-cycle handle must not kill the server process"
        );
        terminate_process(process_id);
    }

    #[test]
    fn open_cycle_probe_reuses_matching_report_server_with_readiness_marker() {
        let reports_dir = "/tmp/xavi-dev-console-probe-reports";
        let cycle_id = "cycle-report-route-test";
        let port = spawn_fake_report_server(
            &render_fake_health_json(reports_dir),
            "HTTP/1.1 200 OK",
            &render_fake_ready_json(reports_dir, cycle_id, 2048, true),
        );
        let config = OpenCycleConfig {
            cycle_id: cycle_id.to_owned(),
            reports_dir: reports_dir.to_owned(),
            host: "127.0.0.1".to_owned(),
            port,
            browser_mode: OpenCycleBrowserMode::PrintUrlOnly,
        };

        let probe = probe_existing_report_server(&config);

        assert_eq!(probe, ReportServerProbe::Matching);
    }

    #[test]
    fn open_cycle_probe_fails_closed_when_readiness_route_fails() {
        let reports_dir = "/tmp/xavi-dev-console-probe-ready-fails";
        let cycle_id = "cycle-report-route-test";
        let port = spawn_fake_report_server_with_html_and_request_limit(
            &render_fake_health_json(reports_dir),
            "HTTP/1.1 500 Internal Server Error",
            r#"{"error":"not ready"}"#,
            "HTTP/1.1 200 OK",
            &fake_report_html_body(cycle_id),
            2,
        );
        let config = OpenCycleConfig {
            cycle_id: cycle_id.to_owned(),
            reports_dir: reports_dir.to_owned(),
            host: "127.0.0.1".to_owned(),
            port,
            browser_mode: OpenCycleBrowserMode::PrintUrlOnly,
        };

        let probe = probe_existing_report_server(&config);

        assert!(matches!(
            probe,
            ReportServerProbe::WrongServer(message)
                if message.contains("readiness endpoint returned HTTP/1.1 500 Internal Server Error")
        ));
    }

    #[test]
    fn open_cycle_probe_fails_closed_when_readiness_marker_is_missing() {
        let reports_dir = "/tmp/xavi-dev-console-probe-ready-marker-missing";
        let cycle_id = "cycle-report-route-test";
        let port = spawn_fake_report_server_with_html_and_request_limit(
            &render_fake_health_json(reports_dir),
            "HTTP/1.1 200 OK",
            r#"{"status":"ok","reports_dir":"/tmp/xavi-dev-console-probe-ready-marker-missing","cycle_id":"cycle-report-route-test","index_html_bytes":2048,"artifact_files_present":true}"#,
            "HTTP/1.1 200 OK",
            &fake_report_html_body(cycle_id),
            2,
        );
        let config = OpenCycleConfig {
            cycle_id: cycle_id.to_owned(),
            reports_dir: reports_dir.to_owned(),
            host: "127.0.0.1".to_owned(),
            port,
            browser_mode: OpenCycleBrowserMode::PrintUrlOnly,
        };

        let probe = probe_existing_report_server(&config);

        assert!(matches!(
            probe,
            ReportServerProbe::WrongServer(message)
                if message.contains("readiness endpoint is missing xavi-dev-console service marker")
        ));
    }

    #[test]
    fn open_cycle_probe_fails_closed_when_readiness_reports_empty_index() {
        let reports_dir = "/tmp/xavi-dev-console-probe-empty-index";
        let cycle_id = "cycle-report-route-test";
        let port = spawn_fake_report_server_with_html_and_request_limit(
            &render_fake_health_json(reports_dir),
            "HTTP/1.1 200 OK",
            &render_fake_ready_json(reports_dir, cycle_id, 0, true),
            "HTTP/1.1 200 OK",
            &fake_report_html_body(cycle_id),
            2,
        );
        let config = OpenCycleConfig {
            cycle_id: cycle_id.to_owned(),
            reports_dir: reports_dir.to_owned(),
            host: "127.0.0.1".to_owned(),
            port,
            browser_mode: OpenCycleBrowserMode::PrintUrlOnly,
        };

        let probe = probe_existing_report_server(&config);

        assert!(matches!(
            probe,
            ReportServerProbe::WrongServer(message) if message.contains("invalid index.html")
        ));
    }

    #[test]
    fn open_cycle_probe_uses_readiness_without_fetching_large_html_route() {
        let reports_dir = "/tmp/xavi-dev-console-probe-large-html";
        let cycle_id = "cycle-report-route-test";
        let html_body = "x".repeat(REPORT_SERVER_HTTP_MAX_RESPONSE_BYTES + 1024);
        let port = spawn_fake_report_server_with_html_and_request_limit(
            &render_fake_health_json(reports_dir),
            "HTTP/1.1 200 OK",
            &render_fake_ready_json(reports_dir, cycle_id, html_body.len() as u64, true),
            "HTTP/1.1 413 Payload Too Large",
            &html_body,
            2,
        );
        let config = OpenCycleConfig {
            cycle_id: cycle_id.to_owned(),
            reports_dir: reports_dir.to_owned(),
            host: "127.0.0.1".to_owned(),
            port,
            browser_mode: OpenCycleBrowserMode::PrintUrlOnly,
        };

        let probe = probe_existing_report_server(&config);

        assert_eq!(probe, ReportServerProbe::Matching);
    }

    #[test]
    fn open_cycle_probe_fails_closed_for_mismatched_reports_dir() {
        let server_reports_dir = "/tmp/xavi-dev-console-probe-reports-server";
        let expected_reports_dir = "/tmp/xavi-dev-console-probe-reports-expected";
        let port = spawn_fake_report_server_with_request_limit(
            &render_fake_health_json(server_reports_dir),
            "HTTP/1.1 200 OK",
            r#"{"cycle_id":"cycle-report-route-test"}"#,
            1,
        );
        let config = OpenCycleConfig {
            cycle_id: "cycle-report-route-test".to_owned(),
            reports_dir: expected_reports_dir.to_owned(),
            host: "127.0.0.1".to_owned(),
            port,
            browser_mode: OpenCycleBrowserMode::PrintUrlOnly,
        };

        let probe = probe_existing_report_server(&config);

        assert!(matches!(
            probe,
            ReportServerProbe::WrongServer(message)
                if message.contains("different reports_dir")
                    && message.contains(server_reports_dir)
        ));
    }

    #[test]
    fn open_cycle_probe_fails_closed_for_wrong_server_on_port() {
        let port = spawn_fake_report_server_with_request_limit(
            r#"{"status":"ok"}"#,
            "HTTP/1.1 200 OK",
            r#"{"cycle_id":"cycle-report-route-test"}"#,
            1,
        );
        let config = OpenCycleConfig {
            cycle_id: "cycle-report-route-test".to_owned(),
            reports_dir: "/tmp/xavi-dev-console-probe-reports".to_owned(),
            host: "127.0.0.1".to_owned(),
            port,
            browser_mode: OpenCycleBrowserMode::PrintUrlOnly,
        };

        let probe = probe_existing_report_server(&config);

        assert!(
            matches!(probe, ReportServerProbe::WrongServer(message) if message.contains("service marker"))
        );
    }

    #[test]
    fn html_templates_debounce_sse_trace_refresh_without_auto_connect() {
        for (html, live_button_label) in [
            (render_console_html("cycle-dev-console-test", None), "실시간 시작"),
            (render_console_html_legacy("cycle-dev-console-test", None), "Start SSE"),
        ] {
            assert!(html.contains("function requestTraceRefresh()"));
            assert!(html.contains("traceRefreshInFlight"));
            assert!(html.contains("traceRefreshPending"));
            assert!(html.contains("traceRefreshDebounceTimer"));
            assert!(html.contains("setTimeout(runTraceRefresh, TRACE_REFRESH_DEBOUNCE_MS)"));
            assert!(html.contains("requestTraceRefresh();"));
            assert!(html.contains("id=\"live-button\""));
            assert!(html.contains(live_button_label));
            assert!(html.contains("let liveEventSource = null"));
            assert!(html.contains("if (liveEventSource)"));
            assert!(html.contains("liveEventSource = new EventSource"));
            assert!(html.contains("liveEventSource.addEventListener('trace'"));
            assert!(html.contains(
                "document.getElementById('live-button').addEventListener('click', connectEvents)"
            ));
            assert!(!html.contains("connectEvents();"));
        }
    }

    #[test]
    fn report_json_includes_trace_integrity_without_raw_trace_fields() {
        let legacy = DevelopmentTraceEntry {
            id: 1,
            event_id: "dispatch-codegen-legacy".to_owned(),
            cycle_id: "cycle-dev-console-test".to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::AgentDispatch,
            role_name: Some("codegen".to_owned()),
            summary: "codegen dispatch without contract".to_owned(),
            body: "codegen dispatch without contract".to_owned(),
            metadata_json: "{}".to_owned(),
            created_at: epoch_timestamp(),
        };
        let report = DevelopmentCycleReport::from_entries(
            "cycle-dev-console-test",
            (vec![legacy.clone()], 1),
            audit_development_trace_cycle("cycle-dev-console-test", &[legacy]),
        );
        let json = render_report_json(&report);

        assert_eq!(report.trace_integrity.status, "failed");
        assert!(json.contains("\"trace_integrity\""));
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"code\":\"invalid_trace_contract\""));
        assert!(json.contains("trace_contract_version"));
        assert!(!json.contains("\"metadata_json\""));
        assert!(!json.contains("\"body\""));
        assert!(!json.contains("\"user_turn_id\""));
    }

    #[test]
    fn sse_trace_frame_uses_trace_event_and_public_projection() {
        let service = test_service();
        let entry = append_role_entry(
            &service,
            DevelopmentTraceKind::AgentDispatch,
            "planning",
            "planning dispatch",
            "{}",
        );
        let frame = sse_trace_frame(&entry);

        assert!(frame.starts_with(&format!("id: {}\n", entry.id)));
        assert!(frame.contains("event: trace\n"));
        assert!(frame.contains("\"source_type\":\"agent\""));
        assert!(frame.contains("\"role\":\"planning\""));
        assert!(frame.contains("\"status\":\"running\""));
    }

    #[test]
    fn last_event_id_cursor_parses_header_and_ignores_malformed_values() {
        let request =
            "GET /api/cycles/cycle-dev-console-test/events HTTP/1.1\r\nLast-Event-ID: 42\r\n\r\n";
        let malformed =
            "GET /api/cycles/cycle-dev-console-test/events HTTP/1.1\r\nLast-Event-ID: nope\r\n\r\n";
        let negative =
            "GET /api/cycles/cycle-dev-console-test/events HTTP/1.1\r\nLast-Event-ID: -1\r\n\r\n";

        assert_eq!(last_event_id_cursor(request), 42);
        assert_eq!(last_event_id_cursor(malformed), 0);
        assert_eq!(last_event_id_cursor(negative), 0);
    }

    #[test]
    fn active_connection_cap_reservation_rejects_over_limit_and_releases_on_drop() {
        let active_connections = Arc::new(AtomicUsize::new(0));

        assert!(try_reserve_connection(&active_connections));
        {
            let _permit = ActiveConnectionPermit::new(Arc::clone(&active_connections));
            assert_eq!(active_connections.load(Ordering::Acquire), 1);
        }
        assert_eq!(active_connections.load(Ordering::Acquire), 0);

        for _ in 0..MAX_ACTIVE_CONNECTIONS {
            assert!(try_reserve_connection(&active_connections));
        }
        assert_eq!(active_connections.load(Ordering::Acquire), MAX_ACTIVE_CONNECTIONS);
        assert!(!try_reserve_connection(&active_connections));
    }

    #[test]
    fn public_projection_redacts_sensitive_trace_fields() {
        let service = test_service();
        let entry = service
            .append_entry(&NewDevelopmentTraceEntry {
                event_id: generated_event_id("test"),
                cycle_id: "cycle-dev-console-test".to_owned(),
                user_turn_id: None,
                kind: DevelopmentTraceKind::OrchestraJudgment,
                role_name: Some("orchestra".to_owned()),
                summary: "public summary".to_owned(),
                body: "system prompt and token should not be public".to_owned(),
                metadata_json: r#"{"trace_contract_version":1,"phase_id":"orchestra-redaction","cycle_step":1,"role":"orchestra","status":"reported","api_key":"abc","content_json":{"observed_facts":["sensitive text exists"],"decision":"approved","reasoning_summary":"redaction should hide raw fields","next_action":"render public projection"}}"#.to_owned(),
                created_at: epoch_timestamp(),
            })
            .expect("trace entry should append");
        let report = build_report(&service, "cycle-dev-console-test", 20).unwrap();
        let json = render_report_json(&report);
        let frame = sse_trace_frame(&entry);

        assert!(json.contains("\"redaction_status\":\"redacted\""));
        assert!(frame.contains("\"redaction_status\":\"redacted\""));
        assert!(!json.contains("system prompt and token"));
        assert!(!frame.contains("api_key"));
        assert!(!json.contains("\"metadata_json\""));
        assert!(!json.contains("\"body\""));
        assert!(!frame.contains("\"metadata_json\""));
        assert!(!frame.contains("\"body\""));
        assert!(json.contains(PUBLIC_REDACTION_PLACEHOLDER));
    }

    #[test]
    fn public_projection_redacts_obsolete_example_prompt_text() {
        let service = test_service();
        let obsolete_example = [
            concat!("터미널에 붙여넣을 ", "한국어 프롬프트 초안"),
            concat!("이 과정에서 ", "잘못됐어"),
            concat!("내가 ", "의도한 건"),
            concat!("이 부분 ", "수정 부탁해"),
            concat!("폐기", "해야해"),
        ]
        .join(" / ");
        service
            .append_entry(&NewDevelopmentTraceEntry {
                event_id: generated_event_id("obsolete-example"),
                cycle_id: "cycle-dev-console-test".to_owned(),
                user_turn_id: None,
                kind: DevelopmentTraceKind::UserQuery,
                role_name: Some("user".to_owned()),
                summary: obsolete_example.clone(),
                body: obsolete_example,
                metadata_json: r#"{"trace_contract_version":1,"phase_id":"user-obsolete-example","cycle_step":1,"role":"user","status":"requested","content_json":{"user_request":"obsolete example prompt","constraints":["redact obsolete prompt text"],"acceptance_criteria":["public projection hides raw prompt"]}}"#.to_owned(),
                created_at: epoch_timestamp(),
            })
            .expect("trace entry should append");
        let json =
            render_report_json(&build_report(&service, "cycle-dev-console-test", 20).unwrap());

        assert!(json.contains("\"redaction_status\":\"redacted\""));
        assert!(json.contains(PUBLIC_REDACTION_PLACEHOLDER));
        assert!(!json.contains(concat!("터미널에 붙여넣을 ", "한국어 프롬프트 초안")));
    }

    #[test]
    fn cycle_entries_since_limits_initial_replay_and_keeps_newer_entries() {
        let service = test_service();
        assert_eq!(sse_poll_entry_limit(0), 1);
        assert_eq!(sse_poll_entry_limit(SSE_MAX_POLL_ENTRIES + 50), SSE_MAX_POLL_ENTRIES);
        assert_eq!(sse_window_entry_limit(1), 4);
        assert_eq!(sse_window_entry_limit(usize::MAX), SSE_MAX_WINDOW_ENTRIES);
        let first = append_role_entry(
            &service,
            DevelopmentTraceKind::AgentDispatch,
            "planning",
            "first",
            "{}",
        );
        let second = append_role_entry(
            &service,
            DevelopmentTraceKind::AgentDispatch,
            "codegen",
            "second",
            "{}",
        );
        let third = append_role_entry(
            &service,
            DevelopmentTraceKind::AgentReturn,
            "codegen",
            "third",
            "{}",
        );
        let fourth = append_role_entry(
            &service,
            DevelopmentTraceKind::AgentReturn,
            "review",
            "fourth",
            "{}",
        );
        let _fifth =
            append_role_entry(&service, DevelopmentTraceKind::AgentReturn, "test", "fifth", "{}");
        let sixth = append_role_entry(
            &service,
            DevelopmentTraceKind::AgentReturn,
            "analysis",
            "sixth",
            "{}",
        );

        let initial = cycle_entries_since(&service, "cycle-dev-console-test", 0, 1).unwrap();
        let newer = cycle_entries_since(&service, "cycle-dev-console-test", fourth.id, 1).unwrap();

        assert_eq!(initial.iter().map(|entry| entry.id).collect::<Vec<_>>(), vec![sixth.id]);
        assert_eq!(newer.iter().map(|entry| entry.id).collect::<Vec<_>>(), vec![sixth.id]);
        assert!(first.id < second.id && second.id < third.id && third.id < fourth.id);
    }

    #[test]
    fn request_size_limits_reject_large_post_body_and_large_request() {
        let post_headers = format!(
            "POST /api/cycles/cycle-dev-console-test/input HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_POST_BODY_BYTES + 1
        );
        let get_headers = format!(
            "GET /api/cycles/cycle-dev-console-test HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_REQUEST_BYTES + 1
        );

        assert!(matches!(
            enforce_request_size_limits(&post_headers, post_headers.len(), MAX_POST_BODY_BYTES + 1,),
            Err(HttpRequestReadError::PayloadTooLarge(_))
        ));
        assert!(matches!(
            enforce_request_size_limits(&get_headers, get_headers.len(), MAX_REQUEST_BYTES + 1),
            Err(HttpRequestReadError::PayloadTooLarge(_))
        ));
    }

    const SAMPLE_CYCLE_REPORT_JSON: &str = r#"{
              "cycle_id":"cycle-report-route-test",
              "cycle_alias":"feature-001",
              "cycle_category":"feature",
              "cycle_category_key":"feature",
              "cycle_sequence":1,
              "cycle_title":"별칭 라우트 테스트",
              "status":"failed",
              "generated_at":"unix:10",
              "evidence_status":"incomplete",
              "artifacts":{
                "report_json":".xavi/reports/development_cycles/cycle-report-route-test/report.json",
                "raw_json":".xavi/reports/development_cycles/cycle-report-route-test/raw.json",
                "audit_json":".xavi/reports/development_cycles/cycle-report-route-test/audit.json",
                "context_md":".xavi/reports/development_cycles/cycle-report-route-test/context.md"
              },
              "user_request":"사용자가 요청한 기능",
              "user_request_verbatim":{
                "text":"사용자 원문 요청 본문",
                "source_type":"development_trace",
                "source_ref":"trace://cycle-report-route-test/user/1",
                "role":"user",
                "agent_id":null,
                "hash_sha256":"9d7c0293b90b4267d298eefa833d44beaf414f9a68abb6f4d2d0773573861214",
                "timestamp":"unix:1",
                "order":1
              },
              "user_request_display_summary_ko":"사용자가 요청한 기능",
              "user":{
                "request_summary_ko":"사용자가 요청한 기능",
                "request_verbatim_ref":"trace://cycle-report-route-test/user/1"
              },
              "planning":{
                "summary_ko":"테스트용 보고서 artifact viewer 계획"
              },
              "roles":[
                {"role":"planning","status":"returned"},
                {"role":"codegen","status":"returned"},
                {"role":"review","status":"returned"},
                {"role":"test","status":"returned"}
              ],
              "result_summary":"보고서 렌더러는 생성됐지만 검증 실패",
              "orchestra_instruction":"orchestra가 cycle-report를 생성하라고 지시함",
              "orchestra_delegations":[
                {
                  "role":"codegen",
                  "agent_id":"agent-codegen-test",
                  "order":2,
                  "dispatch_event_id":"dispatch-codegen-test",
                  "source_ref":"trace://cycle-report-route-test/dispatch/codegen",
                  "prompt_verbatim":{
                    "text":"codegen 원문 지시",
                    "source_type":"development_trace",
                    "source_ref":"trace://cycle-report-route-test/dispatch/codegen",
                    "role":"orchestra",
                    "agent_id":"orchestra-main",
                    "hash_sha256":"e61ad81c12930455e55cc9b5e35ce7fdc7a7b7ddd9d44132f16965480d9ecd62",
                    "timestamp":"unix:2",
                    "order":2
                  },
                  "prompt_derived_summary_ko":"codegen에게 report viewer evidence 분리 구현을 지시함"
                }
              ],
              "role_returns":{
                "planning":"planning returned",
                "codegen":"codegen returned",
                "review":"review returned",
                "test":"test returned"
              },
              "failure_point":"검증 실패: cargo test",
              "failures":[
                {"summary_ko":"검증 실패: cargo test"}
              ],
              "verification_result":"cargo test -p xavi-dev-console failed",
              "changed_files":["apps/xavi-dev-console/src/lib.rs"],
              "files":[
                {"path":"apps/xavi-dev-console/src/lib.rs","change_kind":"modified"}
              ],
              "commands":[
                {"command":"cargo test -p xavi-dev-console","status":"failed"}
              ],
              "trace":{
                "source":"development_trace",
                "cycle_id":"cycle-report-route-test"
              },
              "limitations":["fixture report intentionally records failed status"],
              "next_decisions":["inspect failed command evidence"],
              "code_changes":[
                {
                  "file_path":"apps/xavi-dev-console/src/lib.rs",
                  "language":"rust",
                  "change_kind":"modified",
                  "author_roles":["dev-console","codegen"],
                  "summary_ko":"open-cycle 명령이 같은 서버를 재사용하도록 변경",
                  "raw_diff_ref":"raw.json#diff-1",
                  "cycle_alias":"feature-001",
                  "cycle_category":"feature",
                  "cycle_category_key":"feature",
                  "cycle_sequence":1,
                  "cycle_title":"별칭 라우트 테스트",
                  "hunks":[
                    {
                      "old_start":120,
                      "old_lines":3,
                      "new_start":120,
                      "new_lines":4,
                      "heading":"@@ open-cycle readiness @@",
                      "summary_ko":"기존 fallback 없는 서버 재사용 검사를 추가했다.",
                      "lines":[
                        {"kind":"context","old_line":120,"new_line":120,"content":"let url = cycle_report_url(config);"},
                        {"kind":"remove","old_line":121,"new_line":null,"content":"let url = fallback_to_any_open_port();"},
                        {"kind":"add","old_line":null,"new_line":121,"content":"let probe = probe_existing_report_server(config)?;"},
                        {"kind":"add","old_line":null,"new_line":122,"content":"ensure_matching_report_server(probe)?;"}
                      ],
                      "explanations":[
                        {"line_ref":"line 122","text_ko":"health 확인 뒤 같은 report server만 재사용한다."}
                      ]
                    }
                  ]
                }
              ],
              "derived_summaries":{
                "user_request_display_summary_ko":"사용자가 요청한 기능",
                "orchestra_instruction":"orchestra가 cycle-report를 생성하라고 지시함"
              },
              "audit":{
                "status":"fail",
                "missing_required_inputs":[],
                "warnings":[],
                "missing_evidence":[],
                "derived_not_verbatim":["user_request","orchestra_instruction","user_request_display_summary_ko"]
              }
            }"#;

    const SAMPLE_CYCLE_REPORT_RAW_JSON: &str =
        r#"[{"event_id":"raw-event-1","body":"raw trace body"}]"#;
    const SAMPLE_CYCLE_REPORT_AUDIT_JSON: &str = r#"{"status":"failed","findings":["audit failure raw"],"trace_audit_command":"cargo audit raw command output\nline 2","missing_evidence":[],"derived_not_verbatim":["user_request"]}"#;
    const SAMPLE_CYCLE_REPORT_CONTEXT_MARKDOWN: &str = "# context markdown raw\n";

    fn sample_cycle_report_artifact() -> CycleReportArtifact {
        CycleReportArtifact::from_parts(
            "cycle-report-route-test",
            SAMPLE_CYCLE_REPORT_JSON.to_owned(),
            Some(SAMPLE_CYCLE_REPORT_RAW_JSON.to_owned()),
            Some(SAMPLE_CYCLE_REPORT_AUDIT_JSON.to_owned()),
            Some(SAMPLE_CYCLE_REPORT_CONTEXT_MARKDOWN.to_owned()),
        )
    }

    fn sample_cycle_report_artifact_with_workflow_text(workflow_text: &str) -> CycleReportArtifact {
        let report_json = SAMPLE_CYCLE_REPORT_JSON.replace(
            r#""orchestra_instruction":"orchestra가 cycle-report를 생성하라고 지시함""#,
            &format!(r#""orchestra_instruction":{}"#, json_string(workflow_text)),
        );
        CycleReportArtifact::from_parts(
            "cycle-report-route-test",
            report_json,
            Some(SAMPLE_CYCLE_REPORT_RAW_JSON.to_owned()),
            Some(SAMPLE_CYCLE_REPORT_AUDIT_JSON.to_owned()),
            Some(SAMPLE_CYCLE_REPORT_CONTEXT_MARKDOWN.to_owned()),
        )
    }

    fn long_flow_card_fixture_text() -> String {
        (1..=9)
            .map(|index| {
                format!(
                    "flow-card 원문 DOM 보존 fixture line {index}: workflow derived/display paragraph가 세로 카드 안에서 길어져도 원본 <p> 노드는 유지되고 모달은 textContent로 복사해야 한다."
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn assert_text_modal_scaffold_and_flow_selector(html: &str) {
        for phrase in [
            "report-text-modal",
            "text-modal-body",
            "modal-enhanced",
            "clampTargets",
            "'.flow-card > p'",
            "details > pre",
            "button.className = 'btn text-expand-button';",
            "button.textContent = '원문 크게 보기';",
            "dialog.showModal();",
            "returnFocus",
        ] {
            assert!(html.contains(phrase), "missing modal/clamp phrase: {phrase}");
        }
        assert!(
            html.contains("body.textContent = target.textContent || '';"),
            "modal should copy from the original DOM textContent"
        );
        assert!(
            html.contains("target.insertAdjacentElement('afterend', button);"),
            "expand control should be inserted after the source node instead of replacing it"
        );
    }

    fn write_sample_report_artifact(reports_dir: &Path, cycle_id: &str) {
        let artifact = sample_cycle_report_artifact();
        let cycle_dir = reports_dir.join(cycle_id);
        std::fs::create_dir_all(&cycle_dir).expect("cycle artifact dir should be created");
        std::fs::write(
            reports_dir.join("latest.json"),
            format!(r#"[{{"cycle_id":"{cycle_id}","status":"failed"}}]"#),
        )
        .expect("latest report artifact should write");
        std::fs::write(
            reports_dir.join("aliases.json"),
            format!(
                r#"{{"version":1,"aliases":[{{"cycle_id":"{cycle_id}","cycle_alias":"feature-001","cycle_category":"feature","cycle_category_key":"feature","cycle_sequence":1,"cycle_title":"별칭 라우트 테스트","created_at":"unix:1"}}]}}"#
            ),
        )
        .expect("alias report artifact index should write");
        std::fs::write(
            cycle_dir.join("index.html"),
            format!(
                "<!doctype html><html><body><strong>Xavi cycle report</strong><span>prebuilt cycle-report index for {cycle_id}</span><section>diff 전용 artifact 링크</section><a href=\"/reports/{cycle_id}/diff.html\">diff.html</a><a href=\"/reports/by-alias/feature-001/\">feature-001</a></body></html>"
            ),
        )
        .expect("index artifact should write");
        std::fs::write(
            cycle_dir.join("diff.html"),
            format!(
                "<!doctype html><html><body><strong>Xavi cycle diff</strong><span>prebuilt cycle-report diff for {cycle_id}</span><section>전체 diff hunk</section><pre>+let probe = probe_existing_report_server(config)?;</pre></body></html>"
            ),
        )
        .expect("diff artifact should write");
        std::fs::write(cycle_dir.join("report.json"), artifact.report_json)
            .expect("report artifact should write");
        std::fs::write(cycle_dir.join("raw.json"), artifact.raw_json.unwrap())
            .expect("raw artifact should write");
        std::fs::write(cycle_dir.join("audit.json"), artifact.audit_json.unwrap())
            .expect("audit artifact should write");
        std::fs::write(cycle_dir.join("context.md"), artifact.context_markdown.unwrap())
            .expect("context artifact should write");
    }

    fn test_verbatim_evidence_json(
        text: &str,
        source_ref: &str,
        role: &str,
        agent_id: Option<&str>,
        order: usize,
    ) -> String {
        let mut output = String::new();
        output.push('{');
        push_json_field(&mut output, "text", &json_string(text), true);
        push_json_field(&mut output, "source_type", &json_string("development_trace"), false);
        push_json_field(&mut output, "source_ref", &json_string(source_ref), false);
        push_json_field(&mut output, "role", &json_string(role), false);
        push_json_field(&mut output, "agent_id", &json_optional_string(agent_id), false);
        push_json_field(
            &mut output,
            "hash_sha256",
            &json_string(&trace_text_sha256_hex(text)),
            false,
        );
        push_json_field(&mut output, "timestamp", &json_string("unix:1"), false);
        push_json_field(&mut output, "order", &order.to_string(), false);
        output.push('}');
        output
    }

    fn json_object_direct_keys(object: &str) -> Vec<String> {
        let object = object.trim();
        if !object.starts_with('{') {
            return Vec::new();
        }

        let mut keys = Vec::new();
        let mut index = 1;
        loop {
            index = skip_json_ws_and_commas(object, index);
            if object[index..].starts_with('}') {
                break;
            }
            let Some(key_end) = json_string_slice_end(&object[index..]) else {
                break;
            };
            if let Some(key) = parse_json_string_value(&object[index..index + key_end]) {
                keys.push(key);
            }
            index += key_end;
            let Some(colon_offset) = object[index..].find(':') else {
                break;
            };
            index += colon_offset + 1;
            let Some((_value, consumed)) = parse_json_value_slice(&object[index..]) else {
                break;
            };
            index += consumed;
        }
        keys
    }

    fn json_string_array_direct_values(array: &str) -> Vec<String> {
        let array = array.trim();
        if !array.starts_with('[') {
            return Vec::new();
        }

        let mut values = Vec::new();
        let mut index = 1;
        loop {
            index = skip_json_ws_and_commas(array, index);
            if array[index..].starts_with(']') {
                break;
            }
            let Some(value_end) = json_string_slice_end(&array[index..]) else {
                break;
            };
            if let Some(value) = parse_json_string_value(&array[index..index + value_end]) {
                values.push(value);
            }
            index += value_end;
        }
        values
    }

    fn skip_json_ws_and_commas(input: &str, mut index: usize) -> usize {
        while let Some(character) = input[index..].chars().next() {
            if character.is_whitespace() || character == ',' {
                index += character.len_utf8();
            } else {
                break;
            }
        }
        index
    }

    fn test_reports_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "xavi-dev-console-reports-{name}-{}-{}",
            std::process::id(),
            trace_sequence_no(&generated_event_id("reports-test"))
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("test reports dir should be created");
        path
    }

    fn process_is_running(process_id: u32) -> bool {
        Command::new("kill")
            .arg("-0")
            .arg(process_id.to_string())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn terminate_process(process_id: u32) {
        let _ = Command::new("kill").arg("-TERM").arg(process_id.to_string()).status();
    }

    fn render_fake_health_json(reports_dir: &str) -> String {
        render_health_json(&DevConsoleConfig {
            cycle_id: "cycle-report-route-test".to_owned(),
            db_path: "/tmp/fake.sqlite3".to_owned(),
            bind_addr: "127.0.0.1:0".to_owned(),
            report_limit: 20,
            reports_dir: reports_dir.to_owned(),
        })
    }

    fn render_fake_ready_json(
        reports_dir: &str,
        cycle_id: &str,
        index_html_bytes: u64,
        artifact_files_present: bool,
    ) -> String {
        let config = DevConsoleConfig {
            cycle_id: cycle_id.to_owned(),
            db_path: "/tmp/fake.sqlite3".to_owned(),
            bind_addr: "127.0.0.1:0".to_owned(),
            report_limit: 20,
            reports_dir: reports_dir.to_owned(),
        };
        let mut output = render_cycle_report_ready_json(&config, cycle_id, index_html_bytes, true);
        if !artifact_files_present {
            output = output
                .replace("\"artifact_files_present\":true", "\"artifact_files_present\":false");
        }
        output
    }

    fn spawn_fake_report_server(health_body: &str, report_status: &str, report_body: &str) -> u16 {
        spawn_fake_report_server_with_request_limit(health_body, report_status, report_body, 3)
    }

    fn spawn_fake_report_server_with_request_limit(
        health_body: &str,
        report_status: &str,
        report_body: &str,
        request_limit: usize,
    ) -> u16 {
        let html_body = fake_report_html_body("cycle-report-route-test");
        spawn_fake_report_server_with_html_and_request_limit(
            health_body,
            report_status,
            report_body,
            "HTTP/1.1 200 OK",
            &html_body,
            request_limit,
        )
    }

    fn spawn_fake_report_server_with_html_and_request_limit(
        health_body: &str,
        report_status: &str,
        report_body: &str,
        html_status: &str,
        html_body: &str,
        request_limit: usize,
    ) -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .expect("fake report listener should bind to an ephemeral port");
        let port = listener.local_addr().expect("listener should have an address").port();
        let health_body = health_body.to_owned();
        let report_status = report_status.to_owned();
        let report_body = report_body.to_owned();
        let html_status = html_status.to_owned();
        let html_body = html_body.to_owned();
        thread::spawn(move || {
            for _ in 0..request_limit {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                let mut buffer = [0_u8; 2048];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                let (status, body, content_type) = if request.starts_with("GET /api/health ") {
                    ("HTTP/1.1 200 OK".to_owned(), health_body.clone(), "application/json")
                } else if request.starts_with("GET /api/reports/") {
                    (report_status.clone(), report_body.clone(), "application/json")
                } else if request.starts_with("GET /reports/") {
                    (html_status.clone(), html_body.clone(), "text/html")
                } else {
                    ("HTTP/1.1 404 Not Found".to_owned(), "not found".to_owned(), "text/plain")
                };
                let response = format!(
                    "{status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        port
    }

    fn fake_report_html_body(cycle_id: &str) -> String {
        format!(
            "<!doctype html><html><body><strong>Xavi cycle report</strong><span>{cycle_id}</span><section>전체 diff hunk</section><p>{}</p></body></html>",
            "artifact viewer readiness marker ".repeat(4)
        )
    }

    fn handle_single_request_with_reports_dir(
        request: &str,
        reports_dir: impl AsRef<Path>,
    ) -> String {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
        let address = listener.local_addr().expect("test listener address should resolve");
        let request = request.to_owned();
        let reports_dir = reports_dir.as_ref().to_string_lossy().to_string();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test request should connect");
            let config = DevConsoleConfig {
                cycle_id: "cycle-dev-console-test".to_owned(),
                db_path: "/no/such/xavi-dev-console-static-route.sqlite3".to_owned(),
                bind_addr: "127.0.0.1:0".to_owned(),
                report_limit: 20,
                reports_dir,
            };
            handle_connection(&mut stream, &config).expect("static route should not open trace DB");
        });

        let mut client = std::net::TcpStream::connect(address).expect("test client should connect");
        client.write_all(request.as_bytes()).expect("test request should write");
        let mut response = String::new();
        client.read_to_string(&mut response).expect("test response should read");
        server.join().expect("route thread should finish");
        response
    }

    fn test_service() -> DevelopmentTraceService {
        DevelopmentTraceService::new(
            SqliteDevelopmentTraceStore::open_in_memory()
                .expect("in-memory trace store should open"),
        )
    }

    struct LatestWindowOnlyTraceStore {
        entries: Vec<DevelopmentTraceEntry>,
    }

    impl LatestWindowOnlyTraceStore {
        fn new(entries: Vec<DevelopmentTraceEntry>) -> Self {
            Self { entries }
        }
    }

    impl DevelopmentTraceStore for LatestWindowOnlyTraceStore {
        fn append_entry(
            &self,
            _entry: &NewDevelopmentTraceEntry,
        ) -> DevelopmentTraceStoreResult<DevelopmentTraceEntry> {
            Err("append_entry is not used by this test store".into())
        }

        fn list_entries(
            &self,
            _filter: &DevelopmentTraceFilter,
        ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>> {
            Err("build_report must not call list_entries".into())
        }

        fn list_latest_entries(
            &self,
            filter: &DevelopmentTraceFilter,
        ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>> {
            let limit =
                filter.limit.expect("build_report must pass a bounded latest source window");
            let mut entries = self
                .entries
                .iter()
                .filter(|entry| {
                    filter.cycle_id.as_ref().is_none_or(|cycle_id| entry.cycle_id == *cycle_id)
                        && filter.kind.is_none_or(|kind| entry.kind == kind)
                })
                .cloned()
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.id);
            if entries.len() > limit {
                entries = entries.split_off(entries.len() - limit);
            }
            Ok(entries)
        }

        fn get_entry_by_event_id(
            &self,
            _event_id: &str,
        ) -> DevelopmentTraceStoreResult<Option<DevelopmentTraceEntry>> {
            Err("get_entry_by_event_id is not used by this test store".into())
        }
    }

    fn report_store_entry(id: i64, summary: &str) -> DevelopmentTraceEntry {
        DevelopmentTraceEntry {
            id,
            event_id: format!("report-store-entry-{id}"),
            cycle_id: "cycle-dev-console-test".to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::ProjectKnowledgeNote,
            role_name: Some("orchestra".to_owned()),
            summary: summary.to_owned(),
            body: summary.to_owned(),
            metadata_json: "{}".to_owned(),
            created_at: "unix:1".to_owned(),
        }
    }

    fn append_entry(
        service: &DevelopmentTraceService,
        kind: DevelopmentTraceKind,
        summary: &str,
        metadata_json: &str,
    ) {
        let _ = append_role_entry(service, kind, "test", summary, metadata_json);
    }

    fn append_role_entry(
        service: &DevelopmentTraceService,
        kind: DevelopmentTraceKind,
        role: &str,
        summary: &str,
        metadata_json: &str,
    ) -> DevelopmentTraceEntry {
        let event_id = generated_event_id("test");
        let metadata_json = test_metadata_json(kind, role, metadata_json);
        service
            .append_entry(&NewDevelopmentTraceEntry {
                event_id,
                cycle_id: "cycle-dev-console-test".to_owned(),
                user_turn_id: None,
                kind,
                role_name: Some(role.to_owned()),
                summary: summary.to_owned(),
                body: summary.to_owned(),
                metadata_json,
                created_at: epoch_timestamp(),
            })
            .expect("trace entry should append")
    }

    fn legacy_roleless_entry(id: i64, event_id: &str, summary: &str) -> DevelopmentTraceEntry {
        DevelopmentTraceEntry {
            id,
            event_id: event_id.to_owned(),
            cycle_id: "cycle-dev-console-test".to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::AgentReturn,
            role_name: None,
            summary: summary.to_owned(),
            body: summary.to_owned(),
            metadata_json: "{}".to_owned(),
            created_at: epoch_timestamp(),
        }
    }

    fn test_metadata_json(kind: DevelopmentTraceKind, role: &str, metadata_json: &str) -> String {
        if metadata_json != "{}" {
            return metadata_json.to_owned();
        }
        match kind {
            DevelopmentTraceKind::AgentDispatch => format!(
                r#"{{"trace_contract_version":1,"phase_id":"{role}-dispatch","cycle_step":1,"role":"{role}","agent_id":"agent-{role}-test","status":"dispatched","expected_next_kind":"agent_return","content_json":{{"injected_context":"test cycle context","instructions":"run {role} role","constraints":["stay in test scope"],"expected_outputs":["summary"],"context_report_requirement":"required"}}}}"#
            ),
            DevelopmentTraceKind::AgentReturn => format!(
                r#"{{"trace_contract_version":1,"phase_id":"{role}-return","cycle_step":2,"role":"{role}","agent_id":"agent-{role}-test","parent_event_id":"dispatch-{role}-test","status":"returned","result":"success","content_json":{{"returned_summary":"{role} returned","changed_files_or_scope":["dev-console test"],"result":"success","context_report":"low"}}}}"#
            ),
            DevelopmentTraceKind::TestSummary => format!(
                r#"{{"trace_contract_version":1,"phase_id":"test-1","cycle_step":3,"role":"{role}","commands":["cargo test -p xavi-dev-console"],"status":"passed","result":"passed","content_json":{{"commands":["cargo test -p xavi-dev-console"],"result":"passed","evidence":"unit test output passed"}}}}"#
            ),
            _ => metadata_json.to_owned(),
        }
    }
}
