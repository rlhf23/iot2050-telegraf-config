// @ts-check
/// <reference path="./dashboard-types.js" />

/**
 * @file Service health checking functionality
 */

/** @type {Object.<string, ServiceConfig>} */
const services = {
    grafana: {
        healthUrl: '/grafana/api/health',
        port: 3000
    },
    influxdb: {
        healthUrl: '/influxdb/health',
        port: 8086
    },
    prometheus: {
        healthUrl: '/prometheus/-/healthy',
        port: 9090
    },
    telegraf: {
        healthUrl: '/telegraf/metrics',
        port: 9273
    }
};

/**
 * Check health of a single service
 * @param {string} serviceName - Name of the service
 * @param {ServiceConfig} config - Service configuration
 */
export async function checkServiceHealth(serviceName, config) {
    const statusIndicator = document.getElementById(`${serviceName}-status`);
    const statusText = document.getElementById(`${serviceName}-status-text`);
    const serviceLink = document.getElementById(`${serviceName}-link`);

    if (!statusIndicator || !statusText || !serviceLink) return;

    try {
        const response = await fetch(config.healthUrl, {
            method: 'GET',
            cache: 'no-cache'
        });

        if (response.ok) {
            statusIndicator.className = 'status-indicator healthy';
            statusText.textContent = 'Healthy';
            serviceLink.classList.remove('disabled');
        } else {
            statusIndicator.className = 'status-indicator unhealthy';
            statusText.textContent = `Error (${response.status})`;
            serviceLink.classList.add('disabled');
        }
    } catch (error) {
        // If fetch fails, service is likely down
        statusIndicator.className = 'status-indicator unhealthy';
        statusText.textContent = 'Unavailable';
        serviceLink.classList.add('disabled');
    }
}

/**
 * Check health of all services
 */
export async function checkAllServices() {
    for (const [serviceName, config] of Object.entries(services)) {
        await checkServiceHealth(serviceName, config);
    }
    
    // Update last refresh time
    const now = new Date();
    const lastUpdateEl = document.getElementById('last-update');
    if (lastUpdateEl) {
        lastUpdateEl.textContent = now.toLocaleTimeString();
    }
}

/**
 * Fetch and display system information
 */
export async function updateSystemInfo() {
    try {
        const response = await fetch('/api/system-info', {
            cache: 'no-cache'
        });
        
        if (response.ok) {
            /** @type {SystemInfo} */
            const data = await response.json();
            
            const setTextContent = (/** @type {string} */ id, /** @type {string} */ value) => {
                const el = document.getElementById(id);
                if (el) el.textContent = value;
            };

            setTextContent('sys-hostname', data.hostname || 'N/A');
            setTextContent('sys-ip', data.ip_address || 'N/A');
            setTextContent('sys-time', data.system_time || 'N/A');
            setTextContent('sys-arch', data.architecture || 'N/A');
            setTextContent('sys-uptime', data.uptime || 'N/A');
            setTextContent('sys-load', data.load_average || 'N/A');
            
            // Format memory info
            if (data.memory) {
                const memText = `${data.memory.used_mb} MB / ${data.memory.total_mb} MB (${data.memory.percent}%)`;
                setTextContent('sys-memory', memText);
            }
            
            // Format disk info
            if (data.disk) {
                const diskText = `${data.disk.used} / ${data.disk.total} (${data.disk.percent})`;
                setTextContent('sys-disk', diskText);
            }
            
            setTextContent('sys-containers', String(data.containers_running || 'N/A'));
        } else {
            console.error('Failed to fetch system info:', response.status);
        }
    } catch (error) {
        console.error('Error fetching system info:', error);
    }
}
