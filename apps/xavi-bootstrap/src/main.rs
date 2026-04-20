//! Bootstrap binary for the Xavi workspace.

use xavi_application::services::health_check_service::HealthCheckService;
use xavi_infrastructure::health::in_memory_status_reader::InMemoryHealthStatusReader;

fn main() {
    let service = HealthCheckService::new(InMemoryHealthStatusReader::healthy());
    let report = service.execute();

    println!("xavi-bootstrap initialized: status={:?}, message={}", report.status, report.message);
}
