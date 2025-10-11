// @ts-check
/// <reference path="./types.js" />

/**
 * @file Main application entry point
 * Initializes all modules and sets up event handlers
 */

import { closePreviewModal } from './utils.js';
import { 
    createSession, 
    loadFiles, 
    handleFileUpload, 
    deleteFile, 
    updateFileConfig,
    getUploadedFiles 
} from './file-manager.js';
import { 
    openOpcUaBrowser, 
    closeOpcUaBrowser, 
    connectOpcUaBrowser, 
    useSelectedNodes,
    getSelectedNodesForConfig 
} from './opcua-browser.js';
import { 
    generateConfig, 
    deployConfig, 
    viewCurrentConfig,
    pollNamespaces 
} from './config-generator.js';

// ============================================================================
// State
// ============================================================================

/** @type {SelectedNodeForConfig[]} */
let selectedNodesForConfig = [];

// ============================================================================
// Initialization
// ============================================================================

document.addEventListener('DOMContentLoaded', async () => {
    await createSession();
    await loadFiles();
    setupEventListeners();
});

// ============================================================================
// Event Listeners
// ============================================================================

function setupEventListeners() {
    // File upload
    const uploadArea = document.getElementById('upload-area');
    const fileInput = /** @type {HTMLInputElement} */ (document.getElementById('file-input'));

    if (uploadArea && fileInput) {
        uploadArea.addEventListener('click', () => fileInput.click());

        uploadArea.addEventListener('dragover', (e) => {
            e.preventDefault();
            uploadArea.classList.add('dragover');
        });

        uploadArea.addEventListener('dragleave', () => {
            uploadArea.classList.remove('dragover');
        });

        uploadArea.addEventListener('drop', (e) => {
            e.preventDefault();
            uploadArea.classList.remove('dragover');
            if (e.dataTransfer?.files) {
                handleFileUpload(e.dataTransfer.files);
            }
        });

        fileInput.addEventListener('change', (e) => {
            const target = /** @type {HTMLInputElement} */ (e.target);
            if (target.files) {
                handleFileUpload(target.files);
            }
        });
    }

    // Anonymous auth checkbox
    const anonymousAuth = document.getElementById('anonymous-auth');
    if (anonymousAuth) {
        anonymousAuth.addEventListener('change', (e) => {
            const target = /** @type {HTMLInputElement} */ (e.target);
            const username = /** @type {HTMLInputElement} */ (document.getElementById('opcua-username'));
            const password = /** @type {HTMLInputElement} */ (document.getElementById('opcua-password'));
            if (username && password) {
                username.disabled = target.checked;
                password.disabled = target.checked;
            }
        });
    }

    // Modal background clicks
    document.addEventListener('click', (e) => {
        const previewModal = document.getElementById('preview-modal');
        const browserModal = document.getElementById('opcua-browser-modal');
        
        if (e.target === previewModal) {
            closePreviewModal();
        }
        if (e.target === browserModal) {
            closeOpcUaBrowser();
        }
    });

    // Listen for node selection events
    window.addEventListener('nodesSelected', (e) => {
        const event = /** @type {CustomEvent} */ (e);
        selectedNodesForConfig = event.detail;
        renderSelectedNodesTable();
        updateGenerateButtonState();
    });

    // Listen for file changes
    window.addEventListener('filesChanged', () => {
        updateGenerateButtonState();
    });
}

// ============================================================================
// Selected Nodes Table
// ============================================================================

/**
 * Render the selected nodes table
 */
function renderSelectedNodesTable() {
    const tbody = document.getElementById('selected-nodes-tbody');
    const section = document.getElementById('selected-nodes-section');
    
    if (!tbody || !section) return;

    if (selectedNodesForConfig.length === 0) {
        section.style.display = 'none';
        return;
    }

    section.style.display = 'block';
    tbody.innerHTML = '';

    selectedNodesForConfig.forEach((node, index) => {
        const row = document.createElement('tr');
        row.style.borderBottom = '1px solid #dee2e6';

        // Node Name
        const nameCell = document.createElement('td');
        nameCell.style.padding = '10px';
        nameCell.textContent = node.display_name;
        row.appendChild(nameCell);

        // Namespace
        const nsCell = document.createElement('td');
        nsCell.style.padding = '10px';
        nsCell.textContent = node.namespace.toString();
        row.appendChild(nsCell);

        // Measurement Name (editable)
        const measurementCell = document.createElement('td');
        measurementCell.style.padding = '10px';
        const measurementInput = document.createElement('input');
        measurementInput.type = 'text';
        measurementInput.value = node.measurement_name;
        measurementInput.style.width = '100%';
        measurementInput.style.padding = '4px 8px';
        measurementInput.style.border = '1px solid #ced4da';
        measurementInput.style.borderRadius = '4px';
        measurementInput.addEventListener('change', (e) => {
            const target = /** @type {HTMLInputElement} */ (e.target);
            selectedNodesForConfig[index].measurement_name = target.value;
        });
        measurementCell.appendChild(measurementInput);
        row.appendChild(measurementCell);

        // Interval (editable)
        const intervalCell = document.createElement('td');
        intervalCell.style.padding = '10px';
        const intervalInput = document.createElement('input');
        intervalInput.type = 'number';
        intervalInput.value = node.interval_ms.toString();
        intervalInput.style.width = '100px';
        intervalInput.style.padding = '4px 8px';
        intervalInput.style.border = '1px solid #ced4da';
        intervalInput.style.borderRadius = '4px';
        intervalInput.addEventListener('change', (e) => {
            const target = /** @type {HTMLInputElement} */ (e.target);
            selectedNodesForConfig[index].interval_ms = parseInt(target.value) || 1000;
        });
        intervalCell.appendChild(intervalInput);
        row.appendChild(intervalCell);

        // Actions
        const actionsCell = document.createElement('td');
        actionsCell.style.padding = '10px';
        const removeBtn = document.createElement('button');
        removeBtn.textContent = 'Remove';
        removeBtn.className = 'btn';
        removeBtn.style.background = '#dc3545';
        removeBtn.style.color = 'white';
        removeBtn.style.padding = '4px 12px';
        removeBtn.style.fontSize = '12px';
        removeBtn.addEventListener('click', () => {
            removeSelectedNode(index);
        });
        actionsCell.appendChild(removeBtn);
        row.appendChild(actionsCell);

        tbody.appendChild(row);
    });

    updateGenerateButtonState();
}

/**
 * Remove a selected node
 * @param {number} index - Index of node to remove
 */
function removeSelectedNode(index) {
    selectedNodesForConfig.splice(index, 1);
    renderSelectedNodesTable();
}

// ============================================================================
// Button State Management
// ============================================================================

/**
 * Update the generate button state
 */
function updateGenerateButtonState() {
    const hasFiles = getUploadedFiles().length > 0;
    const hasNodes = selectedNodesForConfig.length > 0;
    
    const generateBtn = /** @type {HTMLButtonElement} */ (document.getElementById('generate-btn'));
    const pollBtn = /** @type {HTMLButtonElement} */ (document.getElementById('poll-namespaces-btn'));
    
    if (generateBtn) {
        generateBtn.disabled = !(hasFiles || hasNodes);
    }
    if (pollBtn) {
        pollBtn.disabled = !hasFiles;
    }
}

// ============================================================================
// Expose functions to global scope for onclick handlers
// ============================================================================

window.fileManager = {
    deleteFile,
    updateFileConfig
};

window.opcuaBrowser = {
    open: openOpcUaBrowser,
    close: closeOpcUaBrowser,
    connect: connectOpcUaBrowser,
    useSelected: useSelectedNodes
};

window.configGenerator = {
    generate: generateConfig,
    deploy: deployConfig,
    viewCurrent: viewCurrentConfig,
    pollNamespaces: pollNamespaces
};

window.ui = {
    closePreviewModal
};

// Export for type checking
export {};
