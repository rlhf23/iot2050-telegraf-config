// @ts-check
/// <reference path="./types.js" />

/**
 * @file Utility functions for UI interactions
 */

/**
 * Show a notification toast
 * @param {string} message - Message to display
 * @param {'success'|'error'} type - Notification type
 */
export function showNotification(message, type) {
    const notification = document.getElementById('notification');
    if (!notification) return;
    
    notification.textContent = message;
    notification.className = `notification ${type} show`;
    
    setTimeout(() => {
        notification.classList.remove('show');
    }, 5000);
}

/**
 * Show or hide a loading indicator
 * @param {string} elementId - ID of the loading element
 * @param {boolean} show - Whether to show or hide
 */
export function showLoading(elementId, show) {
    const element = document.getElementById(elementId);
    if (!element) return;
    
    if (show) {
        element.classList.add('show');
    } else {
        element.classList.remove('show');
    }
}

/**
 * Format file size in human-readable format
 * @param {number} bytes - File size in bytes
 * @returns {string} Formatted file size
 */
export function formatFileSize(bytes) {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

/**
 * Show the preview modal with content
 * @param {string} content - Content to display
 */
export function showPreviewModal(content) {
    const modal = document.getElementById('preview-modal');
    const previewContent = document.getElementById('preview-content');
    
    if (!modal || !previewContent) return;
    
    previewContent.textContent = content;
    modal.classList.add('show');
}

/**
 * Close the preview modal
 */
export function closePreviewModal() {
    const modal = document.getElementById('preview-modal');
    if (!modal) return;
    
    modal.classList.remove('show');
}

/**
 * Escape HTML to prevent XSS
 * @param {string} text - Text to escape
 * @returns {string} Escaped HTML
 */
export function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
