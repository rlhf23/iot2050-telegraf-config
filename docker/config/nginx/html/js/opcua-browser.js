// @ts-check
/// <reference path="./types.js" />

/**
 * @file OPC-UA Browser functionality
 * Handles connection, tree navigation, and node selection
 */

import { showNotification } from './utils.js';

// ============================================================================
// State
// ============================================================================

/** @type {string|null} */
let opcuaBrowserSessionId = null;

/** @type {Map<string, OpcUaNode>} */
let selectedNodes = new Map();

/** @type {SelectedNodeForConfig[]} */
let selectedNodesForConfig = [];

// ============================================================================
// Modal Management
// ============================================================================

/**
 * Open the OPC-UA browser modal
 */
export function openOpcUaBrowser() {
    const modal = document.getElementById('opcua-browser-modal');
    if (modal) {
        modal.classList.add('show');
    }
}

/**
 * Close the OPC-UA browser modal
 */
export function closeOpcUaBrowser() {
    const modal = document.getElementById('opcua-browser-modal');
    if (modal) {
        modal.classList.remove('show');
    }
}

// ============================================================================
// Connection
// ============================================================================

/**
 * Connect to OPC-UA server and load root nodes
 */
export async function connectOpcUaBrowser() {
    const opcuaIp = /** @type {HTMLInputElement} */ (document.getElementById('opcua-ip'))?.value;
    if (!opcuaIp) {
        showNotification('Please enter OPC-UA server IP address first', 'error');
        return;
    }

    const statusDiv = document.getElementById('browser-connection-status');
    const statusText = document.getElementById('browser-status-text');
    const connectBtn = /** @type {HTMLButtonElement} */ (document.getElementById('browser-connect-btn'));
    
    if (!statusDiv || !statusText || !connectBtn) return;
    
    statusDiv.style.display = 'block';
    statusText.textContent = 'Connecting to OPC-UA server...';
    connectBtn.disabled = true;

    /** @type {ConnectOpcUaRequest} */
    const requestData = {
        opcua_ip: opcuaIp,
        opcua_username: /** @type {HTMLInputElement} */ (document.getElementById('opcua-username'))?.value || null,
        opcua_password: /** @type {HTMLInputElement} */ (document.getElementById('opcua-password'))?.value || null,
        anonymous: /** @type {HTMLInputElement} */ (document.getElementById('anonymous-auth'))?.checked || false
    };

    try {
        const response = await fetch('/api/opcua/connect', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(requestData)
        });

        /** @type {ConnectOpcUaResponse} */
        const data = await response.json();

        if (data.success) {
            opcuaBrowserSessionId = data.session_id;
            statusText.textContent = `✅ Connected! Found ${data.node_count} root nodes`;
            setTimeout(() => { statusDiv.style.display = 'none'; }, 3000);
            
            // Load root nodes
            await loadRootNodes();
        } else {
            statusText.textContent = `❌ ${data.message}`;
            connectBtn.disabled = false;
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        statusText.textContent = `❌ Error: ${errorMessage}`;
        connectBtn.disabled = false;
    }
}

// ============================================================================
// Node Loading
// ============================================================================

/**
 * Load root nodes from the OPC-UA server
 */
async function loadRootNodes() {
    if (!opcuaBrowserSessionId) return;

    try {
        /** @type {GetNodesRequest} */
        const requestData = {
            session_id: opcuaBrowserSessionId,
            node_id: null
        };

        const response = await fetch('/api/opcua/get-nodes', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(requestData)
        });

        /** @type {GetNodesResponse} */
        const data = await response.json();

        if (data.success) {
            const container = document.getElementById('opcua-tree');
            if (container) {
                renderTree(data.nodes, container);
            }
        } else {
            showNotification('Failed to load nodes', 'error');
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        showNotification(`Error: ${errorMessage}`, 'error');
    }
}

/**
 * Load children for a specific node
 * @param {OpcUaNode} node - Parent node
 * @param {HTMLElement} container - Container to render children into
 */
async function loadNodeChildren(node, container) {
    if (!opcuaBrowserSessionId) return;
    
    container.innerHTML = '<div style="padding: 10px; color: #6c757d;">Loading...</div>';
    
    try {
        /** @type {GetNodesRequest} */
        const requestData = {
            session_id: opcuaBrowserSessionId,
            node_id: node.node_id
        };

        const response = await fetch('/api/opcua/get-nodes', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(requestData)
        });

        /** @type {GetNodesResponse} */
        const data = await response.json();

        if (data.success) {
            container.innerHTML = '';
            data.nodes.forEach(childNode => {
                const childEl = createNodeElement(childNode);
                container.appendChild(childEl);
            });
            node.children_loaded = true;
        } else {
            container.innerHTML = '<div style="padding: 10px; color: #dc3545;">Failed to load children</div>';
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        container.innerHTML = `<div style="padding: 10px; color: #dc3545;">Error: ${errorMessage}</div>`;
    }
}

// ============================================================================
// Tree Rendering
// ============================================================================

/**
 * Render the tree of nodes
 * @param {OpcUaNode[]} nodes - Nodes to render
 * @param {HTMLElement} container - Container element
 */
function renderTree(nodes, container) {
    container.innerHTML = '';
    
    nodes.forEach(node => {
        const nodeEl = createNodeElement(node);
        container.appendChild(nodeEl);
    });
}

/**
 * Create a DOM element for a node
 * @param {OpcUaNode} node - Node to create element for
 * @returns {HTMLElement} Node element
 */
function createNodeElement(node) {
    const div = document.createElement('div');
    div.style.marginLeft = '20px';
    div.style.marginBottom = '5px';
    
    const header = document.createElement('div');
    header.style.display = 'flex';
    header.style.alignItems = 'center';
    header.style.gap = '8px';
    header.style.padding = '4px';
    header.style.cursor = 'pointer';
    header.style.borderRadius = '4px';
    
    header.addEventListener('mouseenter', () => {
        header.style.background = '#f0f0f0';
    });
    header.addEventListener('mouseleave', () => {
        header.style.background = 'transparent';
    });

    // Expand/collapse icon (only for folders)
    const isFolder = node.node_class === 'Object' || node.node_class === 'ObjectType';
    if (isFolder) {
        const icon = document.createElement('span');
        icon.textContent = '▶';
        icon.style.display = 'inline-block';
        icon.style.width = '16px';
        icon.style.fontSize = '12px';
        icon.style.transition = 'transform 0.2s';
        header.appendChild(icon);

        icon.addEventListener('click', async (e) => {
            e.stopPropagation();
            const childContainer = div.querySelector('.node-children');
            if (childContainer instanceof HTMLElement) {
                if (childContainer.style.display === 'none') {
                    // Load children if not loaded
                    if (!node.children_loaded) {
                        await loadNodeChildren(node, childContainer);
                    }
                    childContainer.style.display = 'block';
                    icon.style.transform = 'rotate(90deg)';
                } else {
                    childContainer.style.display = 'none';
                    icon.style.transform = 'rotate(0deg)';
                }
            }
        });
    } else {
        const spacer = document.createElement('span');
        spacer.style.width = '16px';
        spacer.style.display = 'inline-block';
        header.appendChild(spacer);
    }

    // Checkbox (only for variables)
    if (node.node_class === 'Variable') {
        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.checked = selectedNodes.has(node.node_id);
        checkbox.addEventListener('change', () => {
            if (checkbox.checked) {
                selectedNodes.set(node.node_id, node);
            } else {
                selectedNodes.delete(node.node_id);
            }
            updateSelectedCount();
        });
        header.appendChild(checkbox);
    }

    // Node label
    const label = document.createElement('span');
    label.textContent = `${node.display_name} (${node.node_class})`;
    label.style.fontSize = '14px';
    if (node.data_type) {
        label.textContent += ` [${node.data_type}]`;
        label.style.color = '#0066cc';
    }
    header.appendChild(label);

    div.appendChild(header);

    // Children container
    if (isFolder) {
        const childContainer = document.createElement('div');
        childContainer.className = 'node-children';
        childContainer.style.display = 'none';
        div.appendChild(childContainer);
    }

    return div;
}

/**
 * Update the selected node count display
 */
function updateSelectedCount() {
    const count = selectedNodes.size;
    const countEl = document.getElementById('selected-count');
    const useBtn = /** @type {HTMLButtonElement} */ (document.getElementById('use-selected-btn'));
    
    if (countEl) {
        countEl.textContent = `${count} nodes selected`;
    }
    if (useBtn) {
        useBtn.disabled = count === 0;
    }
}

// ============================================================================
// Node Selection
// ============================================================================

/**
 * Transfer selected nodes to the configuration table
 */
export function useSelectedNodes() {
    if (selectedNodes.size === 0) {
        showNotification('No nodes selected', 'error');
        return;
    }

    // Convert selected nodes to config format
    selectedNodesForConfig = [];
    selectedNodes.forEach((node, nodeId) => {
        // Extract namespace from node_id (format: ns=X;...)
        const nsMatch = nodeId.match(/ns=(\d+)/);
        const namespace = nsMatch ? parseInt(nsMatch[1]) : 0;

        selectedNodesForConfig.push({
            node_id: nodeId,
            namespace: namespace,
            browse_name: node.browse_name,
            display_name: node.display_name,
            measurement_name: node.display_name.replace(/[^a-zA-Z0-9_]/g, '_'),
            interval_ms: 1000,
            data_type: node.data_type || 'Unknown'
        });
    });

    // Notify parent that nodes are ready
    window.dispatchEvent(new CustomEvent('nodesSelected', { 
        detail: selectedNodesForConfig 
    }));

    closeOpcUaBrowser();
    showNotification(`Added ${selectedNodesForConfig.length} nodes to configuration`, 'success');
}

/**
 * Get the currently selected nodes for config
 * @returns {SelectedNodeForConfig[]}
 */
export function getSelectedNodesForConfig() {
    return selectedNodesForConfig;
}
