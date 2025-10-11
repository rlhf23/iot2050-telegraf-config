# JavaScript Modules - Architecture

## Overview
The WebUI JavaScript has been refactored from a monolithic 1459-line script into focused, maintainable modules with JSDoc type annotations.

## File Structure

```
js/
├── types.js              # JSDoc type definitions (135 lines)
├── utils.js              # UI utilities (85 lines)
├── opcua-browser.js      # OPC-UA browser (369 lines)
├── file-manager.js       # File management (280 lines)
├── config-generator.js   # Config generation (258 lines)
└── main.js               # App initialization (279 lines)
```

## Module Responsibilities

### `types.js`
- **Purpose**: Centralized type definitions using JSDoc
- **Exports**: None (just type definitions)
- **Key Types**:
  - `OpcUaNode` - OPC-UA node structure
  - `SelectedNodeForConfig` - Selected node with config
  - `GenerateConfigRequest/Response` - API contracts
  - All API request/response types

### `utils.js`
- **Purpose**: Reusable UI helper functions
- **Exports**:
  - `showNotification(message, type)` - Toast notifications
  - `showLoading(elementId, show)` - Loading indicators
  - `formatFileSize(bytes)` - Human-readable file sizes
  - `showPreviewModal(content)` - Config preview
  - `escapeHtml(text)` - XSS prevention

### `opcua-browser.js`
- **Purpose**: OPC-UA server browsing and node selection
- **Exports**:
  - `openOpcUaBrowser()` - Show browser modal
  - `connectOpcUaBrowser()` - Connect to OPC-UA server
  - `useSelectedNodes()` - Transfer selections to config
  - `getSelectedNodesForConfig()` - Get current selections
- **Features**:
  - Lazy loading tree navigation
  - Checkbox selection for Variables
  - Expand/collapse folders
  - Session management

### `file-manager.js`
- **Purpose**: XML file upload and management
- **Exports**:
  - `createSession()` - Initialize session
  - `handleFileUpload(files)` - Upload XML files
  - `deleteFile(filename)` - Remove file
  - `updateFileConfig(filename, field, value)` - Edit config
  - `getSessionId()`, `getUploadedFiles()`, `getFileConfigs()` - Getters
- **Features**:
  - Drag & drop upload
  - Per-file configuration (namespace, interval, listener mode)
  - Session-based file storage

### `config-generator.js`
- **Purpose**: Telegraf configuration generation and deployment
- **Exports**:
  - `generateConfig()` - Generate telegraf.conf
  - `deployConfig()` - Deploy to Telegraf container
  - `viewCurrentConfig()` - View active config
  - `pollNamespaces()` - Auto-populate namespaces from OPC-UA
- **Features**:
  - Combines XML files + selected nodes
  - Preview before deploy
  - Docker container restart

### `main.js`
- **Purpose**: Application entry point and event wiring
- **Responsibilities**:
  - Initialize all modules on DOMContentLoaded
  - Set up event listeners (file upload, drag & drop, modals)
  - Manage selected nodes table
  - Expose functions to `window` for onclick handlers
- **Global Exports** (for onclick handlers):
  - `window.fileManager.*`
  - `window.opcuaBrowser.*`
  - `window.configGenerator.*`
  - `window.ui.*`

## Type Safety with JSDoc

### How It Works
```javascript
// types.js defines the types
/**
 * @typedef {Object} OpcUaNode
 * @property {string} node_id
 * @property {string} display_name
 * @property {'Variable'|'Object'|'ObjectType'} node_class
 */

// Other files reference them
/// <reference path="./types.js" />

/**
 * @param {OpcUaNode} node
 * @returns {HTMLElement}
 */
function createNodeElement(node) {
    // VS Code now knows node.display_name exists and is a string!
    return element;
}
```

### Benefits
- ✅ **Autocomplete**: IDE suggests properties and methods
- ✅ **Type checking**: Warns about typos and wrong types
- ✅ **Documentation**: Hover to see parameter types
- ✅ **Refactoring**: Rename safely across files
- ✅ **No build step**: Works directly in browser

### Enabling Type Checking
Add to `.vscode/settings.json`:
```json
{
    "js/ts.implicitProjectConfig.checkJs": true
}
```

Or add `// @ts-check` to top of each file.

## Module Loading

### ES6 Modules
```html
<script type="module" src="/js/main.js"></script>
```

- **Automatic**: Browser handles imports
- **Scoped**: No global namespace pollution
- **Async**: Non-blocking page load
- **Cached**: Browser caches modules

### Browser Support
- ✅ Chrome 61+
- ✅ Firefox 60+
- ✅ Safari 11+
- ✅ Edge 16+
- ❌ IE11 (not supported, but who cares)

## Development Workflow

### 1. Edit a Module
```bash
vim js/opcua-browser.js
```

### 2. Refresh Browser
No build step! Just refresh.

### 3. Check Console
```javascript
// Errors show which module failed
// Stack traces point to exact line in source file
```

### 4. Type Check (Optional)
```bash
# If you have TypeScript installed
npx tsc --noEmit --allowJs --checkJs js/*.js
```

## Testing

### Manual Testing
1. Open browser dev tools
2. Go to Sources tab
3. Set breakpoints in individual modules
4. Step through code

### Unit Testing (Future)
```javascript
// Easy to test individual functions
import { formatFileSize } from './utils.js';

assert(formatFileSize(1024) === '1.0 KB');
```

## Common Patterns

### API Calls with Types
```javascript
/** @type {ConnectOpcUaRequest} */
const request = {
    opcua_ip: ip,
    opcua_username: user,
    opcua_password: pass,
    anonymous: false
};

const response = await fetch('/api/opcua/connect', {
    method: 'POST',
    body: JSON.stringify(request)
});

/** @type {ConnectOpcUaResponse} */
const data = await response.json();

// Now data.session_id is typed!
```

### Event Communication
```javascript
// opcua-browser.js dispatches event
window.dispatchEvent(new CustomEvent('nodesSelected', { 
    detail: selectedNodes 
}));

// main.js listens
window.addEventListener('nodesSelected', (e) => {
    const nodes = e.detail;
    renderTable(nodes);
});
```

### Exposing to onclick Handlers
```javascript
// main.js
window.opcuaBrowser = {
    open: openOpcUaBrowser,
    connect: connectOpcUaBrowser
};

// HTML
<button onclick="window.opcuaBrowser.open()">Browse</button>
```

## Migration Notes

### Before (Monolithic)
- 1459 lines in one `<script>` tag
- Hard to find functions
- No type safety
- Global namespace pollution
- Difficult to test

### After (Modular)
- 601 lines HTML + 6 focused modules
- Clear separation of concerns
- JSDoc type annotations
- Module-scoped variables
- Easy to test individual functions

### Breaking Changes
- None! All onclick handlers updated to use `window.*` namespaces
- Same functionality, better organization

## Future Improvements

- [ ] Add unit tests with Jest or Vitest
- [ ] Extract CSS to separate file
- [ ] Add search/filter to OPC-UA browser
- [ ] Implement session cleanup task
- [ ] Add keyboard shortcuts
- [ ] Improve error handling with typed errors

## Questions?

**Q: Why not TypeScript?**  
A: JSDoc gives us 80% of the benefits without build complexity. For this project size, it's the sweet spot.

**Q: Why ES6 modules instead of a bundler?**  
A: Modern browsers support modules natively. No webpack/rollup needed for this scale.

**Q: Can I still use this without a web server?**  
A: No, ES6 modules require HTTP/HTTPS (CORS). But you're already running nginx, so you're good!

**Q: What if I need to support IE11?**  
A: You don't. It's 2025. Let it go.
