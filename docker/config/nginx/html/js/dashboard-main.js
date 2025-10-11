// @ts-check

/**
 * @file Dashboard main entry point
 */

import { checkAllServices, updateSystemInfo } from './dashboard-services.js';
import { 
    viewTelegrafConfig, 
    closeConfigModal,
    viewDockerLogs,
    viewContainerLogs,
    loadContainerLogs,
    closeLogsModal,
    confirmRestart,
    closeRestartModal,
    executeRestart
} from './dashboard-modals.js';

// ============================================================================
// Initialization
// ============================================================================

// Initial checks
checkAllServices();
updateSystemInfo();

// Refresh every 10 seconds
setInterval(() => {
    checkAllServices();
    updateSystemInfo();
}, 10000);

// ============================================================================
// Modal Click Handlers
// ============================================================================

window.onclick = function(event) {
    const configModal = document.getElementById('configModal');
    const logsModal = document.getElementById('logsModal');
    const restartModal = document.getElementById('restartModal');
    
    if (event.target === configModal) {
        closeConfigModal();
    }
    if (event.target === logsModal) {
        closeLogsModal();
    }
    if (event.target === restartModal) {
        closeRestartModal();
    }
};

// ============================================================================
// Expose functions to global scope for onclick handlers
// ============================================================================

window.dashboard = {
    viewTelegrafConfig,
    closeConfigModal,
    viewDockerLogs,
    viewContainerLogs,
    loadContainerLogs,
    closeLogsModal,
    confirmRestart,
    closeRestartModal,
    executeRestart
};

// Export for type checking
export {};
