"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.docSearchLocal = void 0;
/**
 * doc.search.local - Local FTS (SQLite FTS5) tool
 */
const sqlite3_1 = __importDefault(require("sqlite3"));
let db = null;
// Promisify database methods
const dbAll = (sql, ...params) => {
    return new Promise((resolve, reject) => {
        if (!db) {
            reject(new Error('Database not initialized'));
            return;
        }
        db.all(sql, params, (err, rows) => {
            if (err)
                reject(err);
            else
                resolve(rows);
        });
    });
};
const dbRun = (sql, ...params) => {
    return new Promise((resolve, reject) => {
        if (!db) {
            reject(new Error('Database not initialized'));
            return;
        }
        db.run(sql, params, function (err) {
            if (err)
                reject(err);
            else
                resolve(this); // 'this' contains lastID and changes
        });
    });
};
// Initialize the database and populate with sample data
async function initializeDB() {
    return new Promise((resolve, reject) => {
        // Open a new SQLite database
        db = new sqlite3_1.default.Database('./search_index.db', (err) => {
            if (err) {
                reject(err);
                return;
            }
            // Create table with FTS5
            db.exec(`
        CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
          id,
          content,
          uri,
          stamp,
          tokenize='porter'
        )
      `, (err) => {
                if (err) {
                    reject(err);
                    return;
                }
                // Clear existing data for fresh start
                dbRun('DELETE FROM documents_fts').then(() => {
                    // Insert sample data if provided
                    // In a real implementation, this would be populated from sample.corpus.jsonl
                    const sampleDocs = [
                        { id: '1', uri: 'policy/refunds', content: 'Our refund policy allows returns within 30 days of purchase. Items must be in original condition with tags attached.', stamp: '2023-01-15T00:00:00Z' },
                        { id: '2', uri: 'policy/shipping', content: 'Standard shipping takes 3-5 business days. Express shipping is available for an additional fee and takes 1-2 business days.', stamp: '2023-02-20T00:00:00Z' },
                        { id: '3', uri: 'faq/warranty', content: 'All products come with a 90-day warranty covering manufacturing defects. Extended warranty options are available for purchase.', stamp: '2023-03-10T00:00:00Z' },
                        { id: '4', uri: 'policy/exchanges', content: 'Exchanges are allowed within 45 days of purchase. Items must be unworn and in original packaging.', stamp: '2023-04-05T00:00:00Z' },
                        { id: '5', uri: 'support/returns', content: 'To initiate a return, please contact our support team with your order number and reason for return. A return label will be provided via email.', stamp: '2023-05-12T00:00:00Z' }
                    ];
                    // Prepare statement
                    const stmt = db.prepare('INSERT INTO documents_fts (id, content, uri, stamp) VALUES (?, ?, ?, ?)');
                    for (const doc of sampleDocs) {
                        stmt.run([doc.id, doc.content, doc.uri, doc.stamp]);
                    }
                    stmt.finalize((err) => {
                        if (err) {
                            reject(err);
                        }
                        else {
                            resolve();
                        }
                    });
                }).catch(reject);
            });
        });
    });
}
exports.docSearchLocal = {
    spec: {
        name: 'doc.search.local',
        description: 'Local full-text search using SQLite FTS5',
        io: {
            input: {
                type: 'object',
                properties: {
                    q: { type: 'string' },
                    k: { type: 'number', default: 8 },
                    filter: { type: 'object' }
                },
                required: ['q']
            },
            output: {
                type: 'object',
                properties: {
                    hits: {
                        type: 'array',
                        items: {
                            type: 'object',
                            properties: {
                                id: { type: 'string' },
                                uri: { type: 'string' },
                                score: { type: 'number' },
                                snippet: { type: 'string' },
                                stamp: { type: 'string', format: 'date-time' }
                            },
                            required: ['id', 'uri', 'score', 'snippet', 'stamp']
                        }
                    }
                },
                required: ['hits']
            }
        },
        constraints: {
            input_tokens_max: 512,
            latency_p50_ms: 120,
            cost_per_call_usd: 0.0001,
            rate_limit_qps: 50,
            side_effects: false
        },
        provenance: {
            attribution_required: true
        }
    },
    invoke: async (args) => {
        if (!db) {
            await initializeDB();
        }
        const query = args.q;
        const k = args.k || 8;
        // Perform FTS5 search
        const results = await dbAll(`
      SELECT id, uri, stamp, 
             snippet(documents_fts, 1, '[HIGHLIGHT]', '[/HIGHLIGHT]', '...', 160) as snippet,
             rank as score
      FROM documents_fts
      WHERE documents_fts MATCH ?
      ORDER BY rank
      LIMIT ?
    `, query, k);
        // Process results to create proper snippets
        const hits = results.map((row) => ({
            id: row.id,
            uri: row.uri,
            score: row.score,
            snippet: row.snippet,
            stamp: row.stamp
        }));
        return { result: { hits } };
    }
};
//# sourceMappingURL=doc.search.local.js.map