# 🦀 Rust on OpenShift — Container Demo

A lightweight Rust web service built with [Axum](https://github.com/tokio-rs/axum), designed to test and validate container deployments on OpenShift. The application compiles to a single static binary using musl, runs from a `scratch` image, and typically produces a final container image under 15 MB.

---

## Endpoints

### `GET /` — Landing Page

A dark-themed dashboard showing real-time pod identity, uptime, and quick links to every endpoint. Useful for confirming your Route and Service are wired up correctly.

**Displays:**
- Pod hostname (maps to the Kubernetes pod name)
- Current UID assigned by OpenShift's random UID allocation
- Application uptime in seconds

---

### `GET /healthz` — Liveness Probe

Returns a JSON health check response. Wire this up to your container's `livenessProbe` so OpenShift knows when to restart an unhealthy pod.

**Response:**
```json
{
  "status": "ok",
  "uptime_seconds": 142,
  "timestamp": "2025-02-09T18:30:00.000Z"
}
```

---

### `GET /readyz` — Readiness Probe

Returns a JSON readiness check response. Use this as your `readinessProbe` to control when the pod starts receiving traffic from the Service.

**Response:**
```json
{
  "status": "ready",
  "uptime_seconds": 142,
  "timestamp": "2025-02-09T18:30:00.000Z"
}
```

---

### `GET /info` — Container Introspection

Returns detailed information about the running container environment. This is the most useful endpoint for validating your OpenShift deployment configuration.

**Response:**
```json
{
  "hostname": "rustdemo-7b4f5c6d8-x9k2m",
  "user_id": 1000680000,
  "group_id": 0,
  "environment": {
    "KUBERNETES_SERVICE_HOST": "172.30.0.1",
    "KUBERNETES_SERVICE_PORT": "443",
    "POD_NAME": "rustdemo-7b4f5c6d8-x9k2m",
    "POD_NAMESPACE": "my-project",
    "HOSTNAME": "rustdemo-7b4f5c6d8-x9k2m",
    "APP_VERSION": "0.1.0"
  },
  "system": {
    "os_name": "Linux",
    "os_version": "",
    "kernel_version": "5.14.0-284.el9.x86_64",
    "cpu_count": 4,
    "total_memory_mb": 16384,
    "used_memory_mb": 8012
  }
}
```

**What to look for:**
- `user_id` confirms OpenShift's random UID assignment is working (should be a high number, not 1001)
- `group_id` should be `0` (root group) per OpenShift convention
- `environment` surfaces Kubernetes and OpenShift injected variables
- `system` shows the node's resource profile visible to the container

---

### `GET /fib?n=<number>` — CPU Stress Test

Computes the Nth Fibonacci number using naive recursion. This is intentionally inefficient, making it a simple way to stress-test CPU limits and observe throttling behavior under OpenShift resource constraints.

**Parameters:**
| Parameter | Type | Default | Max | Description |
|-----------|------|---------|-----|-------------|
| `n` | integer | 10 | 45 | Fibonacci sequence index to compute |

**Response:**
```json
{
  "n": 40,
  "result": 102334155,
  "computation_ms": 512.34
}
```

**Suggested tests:**
- `/fib?n=10` — instant response, sanity check
- `/fib?n=35` — ~50ms, light load
- `/fib?n=40` — ~500ms, moderate load
- `/fib?n=45` — ~5s, heavy load (capped maximum)

Use this alongside `oc adm top pod` to watch CPU consumption climb in real time.

---

### `GET /crash` — Crash Test

Triggers an intentional panic after a 100ms delay, causing the container process to exit. Use this to validate that OpenShift's restart policy (`Always` by default) correctly recovers the pod.

**Response (before crash):**
```
Crashing in 100ms... watch your pod restart! 💥
```

**What to observe:**
- Run `oc get pods -w` in another terminal to watch the restart count increment
- Check `oc describe pod <name>` to see the `CrashLoopBackOff` state if hit repeatedly
- Verify your liveness probe detects the restart and the pod returns to `Running`

---

### `GET /metrics` — Prometheus Metrics

Returns metrics in Prometheus exposition format. If you have the OpenShift monitoring stack or a Prometheus Operator deployed, you can scrape this endpoint with a `ServiceMonitor`.

**Response:**
```
# HELP app_uptime_seconds Time since application started
# TYPE app_uptime_seconds gauge
app_uptime_seconds 142

# HELP app_memory_total_bytes Total system memory
# TYPE app_memory_total_bytes gauge
app_memory_total_bytes 17179869184

# HELP app_memory_used_bytes Used system memory
# TYPE app_memory_used_bytes gauge
app_memory_used_bytes 8401207296

# HELP app_cpu_count Number of CPUs available
# TYPE app_cpu_count gauge
app_cpu_count 4
```

---

## Quick Start

```bash
# Extract and enter the project
tar xzf openshift-rustdemo.tar.gz
cd openshift-rustdemo

# Generate the lock file
cargo generate-lockfile

# Build the container image
podman build -t openshift-rustdemo .

# Run locally
podman run -p 8080:8080 openshift-rustdemo

# Open http://localhost:8080 in your browser
```

## Deploy to OpenShift

```bash
# Log in and create a project
oc login <cluster-url>
oc new-project rustdemo

# Push the image to a registry OpenShift can reach
podman tag openshift-rustdemo quay.io/<you>/openshift-rustdemo:latest
podman push quay.io/<you>/openshift-rustdemo:latest

# Update the image reference in the manifest
sed -i 's|image:.*|image: quay.io/<you>/openshift-rustdemo:latest|' openshift-deploy.yaml

# Deploy
oc apply -f openshift-deploy.yaml

# Watch it come up
oc get pods -w
oc get route rustdemo
```

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `PORT` | `8080` | Listen port for the HTTP server |
| `APP_VERSION` | — | Shown in `/info` output |
| `POD_NAME` | — | Injected via Downward API in the deployment manifest |
| `POD_NAMESPACE` | — | Injected via Downward API in the deployment manifest |

## Resource Profile

The included deployment manifest requests minimal resources appropriate for this lightweight binary:

| | Request | Limit |
|---|---------|-------|
| **CPU** | 50m | 500m |
| **Memory** | 16Mi | 64Mi |

## Project Structure

```
openshift-rustdemo/
├── Cargo.toml            # Dependencies and release optimizations
├── Dockerfile            # Multi-stage build: rust → scratch
├── .dockerignore         # Keep build context clean
├── openshift-deploy.yaml # Deployment, Service, and Route
└── src/
    └── main.rs           # Application source
```
