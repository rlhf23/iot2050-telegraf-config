# Simatic Standard Namespace Discovery

## Overview

Implement OPC-UA variable discovery using the Siemens Simatic Standard Namespace (ns=3) instead of ServerInterfaces. This provides full hierarchical variable names directly from the PLC without requiring XML file uploads.

## Background

### Current Approach (ServerInterfaces)

- **Namespace**: ns=4, ns=5, ns=6, etc. (PLC-specific)
- **Identifiers**: Numeric (`ns=4;i=59`) or short strings (`ns=4;s="Position"`)
- **Names**: Only the last component available (e.g., "Position", "Status")
- **Problem**: No hierarchical path information; requires XML upload for meaningful names

### New Approach (Simatic Standard Namespace)

- **Namespace**: ns=3 (consistent across Siemens PLCs)
- **Identifiers**: Full string paths (`ns=3;s="Database1"."Axis1"."Position"`)
- **Names**: Complete hierarchical path embedded in identifier
- **Benefit**: No XML files needed; direct discovery of meaningful variable names

## Technical Details

### NodeId Format

```
ns=3;s="DB_Name"."Struct_Name"."Variable_Name"
```

Examples:
```
ns=3;s="Database1"."Axis1"."Position"
ns=3;s="MainProcessDB"."StatusBlock"."AutoMode"
ns=3;s="SequenceControlDB"."StepValue"
```

### Identifier Parsing

Input identifier string:
```
"Database1"."Axis1"."Position"
```

Parse to extract:
- **Full path**: `Database1.Axis1.Position` (for display/config name)
- **Raw identifier**: `"Database1"."Axis1"."Position"` (for NodeId)
- **Parent DB**: `Database1` (for grouping)
- **Variable name**: `Position` (last component)

### Config Output Format

```toml
[[inputs.opcua.group]]
  name = "Database1"
  namespace = "3"
  identifier_type = "s"
  nodes = [
    {name="Database1.Axis1.Position", identifier="\"Database1\".\"Axis1\".\"Position\""},
    {name="Database1.SensorValue", identifier="\"Database1\".\"SensorValue\""}
  ]
```

## Implementation Plan

### Phase 1: Core Discovery Function

**File**: `src/backend/opcua_poller.rs`

Add new function:
```rust
pub fn discover_simatic_standard(
    &self,
) -> Result<DiscoveredStandardData, TelegrafError>
```

Steps:
1. Connect to OPC-UA server
2. Browse ns=3 (Simatic Standard Namespace)
3. Filter for Variable nodes with string identifiers
4. Parse identifiers to extract hierarchical paths
5. Group variables by parent DB
6. Return structured discovery results

### Phase 2: Data Structures

**File**: `src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredStandardData {
    pub variables: Vec<DiscoveredStandardVariable>,
    pub groups: HashMap<String, Vec<DiscoveredStandardVariable>>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredStandardVariable {
    pub node_id: NodeId,
    pub full_path: String,
    pub identifier: String,
    pub parent_db: String,
    pub variable_name: String,
    pub data_type: Option<String>,
}
```

### Phase 3: Identifier Parser

**File**: `src/backend/opcua_poller.rs`

```rust
fn parse_simatic_identifier(identifier: &str) -> Option<SimaticIdentifier> {
    // Input:  "\"Database1\".\"Axis1\".\"Position\""
    // Output: SimaticIdentifier {
    //     full_path: "Database1.Axis1.Position",
    //     raw: "\"Database1\".\"Axis1\".\"Position\"",
    //     parent_db: "Database1",
    //     variable_name: "Position"
    // }
}
```

Parsing logic:
1. Remove outer quotes if present
2. Split by `"."`  delimiter
3. Clean each component (remove quotes)
4. Join with `.` for display name
5. Keep original for NodeId reference

### Phase 4: API Endpoint

**File**: `docker/api-service/src/opcua.rs`

```rust
pub async fn discover_simatic_standard(
    State(state): State<AppState>,
    Json(req): Json<DiscoverRequest>,
) -> Result<Json<DiscoverResponse>, (StatusCode, Json<ErrorResponse>)>
```

Request:
```json
{
  "opcua_ip": "192.168.0.1",
  "username": "user",
  "password": "pass"
}
```

Response:
```json
{
  "success": true,
  "data": {
    "variables": [...],
    "groups": {
      "Database1": [...],
      "ProcessStatusDB": [...]
    },
    "total_count": 150
  }
}
```

### Phase 5: Frontend Integration

**File**: `docker/config/nginx/html/` (new page or modify existing)

- Add "Discover from Standard Namespace" button
- Display discovered variables grouped by DB
- Allow selection/deselection of variables
- Generate config from selection

## Key Decisions

### 1. Namespace Index
- **Decision**: ns=3 is the Simatic Standard Namespace
- **Rationale**: Consistent across Siemens S7 PLCs
- **Fallback**: Could make configurable if needed

### 2. Identifier Type
- **Decision**: Use `identifier_type = "s"` (string)
- **Rationale**: NodeId uses string identifiers
- **Format**: Full quoted path as identifier

### 3. Grouping Strategy
- **Decision**: Group by first component of path (parent DB)
- **Example**: `Database1.Axis1.Position` → group "Database1"
- **Alternative**: Could group by full path depth

### 4. Variable Filtering
- **Decision**: Only discover Variable nodes (NodeClass::Variable)
- **Skip**: Objects, Methods, Views, ReferenceTypes
- **Optional**: Could add DataType filtering

### 5. Backward Compatibility
- **Decision**: Keep existing ServerInterfaces discovery
- **Rationale**: Some users may still need it
- **UI**: Add toggle or separate button for Standard Namespace

## Edge Cases

### 1. Embedded Quotes
If a DB name contains quotes:
```
"DB ""quoted"" name"."Variable"
```
Handle by escaping or preserving during parsing.

### 2. Deep Nesting
Variables with many levels:
```
"DB"."Struct"."SubStruct"."SubSubStruct"."Variable"
```
All levels preserved in full_path.

### 3. Empty/Invalid Identifiers
- Skip nodes with empty identifiers
- Log warning for unparseable formats
- Continue discovery on errors

### 4. Large Namespaces
- Use continuation points (already implemented)
- Add progress reporting for UI
- Consider pagination for very large PLCs

## Testing Plan

### Unit Tests
- Identifier parser with various formats
- Grouping logic
- Config generation from discovered data

### Integration Tests
- Mock OPC-UA server with Standard Namespace
- Full discovery workflow
- Config generation end-to-end

### Manual Testing
- Connect to real Siemens S7-1500 PLC
- Verify namespace index (ns=3)
- Compare discovered names with TIA Portal
- Validate generated telegraf.conf

## Effort Estimate

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| Phase 1: Core Discovery | 1-2 days | None |
| Phase 2: Data Structures | 0.5 day | Phase 1 |
| Phase 3: Identifier Parser | 0.5 day | Phase 2 |
| Phase 4: API Endpoint | 1 day | Phase 3 |
| Phase 5: Frontend | 1-2 days | Phase 4 |
| Testing | 1-2 days | All phases |
| **Total** | **5-8 days** | |

## Open Questions

1. **DataType filtering**: Should we read DataType attribute and allow filtering by type (BOOL, INT, REAL, etc.)?

2. **Description attribute**: Should we read the Description attribute for additional context?

3. **Writable variables**: Should we check AccessLevel and filter out write-only variables?

4. **Namespace validation**: Should we verify ns=3 exists before browsing, or just attempt and handle errors?

5. **Performance**: For PLCs with thousands of variables, should we add:
   - Progress callbacks?
   - Streaming results?
   - Cancellation support?

## Success Criteria

1. Discovery returns full hierarchical names without XML files
2. Generated telegraf.conf matches existing XML-based format
3. Works on Siemens S7-1200/S7-1500 PLCs
4. Handles continuation points for large namespaces
5. UI allows variable selection and config generation

## Variable Pattern Matching

### Problem

Even with the Standard Namespace providing full hierarchical paths, users may not know which variables are interesting for their monitoring needs. A PLC may have thousands of variables, and selecting relevant ones manually is tedious.

### Solution: Pattern-Based Variable Discovery

Users provide a portable JSON file containing patterns or known interesting variable names. The system fuzzy-matches these against discovered Standard Namespace variables to suggest relevant nodes.

### Pattern File Format

```json
{
  "version": "1.0",
  "patterns": [
    {
      "pattern": "*_DB.Step",
      "description": "Sequence step state",
      "category": "sequence"
    },
    {
      "pattern": "*StatusBlock.*",
      "description": "Status monitoring",
      "category": "status"
    },
    {
      "pattern": "*Position",
      "description": "Position values",
      "category": "position"
    }
  ],
  "known_variables": [
    "Database1.Axis1.Position",
    "MainProcessDB.StatusBlock.AutoMode",
    "SequenceControlDB.Step"
  ]
}
```

### Fuzzy Matching Algorithm

1. **Exact match**: Discovered variable exactly matches a known variable name
   - Confidence: 100%
   - Auto-select

2. **Pattern match**: Variable matches a wildcard pattern
   - Confidence: 80-95% depending on pattern specificity
   - Suggest to user

3. **No match**: Variable doesn't match any pattern
   - Confidence: 0%
   - Skip or mark as "other"

### Matching Examples

| Discovered Variable | Pattern/Known | Match Type | Confidence |
|---------------------|---------------|------------|------------|
| `Database1.Axis1.Position` | `*Position` | Pattern | 90% |
| `MainProcessDB.Step` | `*_DB.Step` | Pattern | 85% |
| `SequenceControlDB.Step` | `SequenceControlDB.Step` | Exact known | 100% |
| `SystemConfig.Param` | (none) | No match | 0% |

### Privacy Considerations

- Pattern file is **user-provided and user-owned**
- Not stored in repository or cloud
- Contains no PLC identifiers or connection info
- Portable between projects and PLCs

### User Workflow

1. User uploads pattern file (optional)
2. System discovers Standard Namespace variables
3. Variables are scored against patterns
4. High-confidence matches are pre-selected
5. User reviews, accepts, or modifies selections
6. Config is generated from final selection

### Implementation

**File**: `src/backend/pattern_matcher.rs` (new)

```rust
pub struct PatternMatcher {
    patterns: Vec<Pattern>,
    known_variables: Vec<String>,
}

pub struct MatchResult {
    full_path: String,
    confidence: f32,
    match_type: MatchType,
    pattern_matched: Option<String>,
}

pub enum MatchType {
    ExactKnown,    // Exact match from known_variables
    PatternMatch,  // Matched wildcard pattern
    NoMatch,       // No pattern matched
}

impl PatternMatcher {
    pub fn from_json(json: &str) -> Result<Self, Error>;
    pub fn match_variable(&self, variable_path: &str) -> MatchResult;
    pub fn match_all(&self, variables: &[String]) -> Vec<MatchResult>;
}
```

### API Integration

Add optional pattern file upload to discovery endpoint:

```
POST /api/opcua/discover-standard
{
  "opcua_ip": "192.168.0.1",
  "username": "user",
  "password": "pass",
  "pattern_file": "optional: base64 encoded JSON"
}
```

Response includes match confidence:

```json
{
  "variables": [
    {
      "full_path": "Database1.Axis1.Position",
      "confidence": 0.90,
      "match_type": "pattern",
      "pattern_matched": "*Position",
      "selected": true
    },
    {
      "full_path": "SystemConfig.Param",
      "confidence": 0.0,
      "match_type": "none",
      "selected": false
    }
  ]
}
```

## Future Enhancements

1. **Pattern-based filtering**: Filter variables by patterns like `*_DB.Step`, `*Status.*`
2. **AI suggestions**: Use LLM to suggest interesting variables based on names
3. **Variable metadata**: Include engineering units, ranges, descriptions
4. **Favorites**: Save frequently used variable patterns
5. **Comparison**: Compare discovered variables between PLCs
