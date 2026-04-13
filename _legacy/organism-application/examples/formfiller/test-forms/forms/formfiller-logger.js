/**
 * FormFiller Logger - Client-side logging for form interactions
 *
 * Logs all form events to localStorage and optionally to a backend server.
 * Used to analyze form filling attempts and improve automation.
 */

(function() {
    'use strict';

    const STORAGE_KEY = 'formfiller_logs';
    const BACKEND_URL = 'http://localhost:3001/api/log';
    const MAX_LOCAL_LOGS = 1000;

    // Current session ID
    const SESSION_ID = 'sess_' + Date.now().toString(36) + Math.random().toString(36).substr(2, 5);

    // Get form metadata
    function getFormMeta() {
        return {
            url: window.location.href,
            pathname: window.location.pathname,
            title: document.title,
            timestamp: new Date().toISOString(),
            sessionId: SESSION_ID,
            userAgent: navigator.userAgent
        };
    }

    // Get all form fields info
    function getFormFields() {
        const fields = [];
        const inputs = document.querySelectorAll('input, select, textarea');

        inputs.forEach((el, index) => {
            if (el.type === 'hidden' || el.type === 'submit' || el.type === 'button') return;

            const label = findLabelFor(el);
            fields.push({
                index,
                tagName: el.tagName.toLowerCase(),
                type: el.type || 'text',
                id: el.id || null,
                name: el.name || null,
                label: label,
                placeholder: el.placeholder || null,
                required: el.required,
                value: el.type === 'password' ? '[REDACTED]' : (el.value || ''),
                checked: el.type === 'checkbox' || el.type === 'radio' ? el.checked : null,
                selector: getBestSelector(el)
            });
        });

        return fields;
    }

    function findLabelFor(el) {
        // Try label[for]
        if (el.id) {
            const label = document.querySelector(`label[for="${el.id}"]`);
            if (label) return label.textContent.trim();
        }

        // Try parent label
        const parentLabel = el.closest('label');
        if (parentLabel) {
            return parentLabel.textContent.trim().replace(el.value, '').trim();
        }

        // Try aria-label
        if (el.getAttribute('aria-label')) {
            return el.getAttribute('aria-label');
        }

        return null;
    }

    function getBestSelector(el) {
        if (el.id) return '#' + el.id;
        if (el.name) return `[name="${el.name}"]`;

        // Generate a CSS path
        const path = [];
        let current = el;
        while (current && current !== document.body) {
            let selector = current.tagName.toLowerCase();
            if (current.id) {
                selector = '#' + current.id;
                path.unshift(selector);
                break;
            }
            if (current.className) {
                selector += '.' + current.className.split(' ').join('.');
            }
            path.unshift(selector);
            current = current.parentElement;
        }
        return path.join(' > ');
    }

    // Log entry structure
    function createLogEntry(event, data = {}) {
        return {
            ...getFormMeta(),
            event,
            data,
            fields: event === 'form_load' || event === 'form_submit' ? getFormFields() : undefined
        };
    }

    // Save to localStorage
    function saveToLocal(entry) {
        try {
            let logs = JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]');
            logs.push(entry);

            // Keep only last N entries
            if (logs.length > MAX_LOCAL_LOGS) {
                logs = logs.slice(-MAX_LOCAL_LOGS);
            }

            localStorage.setItem(STORAGE_KEY, JSON.stringify(logs));
        } catch (e) {
            console.warn('FormFiller Logger: Failed to save to localStorage', e);
        }
    }

    // Send to backend (fire and forget)
    function sendToBackend(entry) {
        try {
            fetch(BACKEND_URL, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(entry),
                mode: 'cors'
            }).catch(() => {
                // Backend not available, that's OK
            });
        } catch (e) {
            // Ignore
        }
    }

    // Main log function
    function log(event, data = {}) {
        const entry = createLogEntry(event, data);
        saveToLocal(entry);
        sendToBackend(entry);
        console.log(`[FormFiller] ${event}`, data);
    }

    // Track field focus
    function setupFieldTracking() {
        document.addEventListener('focusin', (e) => {
            const el = e.target;
            if (el.matches('input, select, textarea')) {
                log('field_focus', {
                    selector: getBestSelector(el),
                    id: el.id,
                    name: el.name,
                    type: el.type
                });
            }
        });

        // Track field changes
        document.addEventListener('change', (e) => {
            const el = e.target;
            if (el.matches('input, select, textarea')) {
                log('field_change', {
                    selector: getBestSelector(el),
                    id: el.id,
                    name: el.name,
                    type: el.type,
                    value: el.type === 'password' ? '[REDACTED]' : el.value,
                    checked: el.checked
                });
            }
        });

        // Track input (for real-time typing detection)
        let inputTimeout = null;
        document.addEventListener('input', (e) => {
            const el = e.target;
            if (el.matches('input, textarea')) {
                clearTimeout(inputTimeout);
                inputTimeout = setTimeout(() => {
                    log('field_input', {
                        selector: getBestSelector(el),
                        id: el.id,
                        name: el.name,
                        valueLength: el.value.length,
                        inputSpeed: 'batch' // Could calculate WPM
                    });
                }, 500);
            }
        });
    }

    // Track form submissions
    function setupFormTracking() {
        document.addEventListener('submit', (e) => {
            const form = e.target;
            log('form_submit', {
                formId: form.id,
                formAction: form.action,
                method: form.method,
                fields: getFormFields()
            });
        });

        // Track validation errors
        document.addEventListener('invalid', (e) => {
            const el = e.target;
            log('validation_error', {
                selector: getBestSelector(el),
                id: el.id,
                name: el.name,
                validationMessage: el.validationMessage,
                value: el.value
            });
        }, true);
    }

    // Track button clicks (especially Next/Prev in wizards)
    function setupButtonTracking() {
        document.addEventListener('click', (e) => {
            const el = e.target.closest('button, input[type="button"], input[type="submit"], a.btn, .btn');
            if (el) {
                log('button_click', {
                    selector: getBestSelector(el),
                    id: el.id,
                    text: el.textContent?.trim() || el.value,
                    type: el.type,
                    className: el.className
                });
            }
        });
    }

    // Track page/step changes in wizards
    function setupStepTracking() {
        // Watch for DOM changes that might indicate step change
        const observer = new MutationObserver((mutations) => {
            mutations.forEach((mutation) => {
                if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
                    const el = mutation.target;
                    if (el.classList.contains('active') || el.classList.contains('show')) {
                        log('step_change', {
                            selector: getBestSelector(el),
                            className: el.className
                        });
                    }
                }
            });
        });

        observer.observe(document.body, {
            attributes: true,
            subtree: true,
            attributeFilter: ['class']
        });
    }

    // Error tracking
    function setupErrorTracking() {
        window.addEventListener('error', (e) => {
            log('js_error', {
                message: e.message,
                filename: e.filename,
                lineno: e.lineno,
                colno: e.colno
            });
        });
    }

    // Export logs
    window.FormFillerLogger = {
        getLogs: () => JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]'),

        clearLogs: () => localStorage.removeItem(STORAGE_KEY),

        downloadLogs: () => {
            const logs = JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]');
            const blob = new Blob([JSON.stringify(logs, null, 2)], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `formfiller-logs-${new Date().toISOString().slice(0,10)}.json`;
            a.click();
            URL.revokeObjectURL(url);
        },

        log: log,

        getSessionId: () => SESSION_ID
    };

    // Initialize
    function init() {
        setupFieldTracking();
        setupFormTracking();
        setupButtonTracking();
        setupStepTracking();
        setupErrorTracking();

        // Log page load
        log('form_load', {
            fields: getFormFields()
        });

        console.log('[FormFiller Logger] Initialized. Session:', SESSION_ID);
        console.log('[FormFiller Logger] Use FormFillerLogger.downloadLogs() to export');
    }

    // Start when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
