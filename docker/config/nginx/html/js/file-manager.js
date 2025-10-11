// @ts-check
/// <reference path="./types.js" />

/**
 * @file File upload and management functionality
 */

import { showNotification, showLoading, formatFileSize, escapeHtml } from './utils.js';

// ============================================================================
// State
// ============================================================================

/** @type {string|null} */
let sessionId = null;

/** @type {UploadedFile[]} */
let uploadedFiles = [];

/** @type {Object.<string, FileConfig>} */
let fileConfigs = {};

// ============================================================================
// Session Management
// ============================================================================

/**
 * Create a new session
 */
export async function createSession() {
    try {
        const response = await fetch('/api/session/create');
        /** @type {SessionResponse} */
        const data = await response.json();
        
        if (data.success) {
            sessionId = data.session_id;
            const sessionEl = document.getElementById('session-id');
            if (sessionEl) {
                sessionEl.textContent = data.session_id;
            }
        }
    } catch (error) {
        console.error('Error creating session:', error);
    }
}

/**
 * Get the current session ID
 * @returns {string|null}
 */
export function getSessionId() {
    return sessionId;
}

/**
 * Get uploaded files
 * @returns {UploadedFile[]}
 */
export function getUploadedFiles() {
    return uploadedFiles;
}

/**
 * Get file configurations
 * @returns {Object.<string, FileConfig>}
 */
export function getFileConfigs() {
    return fileConfigs;
}

// ============================================================================
// File Loading
// ============================================================================

/**
 * Load files for the current session
 */
export async function loadFiles() {
    if (!sessionId) return;

    try {
        const response = await fetch(`/api/config/files/${sessionId}`);
        const data = await response.json();

        if (data.success) {
            uploadedFiles = data.files;
            renderFileList();
            updateGenerateButton();
        }
    } catch (error) {
        console.error('Error loading files:', error);
    }
}

// ============================================================================
// File Upload
// ============================================================================

/**
 * Handle file upload
 * @param {FileList} files - Files to upload
 */
export async function handleFileUpload(files) {
    if (!sessionId) {
        showNotification('Session not initialized', 'error');
        return;
    }

    if (files.length === 0) return;

    showLoading('upload-loading', true);

    const formData = new FormData();
    formData.append('session_id', sessionId);

    for (let i = 0; i < files.length; i++) {
        formData.append('files', files[i]);
    }

    try {
        const response = await fetch('/api/config/upload', {
            method: 'POST',
            body: formData
        });

        const data = await response.json();

        if (data.success) {
            showNotification(data.message, 'success');
            await loadFiles();
        } else {
            showNotification(data.message || 'Upload failed', 'error');
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        showNotification('Error uploading files: ' + errorMessage, 'error');
    } finally {
        showLoading('upload-loading', false);
    }
}

// ============================================================================
// File Management
// ============================================================================

/**
 * Delete a file
 * @param {string} filename - Name of file to delete
 */
export async function deleteFile(filename) {
    if (!sessionId) return;

    if (!confirm(`Delete ${filename}?`)) {
        return;
    }

    try {
        const response = await fetch('/api/config/delete', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                session_id: sessionId,
                filename: filename
            })
        });

        const data = await response.json();

        if (data.success) {
            showNotification(data.message, 'success');
            await loadFiles();
        } else {
            showNotification(data.message || 'Delete failed', 'error');
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        showNotification('Error deleting file: ' + errorMessage, 'error');
    }
}

/**
 * Update file configuration
 * @param {string} filename - Filename
 * @param {string} field - Field to update
 * @param {string|number|boolean} value - New value
 */
export function updateFileConfig(filename, field, value) {
    if (!fileConfigs[filename]) {
        fileConfigs[filename] = {
            namespace: 'default',
            interval_ms: 1000,
            use_listener: false,
            custom_ip: null
        };
    }
    
    // @ts-ignore - Dynamic field access
    fileConfigs[filename][field] = value;
}

// ============================================================================
// Rendering
// ============================================================================

/**
 * Render the file list
 */
function renderFileList() {
    const fileList = document.getElementById('file-list');
    if (!fileList) return;
    
    if (uploadedFiles.length === 0) {
        fileList.innerHTML = '';
        return;
    }
    
    fileList.innerHTML = uploadedFiles.map(file => {
        // Initialize config for this file if not exists
        if (!fileConfigs[file.name]) {
            fileConfigs[file.name] = {
                namespace: 'default',
                interval_ms: 1000,
                use_listener: false,
                custom_ip: null
            };
        }
        
        const config = fileConfigs[file.name];
        const escapedName = escapeHtml(file.name);
        
        return `
            <div class="file-item">
                <div class="file-header">
                    <div class="file-info">
                        <span>✓</span>
                        <span class="file-name">${escapedName}</span>
                        <span class="file-size">(${formatFileSize(file.size)})</span>
                    </div>
                    <button class="btn btn-danger" onclick="window.fileManager.deleteFile('${escapedName}')">Remove</button>
                </div>
                <div class="file-config">
                    <div class="file-config-field">
                        <label>Namespace</label>
                        <input type="text" 
                               value="${config.namespace}" 
                               onchange="window.fileManager.updateFileConfig('${file.name}', 'namespace', this.value)"
                               placeholder="e.g., my_namespace">
                    </div>
                    <div class="file-config-field">
                        <label>Interval (ms)</label>
                        <input type="number" 
                               value="${config.interval_ms}" 
                               onchange="window.fileManager.updateFileConfig('${file.name}', 'interval_ms', parseInt(this.value))"
                               min="100"
                               placeholder="1000">
                    </div>
                    <div class="file-config-field">
                        <label>
                            <input type="checkbox" 
                                   ${config.use_listener ? 'checked' : ''}
                                   onchange="window.fileManager.updateFileConfig('${file.name}', 'use_listener', this.checked)">
                            Use Listener
                        </label>
                    </div>
                </div>
            </div>
        `;
    }).join('');
}

/**
 * Update the generate button state
 */
function updateGenerateButton() {
    // This will be called from main.js which has access to selected nodes
    window.dispatchEvent(new Event('filesChanged'));
}
