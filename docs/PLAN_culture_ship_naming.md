# Culture Ship Naming Plan

Devices in this fleet are named after Iain M. Banks's Culture ships. Each device gets:

- **Display name**: the full Culture ship name (e.g. `ROU Killing Time`)
- **Hostname**: a short slug derived from it (e.g. `killing-time`)
- **Bucket**: the hostname (e.g. `killing-time`) — makes InfluxDB backups from multiple devices mergeable
- **Diagnostics bucket**: `{hostname}-diag` (e.g. `killing-time-diag`)

The Rust crate `gsv-culture-ships` provides the canonical list. `src/backend/ships.rs` handles prefix stripping, slugification, truncation, and collision disambiguation.

---

## Phase 1: Plumbing (in progress)

Core ship name propagation: `.env` → Docker containers → Telegraf → Nginx.

### 1. Ship name in `.env` via `setup.sh`

- `setup.sh` writes `SHIP_DISPLAY_NAME` and `SHIP_HOSTNAME` to the on-device `.env`
- If not provided, defaults to generating a random ship name
- `INFLUXDB_BUCKET` defaults to `$SHIP_HOSTNAME` (fallback: `telegraf`)
- `INFLUXDB_DIAGNOSTICS_BUCKET` defaults to `${SHIP_HOSTNAME}-diag` (fallback: `telegraf-diag`)

### 2. Telegraf `[agent]` hostname in `format.rs`

- Replace `hostname = ""` with `hostname = "${SHIP_HOSTNAME}"`
- Telegraf's built-in env var substitution picks this up from `.env`
- Update tests and snapshots

### 3. Ship name fields in config structs

- `TelegrafConfig` gains `ship_display_name: Option<String>` and `ship_hostname: Option<String>`
- `DeploymentConfig` gains same fields
- Wire through from CLI → config → deployment

### 4. CLI `--ship-name` flag

- Add `--ship-name <name>` to `deploy provision` and `deploy setup` subcommands
- Accepts: explicit ship name (e.g. `"ROU Killing Time"`), `random`, or omitted (reads existing `.env` / auto-assigns)
- Uses `derive_hostname()` and `random_ship_name()` from `ships.rs`

### 5. `docker-compose.yml` container hostnames

- Add `hostname: ${SHIP_HOSTNAME:-iot2050}` to `telegraf` and `nginx-dashboard` services
- Pass `SHIP_DISPLAY_NAME` and `SHIP_HOSTNAME` as env vars to containers that need them

### 6. Nginx system-info and landing page

- `system-info.sh` reads `SHIP_DISPLAY_NAME` and `SHIP_HOSTNAME` from env, includes in JSON output
- `index.html` title becomes `{SHIP_DISPLAY_NAME} — Monitoring Dashboard`
- System info panel shows both display name and hostname

### 7. Bucket defaults in `setup.sh`

- `INFLUXDB_BUCKET=${SHIP_HOSTNAME:-telegraf}`
- `INFLUXDB_DIAGNOSTICS_BUCKET=${SHIP_HOSTNAME:-telegraf}-diag`
- Preserves backward compatibility when `SHIP_HOSTNAME` is absent

### 8. API service dashboard default bucket

- Replace hardcoded `"telegraf"` default with `env::var("INFLUXDB_BUCKET")` fallback

---

## Phase 2: Dashboard Templates (follow-up)

### 9. Grafana dashboard JSONs — template variables

- Add `bucket` and `diagBucket` Grafana template variables to system and OPC-UA dashboards
- Replace 12 hardcoded `"telegraf_diagnostics"` / `"telegraf"` refs with `v.diagBucket` / `v.bucket`
- Variables query InfluxDB for available buckets, defaulting to the provisioned ones
- In merged-InfluxDB scenario, user picks from dropdown

### 10. Rust dashboard generation (`dashboard.rs`)

- Add `diagnostics_bucket` field to `DashboardConfig`
- Generated dashboards use `v.bucket` / `v.diagBucket` instead of hardcoded bucket names
- Per-device: values resolve to ship-derived names at provisioning

### 11. Chronograf dashboards — envsubst at deploy

- Replace hardcoded bucket refs with `${INFLUXDB_BUCKET}` / `${INFLUXDB_DIAGNOSTICS_BUCKET}` placeholders
- `setup.sh` runs `envsubst` on Chronograf dashboard JSONs before placing them
- No runtime flexibility, but correct per-device names at deploy time

---

## Phase 3: Polish (follow-up)

### 12. Update tests and snapshots

- `format_test.rs`, `config_generator_test.rs`, integration test snapshots
- CI workflow fixtures (`INFLUXDB_DIAGNOSTICS_BUCKET` env vars)
- `test.sh` bucket defaults

---

## Naming Examples

| Display Name | Hostname | Bucket | Diag Bucket |
|---|---|---|---|
| ROU Killing Time | killing-time | killing-time | killing-time-diag |
| GCU Arbitrary | arbitrary | arbitrary | arbitrary-diag |
| GSV Unreliable Witness | unreliable-witness | unreliable-witness | unreliable-witness-diag |
| GCU Boo! | boo | boo | boo-diag |
| GSV Anticipation Of A New Lover's Arrival, The | anticipation-of-a-new | anticipation-of-a-new | anticipation-of-a-new-diag |