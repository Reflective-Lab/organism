/**
 * FormFiller Test Server with Logging
 *
 * - Serves static forms
 * - Receives and stores form interaction logs
 * - Provides API to query logs
 * - Generates analysis reports
 */

const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = 3001;
const LOGS_FILE = path.join(__dirname, 'logs.json');
const FORMS_DIR = path.join(__dirname, '../forms');

// MIME types
const MIME_TYPES = {
    '.html': 'text/html',
    '.css': 'text/css',
    '.js': 'application/javascript',
    '.json': 'application/json',
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.svg': 'image/svg+xml'
};

// Load existing logs
function loadLogs() {
    try {
        if (fs.existsSync(LOGS_FILE)) {
            return JSON.parse(fs.readFileSync(LOGS_FILE, 'utf8'));
        }
    } catch (e) {
        console.error('Error loading logs:', e);
    }
    return [];
}

// Save logs
function saveLogs(logs) {
    fs.writeFileSync(LOGS_FILE, JSON.stringify(logs, null, 2));
}

// Add CORS headers
function setCorsHeaders(res) {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
}

// Serve static files
function serveStatic(req, res) {
    let filePath = req.url === '/' ? '/index.html' : req.url;
    filePath = path.join(FORMS_DIR, filePath.split('?')[0]);

    const ext = path.extname(filePath);
    const contentType = MIME_TYPES[ext] || 'application/octet-stream';

    fs.readFile(filePath, (err, content) => {
        if (err) {
            if (err.code === 'ENOENT') {
                res.writeHead(404);
                res.end('Not Found');
            } else {
                res.writeHead(500);
                res.end('Server Error');
            }
        } else {
            res.writeHead(200, { 'Content-Type': contentType });
            res.end(content);
        }
    });
}

// API: Add log entry
function handleLogPost(req, res) {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
        try {
            const entry = JSON.parse(body);
            entry.serverTimestamp = new Date().toISOString();

            const logs = loadLogs();
            logs.push(entry);

            // Keep last 10000 entries
            if (logs.length > 10000) {
                logs.splice(0, logs.length - 10000);
            }

            saveLogs(logs);

            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ success: true, count: logs.length }));
        } catch (e) {
            res.writeHead(400, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: e.message }));
        }
    });
}

// API: Get logs
function handleLogsGet(req, res) {
    const url = new URL(req.url, `http://localhost:${PORT}`);
    const params = url.searchParams;

    let logs = loadLogs();

    // Filter by event type
    const event = params.get('event');
    if (event) {
        logs = logs.filter(l => l.event === event);
    }

    // Filter by session
    const session = params.get('session');
    if (session) {
        logs = logs.filter(l => l.sessionId === session);
    }

    // Filter by pathname
    const pathname = params.get('pathname');
    if (pathname) {
        logs = logs.filter(l => l.pathname && l.pathname.includes(pathname));
    }

    // Limit
    const limit = parseInt(params.get('limit') || '100');
    logs = logs.slice(-limit);

    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(logs, null, 2));
}

// API: Get analysis
function handleAnalysisGet(req, res) {
    const logs = loadLogs();

    const analysis = {
        totalLogs: logs.length,
        sessions: [...new Set(logs.map(l => l.sessionId))].length,
        eventCounts: {},
        formLoads: {},
        validationErrors: [],
        fieldUsage: {},
        successRate: { attempts: 0, successes: 0 }
    };

    logs.forEach(log => {
        // Count events
        analysis.eventCounts[log.event] = (analysis.eventCounts[log.event] || 0) + 1;

        // Track form loads by path
        if (log.event === 'form_load' && log.pathname) {
            analysis.formLoads[log.pathname] = (analysis.formLoads[log.pathname] || 0) + 1;
        }

        // Collect validation errors
        if (log.event === 'validation_error') {
            analysis.validationErrors.push({
                pathname: log.pathname,
                field: log.data?.selector || log.data?.name,
                message: log.data?.validationMessage,
                timestamp: log.timestamp
            });
        }

        // Track field usage
        if (log.event === 'field_change' && log.data?.selector) {
            const key = `${log.pathname}:${log.data.selector}`;
            if (!analysis.fieldUsage[key]) {
                analysis.fieldUsage[key] = {
                    pathname: log.pathname,
                    selector: log.data.selector,
                    name: log.data.name,
                    id: log.data.id,
                    fillCount: 0,
                    lastValue: null
                };
            }
            analysis.fieldUsage[key].fillCount++;
            analysis.fieldUsage[key].lastValue = log.data.value;
        }

        // Track submission attempts
        if (log.event === 'form_submit') {
            analysis.successRate.attempts++;
        }
    });

    // Convert fieldUsage to array sorted by fillCount
    analysis.fieldUsage = Object.values(analysis.fieldUsage)
        .sort((a, b) => b.fillCount - a.fillCount);

    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(analysis, null, 2));
}

// API: Get field mapping suggestions
function handleSuggestionsGet(req, res) {
    const logs = loadLogs();

    // Find all form_load events to get field structures
    const formFields = {};

    logs.forEach(log => {
        if (log.event === 'form_load' && log.fields && log.pathname) {
            if (!formFields[log.pathname]) {
                formFields[log.pathname] = {};
            }

            log.fields.forEach(field => {
                const key = field.selector;
                if (!formFields[log.pathname][key]) {
                    formFields[log.pathname][key] = {
                        ...field,
                        suggestedSource: guessDatasource(field),
                        confidence: 0
                    };
                }
            });
        }

        // Improve confidence based on successful fills
        if (log.event === 'field_change' && log.pathname && log.data?.selector) {
            const key = log.data.selector;
            if (formFields[log.pathname]?.[key]) {
                formFields[log.pathname][key].confidence += 0.1;
                formFields[log.pathname][key].lastValue = log.data.value;
            }
        }
    });

    // Convert to array format suitable for FormConfig
    const suggestions = {};
    for (const [pathname, fields] of Object.entries(formFields)) {
        suggestions[pathname] = Object.values(fields).map(f => ({
            selector: f.selector,
            selector_type: f.selector.startsWith('#') ? 'Css' : 'Name',
            field_type: mapFieldType(f.type),
            source: f.suggestedSource,
            confidence: Math.min(f.confidence, 1.0),
            label: f.label,
            required: f.required
        }));
    }

    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(suggestions, null, 2));
}

// Guess data source from field metadata
function guessDatasource(field) {
    const hints = [field.name, field.id, field.label, field.placeholder]
        .filter(Boolean)
        .join(' ')
        .toLowerCase();

    if (field.type === 'email' || hints.includes('email') || hints.includes('e-post') || hints.includes('epost')) {
        return 'Email';
    }
    if (field.type === 'tel' || hints.includes('telefon') || hints.includes('phone') || hints.includes('mobil')) {
        return 'Phone';
    }
    if (hints.includes('personnummer') || hints.includes('personal number')) {
        return 'PersonalNumber';
    }
    if (hints.includes('förnamn') || hints.includes('fornamn') || hints.includes('first')) {
        return 'FirstName';
    }
    if (hints.includes('efternamn') || hints.includes('last') || hints.includes('surname')) {
        return 'LastName';
    }
    if (hints.includes('gata') || hints.includes('adress') || hints.includes('street')) {
        return 'Street';
    }
    if (hints.includes('postnummer') || hints.includes('postal') || hints.includes('zip')) {
        return 'PostalCode';
    }
    if (hints.includes('ort') || hints.includes('stad') || hints.includes('city')) {
        return 'City';
    }

    return null;
}

function mapFieldType(htmlType) {
    const map = {
        'text': 'Text',
        'email': 'Email',
        'tel': 'Phone',
        'date': 'Date',
        'select': 'Select',
        'textarea': 'TextArea',
        'checkbox': 'Checkbox',
        'radio': 'Radio'
    };
    return map[htmlType] || 'Text';
}

// API: Clear logs
function handleLogsClear(req, res) {
    saveLogs([]);
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ success: true, message: 'Logs cleared' }));
}

// Create server
const server = http.createServer((req, res) => {
    setCorsHeaders(res);

    // Handle preflight
    if (req.method === 'OPTIONS') {
        res.writeHead(204);
        res.end();
        return;
    }

    const url = new URL(req.url, `http://localhost:${PORT}`);

    // API routes
    if (url.pathname === '/api/log' && req.method === 'POST') {
        return handleLogPost(req, res);
    }
    if (url.pathname === '/api/logs' && req.method === 'GET') {
        return handleLogsGet(req, res);
    }
    if (url.pathname === '/api/logs' && req.method === 'DELETE') {
        return handleLogsClear(req, res);
    }
    if (url.pathname === '/api/analysis' && req.method === 'GET') {
        return handleAnalysisGet(req, res);
    }
    if (url.pathname === '/api/suggestions' && req.method === 'GET') {
        return handleSuggestionsGet(req, res);
    }

    // Serve static files
    serveStatic(req, res);
});

server.listen(PORT, () => {
    console.log(`
╔═══════════════════════════════════════════════════════════╗
║         FormFiller Test Server with Logging               ║
╠═══════════════════════════════════════════════════════════╣
║  Forms:     http://localhost:${PORT}/                        ║
║                                                           ║
║  API Endpoints:                                           ║
║    POST /api/log         - Add log entry                  ║
║    GET  /api/logs        - Get logs (?event=, ?session=)  ║
║    GET  /api/analysis    - Get analysis report            ║
║    GET  /api/suggestions - Get field mapping suggestions  ║
║    DELETE /api/logs      - Clear all logs                 ║
╚═══════════════════════════════════════════════════════════╝
`);
});
