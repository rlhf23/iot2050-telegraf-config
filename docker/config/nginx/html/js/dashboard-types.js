// @ts-check

/**
 * @file Type definitions for the Dashboard
 */

/**
 * @typedef {Object} ServiceConfig
 * @property {string} healthUrl - Health check endpoint
 * @property {number} port - Service port number
 */

/**
 * @typedef {Object} SystemMemory
 * @property {number} used_mb - Used memory in MB
 * @property {number} total_mb - Total memory in MB
 * @property {string} percent - Memory usage percentage
 */

/**
 * @typedef {Object} SystemDisk
 * @property {string} used - Used disk space
 * @property {string} total - Total disk space
 * @property {string} percent - Disk usage percentage
 */

/**
 * @typedef {Object} SystemInfo
 * @property {string} hostname - System hostname
 * @property {string} ip_address - IP address
 * @property {string} system_time - Current system time
 * @property {string} architecture - System architecture
 * @property {string} uptime - System uptime
 * @property {string} load_average - Load average
 * @property {SystemMemory} memory - Memory information
 * @property {SystemDisk} disk - Disk information
 * @property {number} containers_running - Number of running containers
 */

/**
 * @typedef {Object} RestartResponse
 * @property {boolean} success - Whether restart was successful
 * @property {string} message - Response message
 */

// Export empty object to make this a module
export {};
