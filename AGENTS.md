# AGENTS.md

## Project purpose

This repo builds an open-source industrial monitoring stack for Siemens/OPC-UA workflows.

Core flow:

TIA Portal XML exports -> Rust config generator / OPC-UA browser -> Telegraf config -> InfluxDB -> Grafana dashboards.

The project supports:
- Native GUI: `src/bin/gui.rs`
- TUI: `src/bin/tui.rs`
- CLI: `src/bin/cli.rs`
- Docker monitoring stack under `docker/`
- API service under `docker/api-service/`
- Deployment scripts under `docker/scripts/`

## Architecture map

Important Rust areas:
- `src/lib.rs`: core config types, including `TelegrafConfig`
- `src/backend/mod.rs`: config generation orchestration
- `src/backend/format.rs`: XML parsing and Telegraf config output
- `src/backend/opcua_poller.rs`: OPC-UA browsing/polling
- `src/backend/deployment.rs`: Docker stack deployment logic
- `src/backend/ssh_utils.rs`: SSH/device operations
- `src/worker.rs`: background task execution
- `src/error.rs`: error types

Docker stack:
- `docker/docker-compose.yml`: main stack
- `docker/config/telegraf/`: Telegraf templates/config
- `docker/config/grafana/`: dashboards/datasources
- `docker/config/prometheus/`: Prometheus config
- `docker/config/nginx/`: reverse proxy and landing page
- `docker/scripts/`: provisioning, deploy, start/stop, tests

Docs:
- `README.md`: user-facing overview
- `PROJECT_OVERVIEW.md`: architecture
- `docs/CONTAINER_SETUP.md`: container deployment
- `docs/WEBUI_CONFIG_GENERATOR.md`: Web UI config generation
- `docker/README.md`: Docker stack
- `docker/TESTING.md`: Docker testing

## Before editing

Prefer small, focused changes.

Do not rewrite large modules unless asked.

Do not change deployment defaults, credentials, ports, service names, or Docker volume names casually.

Preserve support for:
- x86_64 and ARM64
- Linux industrial edge devices / Siemens IOT2050
- GUI, TUI, CLI, and Docker workflows
- Headless SSH environments

Treat industrial/PLC behavior conservatively. Avoid unsafe assumptions about live control, writes to PLCs, or production devices.

## Build commands

Standard Rust build:

```bash
cargo build
cargo build --release
```

Nix development environment:

```bash
nix develop
```

Run binaries:

```bash
cargo run --bin sie_generate_config
cargo run --bin sie_generate_config_gui
cargo run --bin sie_generate_config_tui
cargo run --bin opcua_test_server
cargo run --bin opcua_client_test
```

## Test commands

Run Rust tests:

```bash
cargo test
```

Run coverage if available:

```bash
./run_coverage.sh
```

Docker stack local smoke test:

```bash
cd docker
./scripts/setup.sh
./scripts/start.sh
./scripts/test.sh
```

Script/integration tests:

```bash
cd docker
./scripts/test-scripts.sh
./scripts/test-integration.sh
```

## Formatting and quality

Before finishing a code change, run where applicable:

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

For Docker/script changes, also run relevant shell tests in docker/scripts/.

## Coding conventions

Rust:

- Prefer explicit errors using existing error types in src/error.rs.
- Keep GUI/TUI/CLI logic thin; put reusable logic in backend/library modules.
- Avoid blocking UI threads; use worker.rs style background work where appropriate.
- Keep OPC-UA browsing lazy where possible.
- Keep user-facing errors understandable.

Shell:

- Keep scripts idempotent where possible.
- Avoid hardcoded credentials.
- Preserve remote deployment behavior.

Docker:

- Do not break reverse proxy paths.
- Keep service names stable unless the task explicitly requires migration.
- Maintain persistent volumes.

## Common task routing

If the task mentions XML parsing, config output, or Telegraf format:

- Start in src/backend/format.rs
- Check tests and sample XML under tests/

If the task mentions OPC-UA browsing, polling, nodes, namespace, or tags:

- Start in src/backend/opcua_poller.rs
- Check GUI/TUI callers

If the task mentions CLI commands:

- Start in src/bin/cli.rs

If the task mentions GUI:

- Start in src/bin/gui.rs

If the task mentions TUI/headless/SSH:

- Start in src/bin/tui.rs
- Also inspect deployment and SSH modules

If the task mentions deploy/provision/start/stop/status:

- Start in src/backend/deployment.rs
- Then inspect docker/scripts/

If the task mentions Grafana/InfluxDB/Prometheus/Nginx:

- Start in docker/config/
- Then inspect docker/docker-compose.yml

If the task mentions API service/container control:

- Start in docker/api-service/

## PR/checklist expectations

Before proposing completion:

- Say what files changed
- Say which tests were run
- Mention any tests not run
- Mention risks around deployment, credentials, ports, or production devices