// @ts-check

/**
 * @file Type definitions for the Telegraf Config Generator
 * These JSDoc types provide IDE autocomplete and type checking without TypeScript compilation
 */

// ============================================================================
// API Request/Response Types
// ============================================================================

/**
 * @typedef {Object} SessionResponse
 * @property {boolean} success
 * @property {string} session_id
 */

/**
 * @typedef {Object} UploadedFile
 * @property {string} name
 * @property {number} size
 * @property {string} path
 */

/**
 * @typedef {Object} FileConfig
 * @property {string} namespace
 * @property {number} interval_ms
 * @property {boolean} use_listener
 * @property {string|null} custom_ip
 */

// ============================================================================
// OPC-UA Types
// ============================================================================

/**
 * @typedef {'Variable'|'Object'|'ObjectType'|'Method'|'DataType'|'ReferenceType'|'View'} NodeClass
 */

/**
 * @typedef {Object} OpcUaNode
 * @property {string} node_id - Node identifier (e.g., "ns=2;s=MyNode")
 * @property {string} browse_name - Browse name of the node
 * @property {string} display_name - Display name of the node
 * @property {NodeClass} node_class - Type of node
 * @property {string|null} data_type - Data type for Variable nodes
 * @property {string|null} description - Node description
 * @property {OpcUaNode[]} children - Child nodes
 * @property {boolean} selected - Whether node is selected
 * @property {boolean} children_loaded - Whether children have been loaded
 * @property {boolean} has_more_children - Whether there are more children to load
 * @property {string|null} continuation_point - Continuation point for pagination
 */

/**
 * @typedef {Object} ConnectOpcUaRequest
 * @property {string} opcua_ip
 * @property {string|null} opcua_username
 * @property {string|null} opcua_password
 * @property {boolean} anonymous
 */

/**
 * @typedef {Object} ConnectOpcUaResponse
 * @property {boolean} success
 * @property {string} session_id
 * @property {string} message
 * @property {number} node_count
 */

/**
 * @typedef {Object} GetNodesRequest
 * @property {string} session_id
 * @property {string|null} node_id - null for root nodes
 */

/**
 * @typedef {Object} GetNodesResponse
 * @property {boolean} success
 * @property {OpcUaNode[]} nodes
 */

/**
 * @typedef {Object} SelectedNodeForConfig
 * @property {string} node_id
 * @property {number} namespace
 * @property {string} browse_name
 * @property {string} display_name
 * @property {string} measurement_name
 * @property {number} interval_ms
 * @property {string} data_type
 */

// ============================================================================
// Config Generation Types
// ============================================================================

/**
 * @typedef {Object} GenerateConfigRequest
 * @property {string} session_id
 * @property {string|null} opcua_ip
 * @property {string|null} opcua_username
 * @property {string|null} opcua_password
 * @property {boolean} anonymous
 * @property {string|null} iot_host
 * @property {string|null} iot_username
 * @property {string|null} iot_password
 * @property {string} output_format
 * @property {Array<{filename: string, namespace: string, interval_ms: number, use_listener: boolean, custom_ip: string|null}>} file_configs
 * @property {SelectedNodeForConfig[]|null} selected_nodes
 */

/**
 * @typedef {Object} GenerateConfigResponse
 * @property {boolean} success
 * @property {string} message
 * @property {string|null} config_path
 * @property {string|null} preview
 */

/**
 * @typedef {Object} DeployConfigRequest
 * @property {string} session_id
 * @property {string} config_path
 */

/**
 * @typedef {Object} DeployConfigResponse
 * @property {boolean} success
 * @property {string} message
 */

// Export empty object to make this a module
export {};
