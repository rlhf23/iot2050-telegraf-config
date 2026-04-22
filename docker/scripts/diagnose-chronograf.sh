#!/bin/bash
# Diagnose Chronograf dashboard import/export issues
# Usage: ./diagnose-chronograf.sh [chronograf_url]
#
# This script:
# 1. Lists all sources registered in Chronograf
# 2. Lists all dashboards and inspects their content
# 3. Exports each dashboard and shows the difference vs the import file
# 4. Tests importing a minimal dashboard to see what Chronograf returns

CHRONOGRAF_URL="${1:-http://localhost:8888}"
API="${CHRONOGRAF_URL}/chronograf/v1"

echo "=== Chronograf Dashboard Diagnostics ==="
echo ""

echo "--- 1. Checking Chronograf health ---"
HEALTH=$(curl -sf "${CHRONOGRAF_URL}/health" 2>/dev/null || echo "UNREACHABLE")
echo "Health: ${HEALTH}"
if [ "${HEALTH}" = "UNREACHABLE" ]; then
    echo "ERROR: Chronograf is not reachable at ${CHRONOGRAF_URL}"
    exit 1
fi
echo ""

echo "--- 2. Registered sources ---"
SOURCES=$(curl -sf "${API}/sources" 2>/dev/null || echo '{}')
echo "${SOURCES}" | python3 -m json.tool 2>/dev/null || echo "${SOURCES}"
echo ""

echo "--- 3. Existing dashboards ---"
DASHBOARDS=$(curl -sf "${API}/dashboards" 2>/dev/null || echo '{}')
DASHBOARD_COUNT=$(echo "${DASHBOARDS}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('dashboards',[])))" 2>/dev/null || echo "0")
echo "Found ${DASHBOARD_COUNT} dashboard(s)"
echo ""

echo "${DASHBOARDS}" | python3 -c "
import sys, json
data = json.load(sys.stdin)
dashboards = data.get('dashboards', [])
for db in dashboards:
    dbid = db.get('id', 'NO_ID')
    name = db.get('name', 'NO_NAME')
    cells = db.get('cells', [])
    num_cells = len(cells)
    cell_names = [c.get('name', 'unnamed') for c in cells]
    print(f'  Dashboard id={dbid}: name=\"{name}\", cells={num_cells} ({cell_names})')
" 2>/dev/null || echo "${DASHBOARDS}" | head -50
echo ""

echo "--- 4. Exporting each dashboard (full JSON) ---"
echo "${DASHBOARDS}" | python3 -c "
import sys, json
data = json.load(sys.stdin)
dashboards = data.get('dashboards', [])
for db in dashboards:
    dbid = db.get('id', '0')
    name = db.get('name', 'NO_NAME')
    print(f'--- Dashboard {dbid} ({name}) ---')
    # When exporting, Chronograf wraps in {meta, dashboard} format
    export = {
        'meta': {
            'chronografVersion': '1.10.9',
            'sources': data.get('meta', {}).get('sources', {})
        },
        'dashboard': db
    }
    print(json.dumps(export, indent=2)[:2000])
    print()
" 2>/dev/null || echo "(could not parse dashboards)"
echo ""

echo "--- 5. Testing minimal dashboard import (flat format) ---"
TEST_DASHBOARD='{
  "name": "Diagnostic Test Dashboard",
  "organization": "default",
  "cells": [
    {
      "i": "diag-test-00000000",
      "x": 0,
      "y": 0,
      "w": 4,
      "h": 4,
      "name": "Test Cell",
      "queries": [
        {
          "query": "from(bucket: \"telegraf\") |> range(start: v.timeRangeStart) |> filter(fn: (r) => r._measurement == \"test\")",
          "queryConfig": {
            "database": "",
            "measurement": "",
            "retentionPolicy": "",
            "fields": [],
            "tags": {},
            "groupBy": {"time": "", "tags": []},
            "areTagsAccepted": false,
            "rawText": "from(bucket: \"telegraf\") |> range(start: v.timeRangeStart) |> filter(fn: (r) => r._measurement == \"test\")",
            "range": null,
            "shifts": null
          },
          "source": "/chronograf/v1/sources/0",
          "type": "flux"
        }
      ],
      "axes": {
        "x": {"bounds": ["", ""], "label": "", "prefix": "", "suffix": "", "base": "10", "scale": "linear"},
        "y": {"bounds": ["", ""], "label": "", "prefix": "", "suffix": "", "base": "10", "scale": "linear"},
        "y2": {"bounds": ["", ""], "label": "", "prefix": "", "suffix": "", "base": "10", "scale": "linear"}
      },
      "type": "line",
      "colors": [
        {"id": "test1", "type": "scale", "hex": "#31C0F6", "name": "Nineteen Eighty Four", "value": "0"},
        {"id": "test2", "type": "scale", "hex": "#A500A5", "name": "Nineteen Eighty Four", "value": "0"},
        {"id": "test3", "type": "scale", "hex": "#FF7E27", "name": "Nineteen Eighty Four", "value": "0"}
      ],
      "legend": {},
      "tableOptions": {"verticalTimeAxis": true, "sortBy": {"internalName": "time", "displayName": "", "visible": true}, "wrapping": "truncate", "fixFirstColumn": true},
      "fieldOptions": [{"internalName": "time", "displayName": "", "visible": true}],
      "timeFormat": "MM/DD/YYYY HH:mm:ss",
      "decimalPlaces": {"isEnforced": true, "digits": 2},
      "note": "",
      "noteVisibility": "default"
    }
  ],
  "templates": []
}'

echo "Sending test dashboard..."
RESPONSE=$(echo "${TEST_DASHBOARD}" | curl -sf -X POST -d @- \
    -H "Content-Type: application/json" \
    "${API}/dashboards" 2>/dev/null || echo 'FAILED')

echo "${RESPONSE}" | python3 -m json.tool 2>/dev/null || echo "${RESPONSE}"

# Check if the response has name and cells
echo ""
echo "Post-import analysis:"
echo "${RESPONSE}" | python3 -c "
import sys, json
data = json.load(sys.stdin)
name = data.get('name', 'NO_NAME')
cells = data.get('cells', [])
print(f'  Imported name: \"{name}\"')
print(f'  Imported cells: {len(cells)}')
if cells:
    for c in cells:
        print(f'    Cell: \"{c.get(\"name\", \"unnamed\")}\"')
else:
    print('  WARNING: No cells imported!')
" 2>/dev/null || echo "(could not parse import response)"

# Now re-fetch the dashboard to see what Chronograf stored
NEW_ID=$(echo "${RESPONSE}" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null)
if [ -n "${NEW_ID}" ]; then
    echo ""
    echo "--- 6. Re-fetching imported dashboard id=${NEW_ID} ---"
    FETCHED=$(curl -sf "${API}/dashboards/${NEW_ID}" 2>/dev/null || echo '{}')
    echo "${FETCHED}" | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(f'  Name: \"{data.get(\"name\", \"NO_NAME\")}\"')
print(f'  Cells: {len(data.get(\"cells\", []))}')
" 2>/dev/null || echo "${FETCHED}" | head -20

    # Clean up: delete the test dashboard
    echo ""
    echo "Cleaning up test dashboard..."
    curl -sf -X DELETE "${API}/dashboards/${NEW_ID}" 2>/dev/null && echo "Deleted." || echo "Could not delete (may need manual cleanup)."
fi

echo ""
echo "=== Diagnostics complete ==="