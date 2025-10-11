// @ts-check
/// <reference path="./types.js" />

/**
 * @file Configuration generation and deployment
 */

import { showNotification, showLoading, showPreviewModal } from './utils.js';
import { getSessionId, getUploadedFiles, getFileConfigs } from './file-manager.js';
import { getSelectedNodesForConfig } from './opcua-browser.js';

// ============================================================================
// Config Generation
// ============================================================================

/**
 * Generate Telegraf configuration
 */
export async function generateConfig() {
    const sessionId = getSessionId();
    const uploadedFiles = getUploadedFiles();
    const selectedNodes = getSelectedNodesForConfig();
    
    const hasFiles = uploadedFiles.length > 0;
    const hasNodes = selectedNodes.length > 0;
    
    if (!sessionId || (!hasFiles && !hasNodes)) {
        showNotification('Please upload XML files or select nodes from browser', 'error');
        return;
    }

    showLoading('generate-loading', true);

    // Build file configurations array
    const fileConfigs = getFileConfigs();
    const file_configs = uploadedFiles.map(file => ({
        filename: file.name,
        namespace: fileConfigs[file.name]?.namespace || 'default',
        interval_ms: fileConfigs[file.name]?.interval_ms || 1000,
        use_listener: fileConfigs[file.name]?.use_listener || false,
        custom_ip: fileConfigs[file.name]?.custom_ip || null
    }));

    /** @type {GenerateConfigRequest} */
    const requestData = {
        session_id: sessionId,
        opcua_ip: /** @type {HTMLInputElement} */ (document.getElementById('opcua-ip'))?.value || null,
        opcua_username: /** @type {HTMLInputElement} */ (document.getElementById('opcua-username'))?.value || null,
        opcua_password: /** @type {HTMLInputElement} */ (document.getElementById('opcua-password'))?.value || null,
        anonymous: /** @type {HTMLInputElement} */ (document.getElementById('anonymous-auth'))?.checked || false,
        // IoT credentials not needed - web UI runs locally, uses defaults
        iot_host: null,
        iot_username: null,
        iot_password: null,
        output_format: /** @type {HTMLSelectElement} */ (document.getElementById('output-format'))?.value || 'influxdb',
        file_configs: file_configs,
        selected_nodes: selectedNodes.length > 0 ? selectedNodes : null
    };

    try {
        const response = await fetch('/api/config/generate', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(requestData)
        });

        /** @type {GenerateConfigResponse} */
        const data = await response.json();

        if (data.success) {
            showNotification(data.message, 'success');
            
            // Show preview modal
            if (data.preview) {
                showPreviewModal(data.preview);
            }
            
            // Enable deploy button
            const deployBtn = /** @type {HTMLButtonElement} */ (document.getElementById('deploy-btn'));
            if (deployBtn) {
                deployBtn.disabled = false;
            }
            
            // Store config path for deployment
            if (data.config_path) {
                window.generatedConfigPath = data.config_path;
            }
        } else {
            showNotification(data.message || 'Generation failed', 'error');
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        showNotification('Error generating config: ' + errorMessage, 'error');
    } finally {
        showLoading('generate-loading', false);
    }
}

// ============================================================================
// Config Deployment
// ============================================================================

/**
 * Deploy configuration to Telegraf
 */
export async function deployConfig() {
    const configPath = window.generatedConfigPath;
    
    if (!configPath) {
        showNotification('Please generate configuration first', 'error');
        return;
    }

    if (!confirm('Deploy configuration to Telegraf? This will restart the Telegraf container.')) {
        return;
    }

    showLoading('generate-loading', true);

    const sessionId = getSessionId();
    if (!sessionId) {
        showNotification('Session not initialized', 'error');
        showLoading('generate-loading', false);
        return;
    }

    /** @type {DeployConfigRequest} */
    const requestData = {
        session_id: sessionId,
        config_path: configPath
    };

    try {
        const response = await fetch('/api/config/deploy', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(requestData)
        });

        /** @type {DeployConfigResponse} */
        const data = await response.json();

        if (data.success) {
            showNotification(data.message, 'success');
        } else {
            showNotification(data.message || 'Deployment failed', 'error');
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        showNotification('Error deploying config: ' + errorMessage, 'error');
    } finally {
        showLoading('generate-loading', false);
    }
}

// ============================================================================
// View Current Config
// ============================================================================

/**
 * View the current Telegraf configuration
 */
export async function viewCurrentConfig() {
    try {
        const response = await fetch('/api/config/telegraf', {
            cache: 'no-cache'
        });

        if (response.ok) {
            const text = await response.text();
            showPreviewModal(text);
        } else {
            showNotification(`Error loading config: ${response.status} ${response.statusText}`, 'error');
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        showNotification(`Error: ${errorMessage}`, 'error');
    }
}

// ============================================================================
// Poll Namespaces
// ============================================================================

/**
 * Poll namespaces from OPC-UA server
 */
export async function pollNamespaces() {
    const sessionId = getSessionId();
    const uploadedFiles = getUploadedFiles();
    
    if (!sessionId || uploadedFiles.length === 0) {
        showNotification('Please upload XML files first', 'error');
        return;
    }

    const opcuaIp = /** @type {HTMLInputElement} */ (document.getElementById('opcua-ip'))?.value;
    if (!opcuaIp) {
        showNotification('Please enter OPC-UA server IP address first', 'error');
        return;
    }

    showLoading('poll-loading', true);

    const requestData = {
        session_id: sessionId,
        opcua_ip: opcuaIp,
        opcua_username: /** @type {HTMLInputElement} */ (document.getElementById('opcua-username'))?.value || null,
        opcua_password: /** @type {HTMLInputElement} */ (document.getElementById('opcua-password'))?.value || null,
        anonymous: /** @type {HTMLInputElement} */ (document.getElementById('anonymous-auth'))?.checked || false,
        filenames: uploadedFiles.map(f => f.name)
    };

    try {
        const response = await fetch('/api/opcua/poll-namespaces', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(requestData)
        });

        const data = await response.json();

        if (data.success && data.mappings) {
            // Update file configs with discovered namespaces
            const fileConfigs = getFileConfigs();
            data.mappings.forEach((/** @type {{filename: string, namespace: string}} */ mapping) => {
                if (fileConfigs[mapping.filename]) {
                    fileConfigs[mapping.filename].namespace = mapping.namespace;
                }
            });

            showNotification(`Successfully polled namespaces for ${data.mappings.length} file(s)`, 'success');
            
            // Re-render file list to show updated namespaces
            window.dispatchEvent(new Event('filesChanged'));
        } else {
            showNotification(data.message || 'Failed to poll namespaces', 'error');
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        showNotification('Error polling namespaces: ' + errorMessage, 'error');
    } finally {
        showLoading('poll-loading', false);
    }
}

// Extend window type for TypeScript
declare global {
    interface Window {
        generatedConfigPath?: string;
    }
}
