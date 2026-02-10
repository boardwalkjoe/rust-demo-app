use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, net::SocketAddr, time::Instant};
use sysinfo::System;

static mut START_TIME: Option<Instant> = None;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    uptime_seconds: u64,
    timestamp: String,
}

#[derive(Serialize)]
struct ContainerInfo {
    hostname: String,
    user_id: u32,
    group_id: u32,
    environment: HashMap<String, String>,
    system: SystemInfo,
}

#[derive(Serialize)]
struct SystemInfo {
    os_name: String,
    os_version: String,
    kernel_version: String,
    cpu_count: usize,
    total_memory_mb: u64,
    used_memory_mb: u64,
}

#[derive(Serialize)]
struct FibResult {
    n: u64,
    result: u64,
    computation_ms: f64,
}

#[derive(Deserialize)]
struct FibQuery {
    n: Option<u64>,
}

fn uptime_secs() -> u64 {
    unsafe { START_TIME.map(|s| s.elapsed().as_secs()).unwrap_or(0) }
}

// ─── Handlers ───────────────────────────────────────────────

async fn landing_page() -> Html<String> {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>🦀 Rust on OpenShift</title>
<style>
  :root {{ --accent: #e44d26; --bg: #0d1117; --card: #161b22; --text: #c9d1d9; --dim: #8b949e; }}
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ font-family: 'Segoe UI', system-ui, sans-serif; background: var(--bg); color: var(--text); min-height: 100vh; display: flex; align-items: center; justify-content: center; }}
  .container {{ max-width: 720px; width: 90%; padding: 2rem; }}
  h1 {{ font-size: 2.5rem; margin-bottom: 0.25rem; }}
  h1 span {{ color: var(--accent); }}
  .subtitle {{ color: var(--dim); font-size: 1.1rem; margin-bottom: 2rem; }}
  .hostname {{ background: var(--card); border: 1px solid #30363d; border-radius: 8px; padding: 1rem 1.25rem; margin-bottom: 2rem; font-family: monospace; font-size: 1rem; }}
  .hostname strong {{ color: var(--accent); }}
  .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin-bottom: 2rem; }}
  .card {{ background: var(--card); border: 1px solid #30363d; border-radius: 8px; padding: 1.25rem; transition: border-color 0.2s; }}
  .card:hover {{ border-color: var(--accent); }}
  .card h3 {{ font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--dim); margin-bottom: 0.5rem; }}
  .card a {{ color: var(--accent); text-decoration: none; font-family: monospace; font-size: 1.05rem; }}
  .card a:hover {{ text-decoration: underline; }}
  .card p {{ color: var(--dim); font-size: 0.85rem; margin-top: 0.4rem; }}
  .footer {{ color: var(--dim); font-size: 0.8rem; text-align: center; margin-top: 1rem; }}
  .uptime {{ animation: pulse 2s infinite; display: inline-block; }}
  @keyframes pulse {{ 0%, 100% {{ opacity: 1; }} 50% {{ opacity: 0.5; }} }}
</style>
</head>
<body>
<div class="container">
  <h1>🦀 <span>Rust</span> on OpenShift</h1>
  <p class="subtitle">A lightweight container demo &mdash; running and ready.</p>

  <div class="hostname">
    <strong>Pod:</strong> {hostname} &nbsp;|&nbsp;
    <strong>Uptime:</strong> <span class="uptime">{uptime}s</span> &nbsp;|&nbsp;
    <strong>UID:</strong> {uid}
  </div>

  <div class="grid">
    <div class="card">
      <h3>🩺 Health</h3>
      <a href="/healthz">/healthz</a>
      <p>Liveness probe endpoint</p>
    </div>
    <div class="card">
      <h3>✅ Ready</h3>
      <a href="/readyz">/readyz</a>
      <p>Readiness probe endpoint</p>
    </div>
    <div class="card">
      <h3>🔍 Container Info</h3>
      <a href="/info">/info</a>
      <p>Runtime environment &amp; system details</p>
    </div>
    <div class="card">
      <h3>🧮 Fibonacci</h3>
      <a href="/fib?n=40">/fib?n=40</a>
      <p>CPU stress test via naive recursion</p>
    </div>
    <div class="card">
      <h3>💥 Crash Test</h3>
      <a href="/crash">/crash</a>
      <p>Trigger panic &mdash; test restart policy</p>
    </div>
    <div class="card">
      <h3>📊 Metrics</h3>
      <a href="/metrics">/metrics</a>
      <p>Prometheus-style metrics</p>
    </div>
  </div>

  <p class="footer">Built with Axum &bull; Compiled with musl &bull; Running from scratch</p>
</div>
</body>
</html>"##,
        hostname = hostname,
        uptime = uptime_secs(),
        uid = unsafe { libc::getuid() },
    );
    Html(html)
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        uptime_seconds: uptime_secs(),
        timestamp: Utc::now().to_rfc3339(),
    })
}

async fn readyz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ready".into(),
        uptime_seconds: uptime_secs(),
        timestamp: Utc::now().to_rfc3339(),
    })
}

async fn info() -> Json<ContainerInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let environment: HashMap<String, String> = env::vars()
        .filter(|(k, _)| {
            // Surface interesting OpenShift / K8s env vars, hide secrets
            k.starts_with("KUBERNETES_")
                || k.starts_with("OPENSHIFT_")
                || k.starts_with("POD_")
                || k == "HOSTNAME"
                || k == "HOME"
                || k == "PATH"
                || k == "RUST_LOG"
                || k == "APP_VERSION"
        })
        .collect();

    Json(ContainerInfo {
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".into()),
        user_id: unsafe { libc::getuid() },
        group_id: unsafe { libc::getgid() },
        environment,
        system: SystemInfo {
            os_name: System::name().unwrap_or_default(),
            os_version: System::os_version().unwrap_or_default(),
            kernel_version: System::kernel_version().unwrap_or_default(),
            cpu_count: sys.cpus().len(),
            total_memory_mb: sys.total_memory() / 1024 / 1024,
            used_memory_mb: sys.used_memory() / 1024 / 1024,
        },
    })
}

fn fib(n: u64) -> u64 {
    if n <= 1 { return n; }
    fib(n - 1) + fib(n - 2)
}

async fn fibonacci(Query(params): Query<FibQuery>) -> impl IntoResponse {
    let n = params.n.unwrap_or(10).min(45); // Cap at 45 to avoid heat death
    let start = Instant::now();
    let result = fib(n);
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Json(FibResult {
        n,
        result,
        computation_ms: elapsed,
    })
}

async fn crash() -> impl IntoResponse {
    // Give a response before panicking
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        panic!("💥 Intentional crash to test OpenShift restart policy!");
    });
    (
        StatusCode::OK,
        "Crashing in 100ms... watch your pod restart! 💥",
    )
}

async fn metrics() -> impl IntoResponse {
    let mut sys = System::new_all();
    sys.refresh_all();

    let uptime = uptime_secs();
    let mem_total = sys.total_memory();
    let mem_used = sys.used_memory();

    let body = format!(
        r#"# HELP app_uptime_seconds Time since application started
# TYPE app_uptime_seconds gauge
app_uptime_seconds {}

# HELP app_memory_total_bytes Total system memory
# TYPE app_memory_total_bytes gauge
app_memory_total_bytes {}

# HELP app_memory_used_bytes Used system memory
# TYPE app_memory_used_bytes gauge
app_memory_used_bytes {}

# HELP app_cpu_count Number of CPUs available
# TYPE app_cpu_count gauge
app_cpu_count {}
"#,
        uptime,
        mem_total,
        mem_used,
        sys.cpus().len(),
    );

    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        body,
    )
}

// ─── Main ───────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    unsafe { START_TIME = Some(Instant::now()); }

    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap_or(8080);

    let app = Router::new()
        .route("/", get(landing_page))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/info", get(info))
        .route("/fib", get(fibonacci))
        .route("/crash", get(crash))
        .route("/metrics", get(metrics));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🦀 Rust demo listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
