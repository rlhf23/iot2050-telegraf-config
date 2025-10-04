# WebUI Configuration Generator - Design Document

## Overview

This document outlines the design and implementation plan for a web-based configuration generator interface for the IoT2050 Telegraf monitoring stack. The WebUI will replace/complement the existing GUI and TUI interfaces, providing a browser-based solution for uploading XML configuration files, generating Telegraf configs, and deploying them to the IoT device.

## Motivation

### Why WebUI?

1. **No Installation Required** - Users just need a web browser
2. **Remote Access** - Configure the device from any machine on the network
3. **Cross-Platform** - Works on Windows, Linux, macOS, mobile devices
4. **Better UX** - Modern drag-and-drop interface, visual feedback
5. **Easier Maintenance** - No desktop GUI dependencies (egui, gtk, etc.)
6. **Mobile Friendly** - Can be made responsive for tablets/phones
7. **Multi-User Capable** - Multiple users can access (with proper auth)

### Use Case

Users will:
1. Access the WebUI via browser (e.g., `http://192.168.1.2:80`)
2. Upload XML configuration files via drag-and-drop or file picker
3. Configure settings (namespace, interval, OPC-UA credentials)
4. Optionally browse OPC-UA server nodes
5. Generate Telegraf configuration
6. Preview the generated config
7. Deploy to the Telegraf container
8. Monitor deployment status

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    IoT2050 Device                        │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │              Nginx (Port 80)                       │ │
│  │  - Serves static HTML/JS/CSS                       │ │
│  │  - Proxies /api/* to API service                   │ │
│  └────────────────────────────────────────────────────┘ │
│                          │                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │         API Service (Port 8000)                    │ │
│  │  - File upload endpoint                            │ │
│  │  - Config generation endpoint                      │ │
│  │  - OPC-UA browsing endpoint                        │ │
│  │  - Deploy/status endpoints                         │ │
│  │  - Uses existing Rust backend                      │ │
│  └────────────────────────────────────────────────────┘ │
│                          │                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │    Monitoring Stack (Telegraf, InfluxDB, etc.)    │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### 1. Backend API Service (Rust - Axum)

Extend the existing `docker/api-service` with new endpoints:

```rust
// Configuration Management
POST   /api/config/upload          // Upload XML files (multipart/form-data)
GET    /api/config/files            // List uploaded files for session
DELETE /api/config/files/:name      // Remove an uploaded file
POST   /api/config/generate         // Generate telegraf config from uploaded files
GET    /api/config/preview          // Preview generated config
POST   /api/config/deploy           // Deploy config to telegraf container
GET    /api/config/status           // Check generation/deployment status

// OPC-UA Integration
POST   /api/opcua/connect           // Connect to OPC-UA server
GET    /api/opcua/browse/:nodeId    // Browse OPC-UA nodes (hierarchical)
POST   /api/opcua/select            // Select nodes for monitoring
GET    /api/opcua/disconnect        // Disconnect from OPC-UA server

// Session Management
GET    /api/session/create          // Create new session ID
DELETE /api/session/:id             // Clean up session data
```

#### 2. Frontend (HTML/JS/CSS)

**Technology Stack:**
- **HTML5** - Semantic markup
- **Vanilla JavaScript** or **Alpine.js** - Lightweight reactivity
- **Tailwind CSS** or **Bootstrap** - Modern styling
- **Fetch API** - Backend communication
- **LocalStorage** - Session persistence

**Key UI Components:**
- File upload area (drag-and-drop)
- File list with remove buttons
- Configuration form (namespace, interval, credentials)
- OPC-UA browser (tree view)
- Generate/Deploy buttons
- Status indicators and progress bars
- Error/success notifications
- Config preview modal

#### 3. File Storage

**Storage Strategy:**

```
/home/iotuser/config-uploads/
  ├── session-{uuid}/
  │   ├── uploads/
  │   │   ├── file1.xml
  │   │   ├── file2.xml
  │   │   └── file3.xml
  │   ├── generated/
  │   │   └── telegraf.conf
  │   └── metadata.json
  └── .cleanup-timestamp
```

**Cleanup Policy:**
- Sessions older than 24 hours are automatically cleaned
- Manual cleanup via API endpoint
- Cleanup on successful deployment (optional)

## API Endpoint Details

### File Upload

```http
POST /api/config/upload
Content-Type: multipart/form-data

session_id: string
files: File[]

Response:
{
  "success": true,
  "uploaded_files": [
    {
      "name": "config1.xml",
      "size": 2345,
      "path": "/uploads/session-abc/config1.xml"
    }
  ]
}
```

### Generate Configuration

```http
POST /api/config/generate
Content-Type: application/json

{
  "session_id": "abc123",
  "namespace": "my_namespace",
  "interval_ms": 1000,
  "opcua_ip": "192.168.1.100",
  "opcua_username": "admin",
  "opcua_password": "password",
  "anonymous": false,
  "output_format": "influxdb",
  "selected_files": ["config1.xml", "config2.xml"]
}

Response:
{
  "success": true,
  "config_path": "/generated/session-abc/telegraf.conf",
  "preview": "# Telegraf Configuration\n..."
}
```

### Deploy Configuration

```http
POST /api/config/deploy
Content-Type: application/json

{
  "session_id": "abc123",
  "config_path": "/generated/session-abc/telegraf.conf"
}

Response:
{
  "success": true,
  "message": "Configuration deployed successfully",
  "telegraf_status": "restarting"
}
```

## Implementation Phases

### Phase 1: File Upload & Management (MVP)
**Estimated Time: 2-3 hours**

- [ ] Add file upload endpoint with multipart parsing
- [ ] Implement session-based file storage
- [ ] Add file listing endpoint
- [ ] Add file deletion endpoint
- [ ] Basic XML validation
- [ ] Simple HTML upload form

**Deliverable:** Users can upload and manage XML files via browser

### Phase 2: Configuration Generation
**Estimated Time: 3-4 hours**

- [ ] Integrate existing `ConfigGenerator` into API service
- [ ] Add configuration generation endpoint
- [ ] Add preview endpoint
- [ ] Handle generation errors gracefully
- [ ] Add configuration form to frontend
- [ ] Display generated config preview

**Deliverable:** Users can generate Telegraf configs from uploaded files

### Phase 3: Deployment Integration
**Estimated Time: 2-3 hours**

- [ ] Add deployment endpoint (copy config, restart telegraf)
- [ ] Integrate with Docker API (restart container)
- [ ] Add deployment status checking
- [ ] Add deploy button to frontend
- [ ] Show deployment progress/status
- [ ] Handle deployment errors

**Deliverable:** Users can deploy generated configs to Telegraf

### Phase 4: OPC-UA Browser
**Estimated Time: 4-5 hours**

- [ ] Add OPC-UA connection endpoint
- [ ] Implement connection pooling/management
- [ ] Add node browsing endpoint (hierarchical)
- [ ] Add node selection endpoint
- [ ] Build tree view component in frontend
- [ ] Add node search/filter functionality
- [ ] Handle connection timeouts

**Deliverable:** Users can browse and select OPC-UA nodes

### Phase 5: UI/UX Polish
**Estimated Time: 3-4 hours**

- [ ] Improve styling and layout
- [ ] Add loading indicators
- [ ] Add error notifications
- [ ] Add success confirmations
- [ ] Make responsive (mobile-friendly)
- [ ] Add help text and tooltips
- [ ] Add keyboard shortcuts

**Deliverable:** Professional, polished user interface

### Phase 6: Security & Production Readiness
**Estimated Time: 3-4 hours**

- [ ] Add API key authentication
- [ ] Add rate limiting
- [ ] Add CSRF protection
- [ ] Add input validation and sanitization
- [ ] Add audit logging
- [ ] Add session cleanup cron job
- [ ] Add HTTPS support (via nginx)

**Deliverable:** Production-ready, secure WebUI

**Total Estimated Time: 17-23 hours**

## Security Considerations

### Authentication & Authorization

1. **API Key Authentication**
   - Generate API key on first setup
   - Store in environment variable
   - Require key in `Authorization` header
   - Rate limit by IP/key

2. **Session Management**
   - Generate secure session IDs (UUID v4)
   - Store session data server-side
   - Expire sessions after 24 hours
   - Clean up on logout

3. **Input Validation**
   - Validate file types (XML only)
   - Limit file sizes (max 10MB per file)
   - Sanitize file names
   - Validate configuration parameters

4. **HTTPS/TLS**
   - Configure nginx with TLS certificates
   - Redirect HTTP to HTTPS
   - Use secure cookies

### File Security

- Store uploads in isolated directory
- Use session-based isolation
- Validate XML structure before processing
- Prevent path traversal attacks
- Clean up temporary files

## UI Mockup

```
┌─────────────────────────────────────────────────────────────┐
│  IoT2050 Configuration Generator                      [Help] │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─ Step 1: Upload XML Configuration Files ───────────────┐ │
│  │                                                          │ │
│  │  ┌────────────────────────────────────────────────────┐ │ │
│  │  │  📁 Drag & drop XML files here                     │ │ │
│  │  │     or click to browse                             │ │ │
│  │  └────────────────────────────────────────────────────┘ │ │
│  │                                                          │ │
│  │  Uploaded Files:                                        │ │
│  │  ✓ config1.xml (2.3 KB)                    [Remove]    │ │
│  │  ✓ config2.xml (1.8 KB)                    [Remove]    │ │
│  │  ✓ sensors.xml (3.1 KB)                    [Remove]    │ │
│  │                                                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌─ Step 2: Configure Settings ────────────────────────────┐ │
│  │                                                          │ │
│  │  Namespace:      [my_namespace________________]         │ │
│  │  Interval (ms):  [1000]                                 │ │
│  │  Output Format:  [InfluxDB ▼]                           │ │
│  │                                                          │ │
│  │  ┌─ OPC-UA Server Settings ─────────────────────────┐  │ │
│  │  │  IP Address:  [192.168.1.100_____________]       │  │ │
│  │  │  Username:    [admin_____________________]       │  │ │
│  │  │  Password:    [••••••••••••••••••••••••••]       │  │ │
│  │  │  ☐ Anonymous Authentication                      │  │ │
│  │  │                                                   │  │ │
│  │  │  [Browse OPC-UA Nodes]                           │  │ │
│  │  └──────────────────────────────────────────────────┘  │ │
│  │                                                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌─ Step 3: Generate & Deploy ─────────────────────────────┐ │
│  │                                                          │ │
│  │  [Generate Configuration]  [Preview]                    │ │
│  │                                                          │ │
│  │  Status: ✓ Configuration generated successfully         │ │
│  │                                                          │ │
│  │  [Deploy to Telegraf]                                   │ │
│  │                                                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Dependencies

### Backend (Rust)

```toml
[dependencies]
# Existing dependencies
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.5", features = ["cors", "fs"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# New dependencies for WebUI
multer = "3.0"              # Multipart form parsing
tempfile = "3.8"            # Temporary file handling
uuid = { version = "1.6", features = ["v4", "serde"] }
tokio-util = { version = "0.7", features = ["io"] }

# Integration with main project
iot2050-telegraf-config = { path = "../.." }
```

### Frontend

- No build step required (vanilla JS)
- Optional: Use CDN for Tailwind CSS or Alpine.js
- All dependencies loaded via CDN (no npm/node required)

## Testing Strategy

### Backend Tests

1. **Unit Tests**
   - File upload validation
   - Session management
   - Config generation
   - Error handling

2. **Integration Tests**
   - End-to-end upload → generate → deploy flow
   - OPC-UA connection and browsing
   - Docker container interaction

### Frontend Tests

1. **Manual Testing**
   - File upload (drag-and-drop, file picker)
   - Form validation
   - Error handling
   - Responsive design

2. **Browser Compatibility**
   - Chrome/Chromium
   - Firefox
   - Safari
   - Edge

## Deployment

### Docker Integration

Add new service to `docker-compose.yml`:

```yaml
services:
  config-api:
    build: ./api-service
    container_name: config-api
    ports:
      - "8000:8000"
    volumes:
      - config-uploads:/app/uploads
      - /var/run/docker.sock:/var/run/docker.sock:ro
    environment:
      - RUST_LOG=info
      - API_KEY=${CONFIG_API_KEY}
    networks:
      - monitoring
    restart: unless-stopped

volumes:
  config-uploads:
```

### Nginx Configuration

```nginx
server {
    listen 80;
    server_name _;

    # Serve static WebUI files
    location / {
        root /usr/share/nginx/html;
        try_files $uri $uri/ /index.html;
    }

    # Proxy API requests
    location /api/ {
        proxy_pass http://config-api:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        
        # File upload settings
        client_max_body_size 50M;
    }
}
```

## Future Enhancements

### Short-term
- [ ] Configuration templates (save/load common configs)
- [ ] Batch operations (upload multiple files at once)
- [ ] Real-time log streaming from Telegraf
- [ ] Configuration validation before deployment
- [ ] Rollback to previous configuration

### Long-term
- [ ] Multi-user support with user accounts
- [ ] Role-based access control (RBAC)
- [ ] Configuration history and versioning
- [ ] Scheduled configuration updates
- [ ] Integration with Git for config versioning
- [ ] WebSocket support for real-time updates
- [ ] Mobile app (React Native/Flutter)

## Advantages Over GUI/TUI

| Feature | GUI (egui) | TUI (ratatui) | WebUI |
|---------|-----------|---------------|-------|
| **Installation** | Binary required | Binary required | Browser only |
| **Remote Access** | ❌ No | ❌ No (SSH only) | ✅ Yes |
| **Cross-Platform** | ⚠️ Compile per OS | ⚠️ Compile per OS | ✅ Any browser |
| **Mobile Support** | ❌ No | ❌ No | ✅ Yes |
| **Drag & Drop** | ⚠️ Limited | ❌ No | ✅ Yes |
| **Multi-User** | ❌ No | ❌ No | ✅ Yes |
| **Maintenance** | ⚠️ GUI deps | ✅ Easy | ✅ Easy |
| **File Upload** | File picker | Manual path | Drag & drop |
| **UX Quality** | ⚠️ Basic | ⚠️ Terminal | ✅ Modern |

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Security vulnerabilities** | High | API key auth, input validation, HTTPS |
| **File upload abuse** | Medium | File size limits, rate limiting, cleanup |
| **OPC-UA connection issues** | Medium | Timeout handling, connection pooling |
| **Concurrent user conflicts** | Low | Session isolation, file locking |
| **Browser compatibility** | Low | Use standard APIs, test on common browsers |
| **Large file handling** | Low | Streaming uploads, progress indicators |

## Success Criteria

1. ✅ Users can upload XML files via browser
2. ✅ Users can generate Telegraf configs without CLI/GUI
3. ✅ Users can deploy configs to Telegraf container
4. ✅ Users can browse OPC-UA nodes (optional)
5. ✅ Interface works on desktop and mobile browsers
6. ✅ Secure authentication and authorization
7. ✅ Graceful error handling and user feedback
8. ✅ Response time < 2 seconds for most operations

## Conclusion

The WebUI configuration generator is a highly feasible and valuable addition to the IoT2050 monitoring stack. It leverages existing infrastructure (nginx, Docker, Rust backend) and provides a superior user experience compared to GUI/TUI alternatives. The implementation can be done incrementally, with an MVP achievable in 5-8 hours and a production-ready version in 17-23 hours.

**Recommendation: Proceed with implementation** ✅

---

**Document Version:** 1.0  
**Last Updated:** 2025-10-05  
**Author:** Cascade AI  
**Status:** Design Proposal
