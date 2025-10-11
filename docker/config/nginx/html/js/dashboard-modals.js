// @ts-check

/**
 * @file Modal management for config, logs, and restart confirmation
 */

import { checkAllServices } from './dashboard-services.js';

// ============================================================================
// Config Modal
// ============================================================================

/**
 * View Telegraf configuration
 */
export async function viewTelegrafConfig() {
    const modal = document.getElementById('configModal');
    const content = document.getElementById('config-content');
    
    if (!modal || !content) return;
    
    modal.style.display = 'block';
    content.textContent = 'Loading...';
    
    try {
        const response = await fetch('/api/config/telegraf', {
            cache: 'no-cache'
        });
        
        if (response.ok) {
            const text = await response.text();
            content.textContent = text;
        } else {
            content.textContent = `Error loading config: ${response.status} ${response.statusText}`;
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        content.textContent = `Error: ${errorMessage}`;
    }
}

/**
 * Close config modal
 */
export function closeConfigModal() {
    const modal = document.getElementById('configModal');
    if (modal) {
        modal.style.display = 'none';
    }
}

// ============================================================================
// Logs Modal
// ============================================================================

/**
 * View Docker logs (from System Info panel)
 */
export function viewDockerLogs() {
    const modal = document.getElementById('logsModal');
    if (modal) {
        modal.style.display = 'block';
        loadContainerLogs();
    }
}

/**
 * View logs for a specific container (from service card)
 * @param {string} container - Container name
 */
export function viewContainerLogs(container) {
    const modal = document.getElementById('logsModal');
    const select = /** @type {HTMLSelectElement} */ (document.getElementById('container-select'));
    
    if (!modal || !select) return;
    
    // Set the dropdown to the selected container
    select.value = container;
    
    // Open modal and load logs
    modal.style.display = 'block';
    loadContainerLogs();
}

/**
 * Load logs for selected container
 */
export async function loadContainerLogs() {
    const select = /** @type {HTMLSelectElement} */ (document.getElementById('container-select'));
    const content = document.getElementById('logs-content');
    
    if (!select || !content) return;
    
    const container = select.value;
    content.textContent = 'Loading logs...';
    
    try {
        const response = await fetch(`/api/containers/${container}/logs`, {
            cache: 'no-cache'
        });
        
        if (response.ok) {
            const text = await response.text();
            content.textContent = text || 'No logs available';
        } else {
            content.textContent = `Error loading logs: ${response.status} ${response.statusText}`;
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        content.textContent = `Error: ${errorMessage}`;
    }
}

/**
 * Close logs modal
 */
export function closeLogsModal() {
    const modal = document.getElementById('logsModal');
    if (modal) {
        modal.style.display = 'none';
    }
}

// ============================================================================
// Restart Modal
// ============================================================================

/** @type {string|null} */
let pendingRestart = null;

/**
 * Show restart confirmation modal
 * @param {string} container - Container name
 * @param {string} displayName - Display name for the service
 */
export function confirmRestart(container, displayName) {
    pendingRestart = container;
    const serviceNameEl = document.getElementById('restart-service-name');
    const modal = document.getElementById('restartModal');
    
    if (serviceNameEl) {
        serviceNameEl.textContent = displayName;
    }
    if (modal) {
        modal.style.display = 'block';
    }
}

/**
 * Close restart modal
 */
export function closeRestartModal() {
    const modal = document.getElementById('restartModal');
    if (modal) {
        modal.style.display = 'none';
    }
    pendingRestart = null;
}

/**
 * Execute the restart
 */
export async function executeRestart() {
    if (!pendingRestart) return;
    
    const container = pendingRestart;
    const button = /** @type {HTMLButtonElement} */ (document.getElementById(`restart-${container}`));
    
    // Close modal
    closeRestartModal();
    
    if (!button) return;
    
    // Disable button and show restarting state
    button.disabled = true;
    button.textContent = '⏳ Restarting...';
    
    try {
        const response = await fetch(`/api/containers/${container}/restart`, {
            method: 'POST',
            cache: 'no-cache'
        });
        
        /** @type {RestartResponse} */
        const result = await response.json();
        
        if (response.ok && result.success) {
            showNotification(`✅ ${container} restarted successfully`, 'success');
            // Refresh service status after 2 seconds
            setTimeout(() => checkAllServices(), 2000);
        } else {
            showNotification(`❌ Failed to restart ${container}: ${result.message}`, 'error');
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        showNotification(`❌ Error: ${errorMessage}`, 'error');
    } finally {
        // Re-enable button
        button.disabled = false;
        button.textContent = '🔄 Restart';
    }
}

/**
 * Show notification toast
 * @param {string} message - Message to display
 * @param {'success'|'error'} type - Notification type
 */
function showNotification(message, type) {
    const notification = document.createElement('div');
    notification.className = `notification ${type}`;
    notification.textContent = message;
    document.body.appendChild(notification);
    
    setTimeout(() => {
        notification.remove();
    }, 5000);
}
