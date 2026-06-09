(function() {
    var THEMES = {
        'indigo-purple': {
            name: 'Indigo Purple',
            vars: {
                '--t-gradient-start': '#667eea',
                '--t-gradient-end': '#764ba2',
                '--t-accent': '#667eea',
                '--t-accent-hover': '#5568d3',
                '--t-card-bg': '#ffffff',
                '--t-text': '#333333',
                '--t-text-muted': '#666666',
                '--t-text-label': '#888888',
                '--t-header-text': '#ffffff',
                '--t-border': '#eeeeee',
                '--t-border-dark': '#dee2e6',
                '--t-step-bg': '#f8f9fa',
                '--t-code-bg': '#f5f5f5',
                '--t-input-border': '#ced4da',
                '--t-input-focus': '#667eea',
                '--t-input-focus-ring': 'rgba(102,126,234,0.1)',
                '--t-btn-primary': '#667eea',
                '--t-btn-primary-hover': '#5568d3',
                '--t-btn-cancel': '#6b7280',
                '--t-btn-cancel-hover': '#4b5563',
                '--t-btn-disabled': '#cccccc',
                '--t-upload-border': '#667eea',
                '--t-upload-border-hover': '#764ba2',
                '--t-upload-hover-bg': '#f8f9fa',
                '--t-upload-dragover-bg': '#e9ecef',
                '--t-session-bg': '#e7f3ff',
                '--t-session-text': '#004085',
                '--t-badge-toggle-bg': '#dbeafe',
                '--t-badge-toggle-text': '#1d4ed8',
                '--t-badge-momentary-bg': '#fce7f3',
                '--t-badge-momentary-text': '#be185d',
                '--t-toggle-off-bg': '#e5e7eb',
                '--t-toggle-off-text': '#374151',
                '--t-toggle-off-hover': '#d1d5db',
                '--t-card-shadow': 'rgba(0,0,0,0.1)',
                '--t-card-shadow-hover': 'rgba(0,0,0,0.15)',
                '--t-modal-shadow': 'rgba(0,0,0,0.3)',
                '--t-file-border': '#dee2e6',
                '--t-spinner-track': '#f3f3f3',
                '--t-notif-success-bg': '#10b981',
                '--t-notif-success-text': '#ffffff',
                '--t-notif-error-bg': '#ef4444',
                '--t-notif-error-text': '#ffffff'
            }
        },
        'ocean-teal': {
            name: 'Ocean Teal',
            vars: {
                '--t-gradient-start': '#0f766e',
                '--t-gradient-end': '#0d9488',
                '--t-accent': '#0d9488',
                '--t-accent-hover': '#0f766e',
                '--t-card-bg': '#ffffff',
                '--t-text': '#1a1a1a',
                '--t-text-muted': '#555555',
                '--t-text-label': '#888888',
                '--t-header-text': '#ffffff',
                '--t-border': '#e2e8f0',
                '--t-border-dark': '#cbd5e1',
                '--t-step-bg': '#f0fdfa',
                '--t-code-bg': '#f0fdfa',
                '--t-input-border': '#94a3b8',
                '--t-input-focus': '#0d9488',
                '--t-input-focus-ring': 'rgba(13,148,136,0.15)',
                '--t-btn-primary': '#0d9488',
                '--t-btn-primary-hover': '#0f766e',
                '--t-btn-cancel': '#64748b',
                '--t-btn-cancel-hover': '#475569',
                '--t-btn-disabled': '#cbd5e1',
                '--t-upload-border': '#0d9488',
                '--t-upload-border-hover': '#0f766e',
                '--t-upload-hover-bg': '#f0fdfa',
                '--t-upload-dragover-bg': '#ccfbf1',
                '--t-session-bg': '#f0fdfa',
                '--t-session-text': '#134e4a',
                '--t-badge-toggle-bg': '#ccfbf1',
                '--t-badge-toggle-text': '#0f766e',
                '--t-badge-momentary-bg': '#fce7f3',
                '--t-badge-momentary-text': '#be185d',
                '--t-toggle-off-bg': '#e2e8f0',
                '--t-toggle-off-text': '#334155',
                '--t-toggle-off-hover': '#cbd5e1',
                '--t-card-shadow': 'rgba(0,0,0,0.1)',
                '--t-card-shadow-hover': 'rgba(0,0,0,0.15)',
                '--t-modal-shadow': 'rgba(0,0,0,0.3)',
                '--t-file-border': '#cbd5e1',
                '--t-spinner-track': '#e2e8f0',
                '--t-notif-success-bg': '#10b981',
                '--t-notif-success-text': '#ffffff',
                '--t-notif-error-bg': '#ef4444',
                '--t-notif-error-text': '#ffffff'
            }
        },
        'industrial-orange': {
            name: 'Industrial Orange',
            vars: {
                '--t-gradient-start': '#c2410c',
                '--t-gradient-end': '#ea580c',
                '--t-accent': '#ea580c',
                '--t-accent-hover': '#c2410c',
                '--t-card-bg': '#ffffff',
                '--t-text': '#1c1917',
                '--t-text-muted': '#57534e',
                '--t-text-label': '#a8a29e',
                '--t-header-text': '#ffffff',
                '--t-border': '#e7e5e4',
                '--t-border-dark': '#d6d3d1',
                '--t-step-bg': '#fafaf9',
                '--t-code-bg': '#fafaf9',
                '--t-input-border': '#a8a29e',
                '--t-input-focus': '#ea580c',
                '--t-input-focus-ring': 'rgba(234,88,12,0.15)',
                '--t-btn-primary': '#ea580c',
                '--t-btn-primary-hover': '#c2410c',
                '--t-btn-cancel': '#78716c',
                '--t-btn-cancel-hover': '#57534e',
                '--t-btn-disabled': '#d6d3d1',
                '--t-upload-border': '#ea580c',
                '--t-upload-border-hover': '#c2410c',
                '--t-upload-hover-bg': '#fff7ed',
                '--t-upload-dragover-bg': '#ffedd5',
                '--t-session-bg': '#fff7ed',
                '--t-session-text': '#9a3412',
                '--t-badge-toggle-bg': '#dbeafe',
                '--t-badge-toggle-text': '#1d4ed8',
                '--t-badge-momentary-bg': '#fce7f3',
                '--t-badge-momentary-text': '#be185d',
                '--t-toggle-off-bg': '#e7e5e4',
                '--t-toggle-off-text': '#44403c',
                '--t-toggle-off-hover': '#d6d3d1',
                '--t-card-shadow': 'rgba(0,0,0,0.1)',
                '--t-card-shadow-hover': 'rgba(0,0,0,0.15)',
                '--t-modal-shadow': 'rgba(0,0,0,0.3)',
                '--t-file-border': '#d6d3d1',
                '--t-spinner-track': '#e7e5e4',
                '--t-notif-success-bg': '#10b981',
                '--t-notif-success-text': '#ffffff',
                '--t-notif-error-bg': '#ef4444',
                '--t-notif-error-text': '#ffffff'
            }
        },
        'slate-steel': {
            name: 'Slate Steel',
            vars: {
                '--t-gradient-start': '#334155',
                '--t-gradient-end': '#475569',
                '--t-accent': '#64748b',
                '--t-accent-hover': '#475569',
                '--t-card-bg': '#ffffff',
                '--t-text': '#1e293b',
                '--t-text-muted': '#64748b',
                '--t-text-label': '#94a3b8',
                '--t-header-text': '#ffffff',
                '--t-border': '#e2e8f0',
                '--t-border-dark': '#cbd5e1',
                '--t-step-bg': '#f8fafc',
                '--t-code-bg': '#f1f5f9',
                '--t-input-border': '#94a3b8',
                '--t-input-focus': '#475569',
                '--t-input-focus-ring': 'rgba(71,85,105,0.15)',
                '--t-btn-primary': '#475569',
                '--t-btn-primary-hover': '#334155',
                '--t-btn-cancel': '#94a3b8',
                '--t-btn-cancel-hover': '#64748b',
                '--t-btn-disabled': '#cbd5e1',
                '--t-upload-border': '#64748b',
                '--t-upload-border-hover': '#475569',
                '--t-upload-hover-bg': '#f8fafc',
                '--t-upload-dragover-bg': '#e2e8f0',
                '--t-session-bg': '#f1f5f9',
                '--t-session-text': '#1e293b',
                '--t-badge-toggle-bg': '#e2e8f0',
                '--t-badge-toggle-text': '#334155',
                '--t-badge-momentary-bg': '#fce7f3',
                '--t-badge-momentary-text': '#be185d',
                '--t-toggle-off-bg': '#e2e8f0',
                '--t-toggle-off-text': '#334155',
                '--t-toggle-off-hover': '#cbd5e1',
                '--t-card-shadow': 'rgba(0,0,0,0.08)',
                '--t-card-shadow-hover': 'rgba(0,0,0,0.12)',
                '--t-modal-shadow': 'rgba(0,0,0,0.25)',
                '--t-file-border': '#cbd5e1',
                '--t-spinner-track': '#e2e8f0',
                '--t-notif-success-bg': '#10b981',
                '--t-notif-success-text': '#ffffff',
                '--t-notif-error-bg': '#ef4444',
                '--t-notif-error-text': '#ffffff'
            }
        },
        'dark-mode': {
            name: 'Dark Mode',
            vars: {
                '--t-gradient-start': '#1e293b',
                '--t-gradient-end': '#0f172a',
                '--t-accent': '#60a5fa',
                '--t-accent-hover': '#93bbfd',
                '--t-card-bg': '#1e293b',
                '--t-text': '#e2e8f0',
                '--t-text-muted': '#94a3b8',
                '--t-text-label': '#64748b',
                '--t-header-text': '#f8fafc',
                '--t-border': '#334155',
                '--t-border-dark': '#475569',
                '--t-step-bg': '#0f172a',
                '--t-code-bg': '#0f172a',
                '--t-input-border': '#475569',
                '--t-input-focus': '#60a5fa',
                '--t-input-focus-ring': 'rgba(96,165,250,0.2)',
                '--t-btn-primary': '#3b82f6',
                '--t-btn-primary-hover': '#60a5fa',
                '--t-btn-cancel': '#475569',
                '--t-btn-cancel-hover': '#64748b',
                '--t-btn-disabled': '#334155',
                '--t-upload-border': '#3b82f6',
                '--t-upload-border-hover': '#60a5fa',
                '--t-upload-hover-bg': '#1e293b',
                '--t-upload-dragover-bg': '#334155',
                '--t-session-bg': '#1e3a5f',
                '--t-session-text': '#93bbfd',
                '--t-badge-toggle-bg': '#1e3a5f',
                '--t-badge-toggle-text': '#60a5fa',
                '--t-badge-momentary-bg': '#4a1942',
                '--t-badge-momentary-text': '#f472b6',
                '--t-toggle-off-bg': '#334155',
                '--t-toggle-off-text': '#cbd5e1',
                '--t-toggle-off-hover': '#475569',
                '--t-card-shadow': 'rgba(0,0,0,0.4)',
                '--t-card-shadow-hover': 'rgba(0,0,0,0.6)',
                '--t-modal-shadow': 'rgba(0,0,0,0.5)',
                '--t-file-border': '#475569',
                '--t-spinner-track': '#334155',
                '--t-notif-success-bg': '#059669',
                '--t-notif-success-text': '#ffffff',
                '--t-notif-error-bg': '#dc2626',
                '--t-notif-error-text': '#ffffff'
            }
        },
        'aquatic-sci-fi': {
            name: 'Aquatic Sci-Fi',
            vars: {
                '--t-gradient-start': '#088D86',
                '--t-gradient-end': '#03A391',
                '--t-accent': '#12AF9E',
                '--t-accent-hover': '#24BFAE',
                '--t-card-bg': '#0d3d3b',
                '--t-text': '#EAFBF7',
                '--t-text-muted': '#BFEFE7',
                '--t-text-label': '#6BD4C4',
                '--t-header-text': '#EAFBF7',
                '--t-border': '#1a6b66',
                '--t-border-dark': '#1a6b66',
                '--t-step-bg': '#0a4f4d',
                '--t-code-bg': '#0a4f4d',
                '--t-input-border': '#1a6b66',
                '--t-input-focus': '#E5E132',
                '--t-input-focus-ring': 'rgba(229,225,50,0.15)',
                '--t-btn-primary': '#12AF9E',
                '--t-btn-primary-hover': '#24BFAE',
                '--t-btn-cancel': '#4a9e97',
                '--t-btn-cancel-hover': '#6BD4C4',
                '--t-btn-disabled': '#1a6b66',
                '--t-upload-border': '#12AF9E',
                '--t-upload-border-hover': '#E5E132',
                '--t-upload-hover-bg': '#0d3d3b',
                '--t-upload-dragover-bg': '#1a6b66',
                '--t-session-bg': '#0a4f4d',
                '--t-session-text': '#6BD4C4',
                '--t-badge-toggle-bg': '#0a4f4d',
                '--t-badge-toggle-text': '#24BFAE',
                '--t-badge-momentary-bg': '#4a3d00',
                '--t-badge-momentary-text': '#E5E132',
                '--t-toggle-off-bg': '#1a6b66',
                '--t-toggle-off-text': '#BFEFE7',
                '--t-toggle-off-hover': '#247a74',
                '--t-card-shadow': 'rgba(0,0,0,0.3)',
                '--t-card-shadow-hover': 'rgba(0,0,0,0.45)',
                '--t-modal-shadow': 'rgba(0,0,0,0.5)',
                '--t-file-border': '#1a6b66',
                '--t-spinner-track': '#1a6b66',
                '--t-notif-success-bg': '#72D090',
                '--t-notif-success-text': '#0a4f4d',
                '--t-notif-error-bg': '#E5E132',
                '--t-notif-error-text': '#0a4f4d'
            }
        }
    };

    var serverTheme = null;

    function applyTheme(themeKey) {
        var theme = THEMES[themeKey];
        if (!theme) return;
        var root = document.documentElement;
        var vars = theme.vars;
        for (var key in vars) {
            if (vars.hasOwnProperty(key)) {
                root.style.setProperty(key, vars[key]);
            }
        }
        try {
            localStorage.setItem('monitoring-theme', themeKey);
        } catch(e) {}
        var select = document.getElementById('theme-picker');
        if (select) select.value = themeKey;
    }

    function populateDropdown(select, themesList) {
        var currentVal = select.value;
        select.innerHTML = '';
        for (var i = 0; i < themesList.length; i++) {
            var t = themesList[i];
            var opt = document.createElement('option');
            opt.value = t.key;
            opt.textContent = t.name + (serverTheme && t.key === serverTheme ? ' (device default)' : '');
            select.appendChild(opt);
        }
        select.value = currentVal;
    }

    function builtInList() {
        var list = [];
        for (var key in THEMES) {
            if (THEMES.hasOwnProperty(key)) {
                list.push({ key: key, name: THEMES[key].name });
            }
        }
        return list;
    }

    function initThemePicker() {
        var container = document.createElement('div');
        container.id = 'theme-picker-container';
        container.innerHTML = '<label for="theme-picker">Theme</label> <select id="theme-picker"></select>';

        var style = document.createElement('style');
        style.textContent = [
            '#theme-picker-container {',
            '  position: fixed;',
            '  top: 10px;',
            '  right: 10px;',
            '  z-index: 9999;',
            '  display: flex;',
            '  align-items: center;',
            '  gap: 6px;',
            '  font-size: 13px;',
            '}',
            '#theme-picker-container label {',
            '  color: var(--t-header-text, #fff);',
            '  text-shadow: 0 1px 3px rgba(0,0,0,0.4);',
            '  font-weight: 500;',
            '}',
            '#theme-picker {',
            '  padding: 4px 8px;',
            '  border-radius: 6px;',
            '  border: 1px solid var(--t-header-text, #fff);',
            '  background: var(--t-gradient-start, #667eea);',
            '  color: var(--t-header-text, #fff);',
            '  font-size: 13px;',
            '  cursor: pointer;',
            '  outline: none;',
            '}',
            '#theme-picker option {',
            '  background: #fff;',
            '  color: #333;',
            '}'
        ].join('\n');
        document.head.appendChild(style);
        document.body.appendChild(container);

        var select = document.getElementById('theme-picker');

        populateDropdown(select, builtInList());

        var saved = null;
        try { saved = localStorage.getItem('monitoring-theme'); } catch(e) {}
        var initial = saved || 'indigo-purple';
        select.value = initial;
        applyTheme(initial);

        select.addEventListener('change', function() {
            applyTheme(select.value);
        });

        fetch('/theme.json')
            .then(function(r) { return r.json(); })
            .then(function(data) {
                if (data && data.theme && THEMES[data.theme]) {
                    serverTheme = data.theme;

                    var saved = null;
                    try { saved = localStorage.getItem('monitoring-theme'); } catch(e) {}

                    if (!saved) {
                        select.value = serverTheme;
                        applyTheme(serverTheme);
                    }

                    populateDropdown(select, builtInList());

                    fetch('/themes.json')
                        .then(function(r) { return r.json(); })
                        .then(function(registry) {
                            if (registry && registry.themes && registry.themes.length > 0) {
                                populateDropdown(select, registry.themes);
                            }
                        })
                        .catch(function() {});
                }
            })
            .catch(function() {});
    }

    function domReady(fn) {
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', fn);
        } else {
            fn();
        }
    }

    domReady(initThemePicker);

    window.MonitoringThemes = { THEMES: THEMES, applyTheme: applyTheme };
})();
