# Dashboard Generation Plan

## Overview

Add Grafana dashboard generation capability to the existing telegraf config generation workflow. When users generate telegraf configs from XML files, they can also generate Grafana dashboards to visualize the data.

## Current Architecture

### Telegraf Config Generation Flow

```
WebUI/GUI/CLI
    │
    ▼
ConfigGenerator::generate_config()
    │
    ├── Parse XML files (format.rs)
    │   └── Extract: group name (measurement), variable names, data types
    │
    ├── Generate telegraf.conf
    │   └── OPC UA inputs with measurement names
    │
    └── Write to disk
         │
         ▼
    User clicks "Send Config" button
         │
         ▼
    ConfigGenerator::send_config()
         │
         ▼
    ssh_utils::send_and_restart_telegraf()
         ├── SCP telegraf.conf to device
         └── Restart telegraf service
```

### Key Files

| File | Purpose |
|------|---------|
| `src/backend/mod.rs` | `ConfigGenerator` - main entry point |
| `src/backend/format.rs` | XML parsing, config formatting |
| `src/backend/ssh_utils.rs` | SSH/SCP operations, telegraf restart |
| `src/gui_controller.rs` | GUI methods calling ConfigGenerator |

## Proposed Architecture

### New Files

| File | Purpose |
|------|---------|
| `config/grafana/dashboard_template.json` | Default dashboard template with placeholders |
| `src/backend/dashboard.rs` | Dashboard generation logic |

### Dashboard Generation Flow

```
WebUI/GUI/CLI
    │
    ▼
ConfigGenerator::generate_config()
    │
    ├── Parse XML files (existing)
    │   └── Extract: group name (measurement), variable names, data types
    │
    ├── Generate telegraf.conf (existing)
    │
    ├── Generate grafana dashboard (NEW)
    │   │
    │   ▼
    │   dashboard::generate_dashboard()
    │       ├── Load template from config/grafana/dashboard_template.json
    │       ├── Replace placeholders with measurement names
    │       └── Write dashboard JSON to config folder
    │
    └── Write to disk

User clicks "Send Config" button (or new "Send Dashboard" button)
    │
    ▼
    ssh_utils::deploy_grafana_dashboard()
        ├── SCP dashboard JSON to /tmp/dashboard.json
        └── curl POST to Grafana API
```

## Template Design

### Template File Location

```
config/
├── grafana/
│   └── dashboard_template.json
├── xml/
│   ├── sample_db.xml
│   └── data_block_1.xml
└── telegraf.conf (generated)
```

### Template Placeholders

```json
{
  "dashboard": {
    "uid": "{{DASHBOARD_UID}}",
    "title": "{{DASHBOARD_TITLE}}",
    "panels": [
      {
        "title": "{{MEASUREMENT}}",
        "datasource": {
          "type": "influxdb",
          "uid": "{{DATASOURCE_UID}}"
        },
        "targets": [{
          "query": "from(bucket: \"{{BUCKET}}\")\n  |> range(start: v.timeRangeStart, stop: v.timeRangeStop)\n  |> filter(fn: (r) => r._measurement == \"{{MEASUREMENT}}\")\n  |> aggregateWindow(every: v.windowPeriod, fn: last, createEmpty: true)"
        }]
      }
    ]
  },
  "overwrite": true
}
```

### Placeholder Resolution

| Placeholder | Source | Example |
|-------------|--------|---------|
| `{{DASHBOARD_UID}}` | Sanitized measurement name | `sample_db` |
| `{{DASHBOARD_TITLE}}` | Measurement name (from UAObject DisplayName) | `Sample_DB` |
| `{{MEASUREMENT}}` | Measurement name from XML | `Sample_DB` |
| `{{BUCKET}}` | InfluxDB bucket (from env or config) | `telegraf` |
| `{{DATASOURCE_UID}}` | Grafana datasource UID or name | `InfluxDB` (by name) |

### Multi-Measurement Dashboards

For dashboards with multiple measurements (one XML file has multiple groups):

```json
{
  "dashboard": {
    "uid": "{{DASHBOARD_UID}}",
    "title": "{{DASHBOARD_TITLE}}",
    "panels": [
      {{#each measurements}}
      {
        "title": "{{this}}",
        "gridPos": { "h": 8, "w": 12, "x": 0, "y": {{@index}} },
        ...
      }
      {{/each}}
    ]
  }
}
```

Alternative: Use simple string replacement and build panels array programmatically in Rust.

## Implementation Phases

### Phase 1: Dashboard Generation Module

1. Create `src/backend/dashboard.rs`
2. Define structures:
   ```rust
   pub struct DashboardConfig {
       pub uid: String,
       pub title: String,
       pub measurements: Vec<String>,
       pub bucket: String,
       pub datasource_name: String,
   }
   
   pub fn generate_dashboard(config: &DashboardConfig, template_path: Option<&Path>) -> Result<String, TelegrafError>;
   ```
3. Implement template loading and placeholder replacement
4. Add unit tests

### Phase 2: Integrate with ConfigGenerator

1. Add `generate_grafana_dashboard()` method to `ConfigGenerator`
2. Load template from `config/grafana/dashboard_template.json`
3. Extract measurements from parsed XML data
4. Generate dashboard JSON alongside telegraf.conf
5. Write dashboard to config folder

### Phase 3: Deploy via SSH/API

1. Add `deploy_grafana_dashboard()` to `ssh_utils.rs`:
   ```rust
   pub fn deploy_grafana_dashboard(
       dashboard_json: &str,
       grafana_url: &str,
       username: &str,
       password: &str,
   ) -> Result<(), TelegrafError>
   ```
2. Use curl over SSH to call Grafana API (same pattern as backup/restore)
3. Add method to ConfigGenerator for high-level call

### Phase 4: GUI/CLI Integration

1. Add "Generate Dashboard" button in GUI (next to "Generate Config")
2. Add "Send Dashboard" button (next to "Send Config")
3. Add CLI commands:
   - `generate-dashboard --config-dir <path>`
   - `send-dashboard --iot-host <host>`
4. Optionally: Combined "Generate and Deploy" workflow

## Dashboard Template Default

### Single Measurement Panel Template

Based on `advgvch.json`, each panel has:

```json
{
  "datasource": {
    "type": "influxdb",
    "uid": "{{DATASOURCE_UID}}"
  },
  "fieldConfig": {
    "defaults": {
      "color": { "mode": "palette-classic" },
      "custom": {
        "drawStyle": "line",
        "lineInterpolation": "linear",
        "lineWidth": 1,
        "fillOpacity": 0
      },
      "thresholds": {
        "mode": "absolute",
        "steps": [
          { "color": "green", "value": 0 },
          { "color": "red", "value": 80 }
        ]
      }
    }
  },
  "gridPos": { "h": 8, "w": 24, "x": 0, "y": 0 },
  "options": {
    "legend": { "displayMode": "list", "placement": "bottom", "showLegend": true },
    "tooltip": { "mode": "single", "sort": "none" }
  },
  "targets": [{
    "datasource": { "type": "influxDB", "uid": "{{DATASOURCE_UID}}" },
    "query": "from(bucket: \"{{BUCKET}}\")\n  |> range(start: v.timeRangeStart, stop: v.timeRangeStop)\n  |> filter(fn: (r) => r._measurement == \"{{MEASUREMENT}}\")\n  |> aggregateWindow(every: v.windowPeriod, fn: last, createEmpty: true)",
    "refId": "A"
  }],
  "title": "{{MEASUREMENT}}",
  "type": "timeseries"
}
```

### Panel Grid Positioning

For multiple panels in one dashboard:
- Panel height: 8
- Panel width: 24 (full width) or 12 (half width)
- Y position increments by panel height
- Formula: `y = index * panel_height`

## Datasource Reference

### Options for Referencing InfluxDB Datasource

**Option A: By Name (Recommended)**
```json
"datasource": {
  "type": "influxdb",
  "name": "InfluxDB"
}
```
- Works if Grafana has datasource named "InfluxDB"
- More portable across environments

**Option B: By UID**
```json
"datasource": {
  "type": "influxdb",
  "uid": "P951FEA4DE68E13C5"
}
```
- Requires knowing the specific UID
- Less portable

**Option C: Grafana Variable**
```json
"datasource": {
  "type": "influxdb",
  "uid": "${DS_INFLUXDB}"
}
```
- Most flexible for templates
- Requires setting up dashboard variable

**Recommendation:** Use Option A (by name) with fallback to Option B if UID is known.

## Testing Plan

### Unit Tests

1. Template loading from file
2. Placeholder replacement
3. Multi-measurement panel generation
4. Edge cases: empty measurements, special characters in names

### Integration Tests (Manual)

1. Generate dashboard from sample XML
2. Verify JSON structure
3. Deploy to Grafana via API
4. Verify dashboard appears in Grafana UI
5. Verify data is displayed correctly

## Design Decisions

1. **Dashboard Generation Trigger**: Separate button - user explicitly clicks "Generate Dashboard" after config generation
2. **Measurement Extraction**: Measurements returned from config generation API (modify `generate_config` response to include measurements list)
3. **Template Customization**: Use default template only - no user upload capability for now
4. **Panel Type**: Always use `timeseries` for simplicity
5. **Multiple XML Files**: One combined dashboard with all measurements
6. **Dashboard Updates**: Always update existing (use measurement name as UID for consistency)

## Implementation Progress

### Phase 1: Dashboard Module ✅ COMPLETE
- [x] Created `src/backend/dashboard.rs`
- [x] Created `config/grafana/dashboard_template.json`
- [x] Implemented `DashboardConfig` struct
- [x] Implemented `generate_dashboard()`, `load_template()`, `fill_template()`, `sanitize_uid()`
- [x] Unit tests passing (9 tests)

### Phase 2: ConfigGenerator Integration ✅ COMPLETE
- [x] Added `generate_grafana_dashboard()` method to `ConfigGenerator`
- [x] Added `deploy_grafana_dashboard()` function to `ssh_utils.rs`
- [x] Exports in `mod.rs`: `DashboardConfig`, `generate_dashboard`, `sanitize_uid`
- [x] All 205 tests passing

### Phase 3: API Integration 🔄 IN PROGRESS
- [ ] Modify `config.rs::generate_config` to return measurements list
- [ ] Create `docker/api-service/src/dashboard.rs` module
- [ ] Add routes in `lib.rs`:
  - `/api/dashboard/generate` - Generate dashboard from measurements
  - `/api/dashboard/deploy` - Deploy to Grafana
- [ ] Measurements returned from config API response

### Phase 4: WebUI Integration 📋 PENDING
- [ ] Add "Generate Dashboard" button in Step 3
- [ ] Add dashboard preview modal
- [ ] Add "Deploy Dashboard" button
- [ ] JavaScript API calls for dashboard flow

## Timeline

| Phase | Task | Status |
|-------|------|--------|
| 1 | Dashboard module + template | ✅ Complete |
| 2 | ConfigGenerator integration | ✅ Complete |
| 3 | API integration | 🔄 In Progress |
| 4 | WebUI integration | 📋 Pending |
| - | Testing | 📋 Pending |