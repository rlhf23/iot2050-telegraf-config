# Per-Device Theme Assignment Plan

## Goal

Make it immediately obvious which device you're looking at by giving each deployed instance a distinct theme/color palette. The theme is configured per-device via `.env`, so a test device and a production device sitting side-by-side on a browser look unmistakably different.

## Current state

- `theme.js` defines 6 palettes and injects a `<select>` dropdown into each page
- Theme choice persists in `localStorage` per browser
- All device instances default to **Indigo Purple** unless the user manually switches
- No server-side theme configuration exists

## Proposed architecture

### 1. Add `THEME` to `.env`

```
# In docker/.env or the deployment .env
THEME=indigo-purple
```

Valid values correspond to the keys in `theme.js`:

| Key              | Display name       |
|------------------|--------------------|
| `indigo-purple`  | Indigo Purple      |
| `ocean-teal`     | Ocean Teal         |
| `industrial-orange` | Industrial Orange |
| `slate-steel`    | Slate Steel        |
| `dark-mode`      | Dark Mode          |
| `aquatic-sci-fi` | Aquatic Sci-Fi     |

If `THEME` is unset or invalid, fall back to `indigo-purple`.

### 2. Expose the configured theme via the API

Add a lightweight endpoint to the existing API service:

```
GET /api/theme
```

Returns:

```json
{ "theme": "ocean-teal" }
```

This is a read-only endpoint. The theme is not changeable via API — it's a deployment-time setting from `.env`. The API reads it from an environment variable injected by docker-compose.

### 3. Consume the server theme on page load

In `theme.js`, the startup sequence becomes:

1. On `DOMContentLoaded`, fetch `/api/theme`
2. If the response gives a valid theme key, store it as `serverTheme`
3. Check `localStorage` for a user-controlled override (`monitoring-theme`)
4. Apply the theme in this priority order:
   - **User override** (`localStorage`) — if the user has manually picked a theme, respect that
   - **Server theme** — the `.env`-configured default for this device
   - **Built-in default** — `indigo-purple`

This way, a fresh browser on a new device immediately sees the operator-chosen theme, but a returning user who has manually switched themes keeps their preference.

5. Render the dropdown as today, but mark the server theme with a subtle label like "(device default)" or highlight it differently, so operators know what the machine is supposed to look like.

### 4. Docker compose plumbing

In `docker/docker-compose.yml`, pass the env var through to the API service:

```yaml
api-service:
  environment:
    - THEME=${THEME:-indigo-purple}
```

No changes needed for nginx — it doesn't need to know the theme.

### 5. Centralized theme registry

`theme.js` remains the single source of truth for the theme list. To make the valid keys discoverable outside JS (e.g. for `.env` comments, documentation, or shell scripts that validate `.env`), mirror the list in a small manifest file:

**`docker/config/nginx/html/themes.json`**

```json
{
  "default": "indigo-purple",
  "themes": [
    { "key": "indigo-purple", "name": "Indigo Purple" },
    { "key": "ocean-teal", "name": "Ocean Teal" },
    { "key": "industrial-orange", "name": "Industrial Orange" },
    { "key": "slate-steel", "name": "Slate Steel" },
    { "key": "dark-mode", "name": "Dark Mode" },
    { "key": "aquatic-sci-fi", "name": "Aquatic Sci-Fi" }
  ]
}
```

`theme.js` loads this file at startup to populate the dropdown instead of hardcoding. This way adding a new theme means editing only `themes.json` — the JS picks up new options automatically. (If the fetch fails, fall back to the built-in list.)

### 6. Random theme assignment (optional)

For quick test deployments where you don't care which specific theme you get, set:

```
THEME=random
```

The API service or start script resolves `random` to one of the valid theme keys before returning it. This makes spin-ups of throwaway test devices visually distinct with zero config. Resolution should be deterministic per device (e.g. seeded from hostname) so redeploying the same device keeps its random theme stable.

## Files to create/modify

| File | Action |
|------|--------|
| `docker/.env` | Add `THEME=indigo-purple` |
| `docker/docker-compose.yml` | Pass `THEME` env to API service |
| `docker/api-service/` (Python) | Add `GET /api/theme` endpoint |
| `docker/config/nginx/html/themes.json` | New — theme registry |
| `docker/config/nginx/html/theme.js` | Load from themes.json; respect server theme; add random resolution |
| `docker/config/nginx/html/index.html` | No change (already loads theme.js) |
| `docker/config/nginx/html/control.html` | No change |
| `docker/config/nginx/html/config-upload.html` | No change |

## Not in scope

- Per-user theme persistence across different browsers (localStorage is per-browser, this is fine)
- Theme auto-detection based on device hostname (manual `.env` is sufficient for now)
- Dark-mode OS-level detection (could be added later)
- More than 6 themes (easy to add via themes.json later)