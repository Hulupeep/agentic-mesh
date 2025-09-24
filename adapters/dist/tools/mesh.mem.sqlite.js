"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.meshMemSqlite = void 0;
/**
 * mesh.mem.sqlite - Typed memory store on SQLite
 */
const sqlite3_1 = __importDefault(require("sqlite3"));
let db = null;
// Promisify database methods
const dbGet = (sql, ...params) => {
    return new Promise((resolve, reject) => {
        if (!db) {
            reject(new Error('Database not initialized'));
            return;
        }
        db.get(sql, params, (err, row) => {
            if (err)
                reject(err);
            else
                resolve(row);
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
// Initialize the database
async function initializeDB() {
    return new Promise((resolve, reject) => {
        db = new sqlite3_1.default.Database('./memory.db', (err) => {
            if (err) {
                reject(err);
                return;
            }
            // Create the memory table
            db.exec(`
        CREATE TABLE IF NOT EXISTS memory (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL,
          provenance TEXT,
          confidence REAL,
          ttl TEXT,
          timestamp TEXT NOT NULL
        )
      `, (err) => {
                if (err) {
                    reject(err);
                }
                else {
                    resolve();
                }
            });
        });
    });
}
exports.meshMemSqlite = {
    spec: {
        name: 'mesh.mem.sqlite',
        description: 'SQLite-based memory store with confidence and provenance',
        io: {
            input: {
                type: 'object',
                properties: {
                    operation: {
                        type: 'string',
                        enum: ['read', 'write', 'forget']
                    },
                    key: { type: 'string' },
                    value: {},
                    provenance: {
                        type: 'array',
                        items: { type: 'string' }
                    },
                    confidence: {
                        type: 'number',
                        minimum: 0,
                        maximum: 1
                    },
                    ttl: { type: 'string' }
                },
                required: ['operation', 'key']
            },
            output: {
                type: 'object',
                properties: {
                    success: { type: 'boolean' },
                    value: {},
                    message: { type: 'string' }
                }
            }
        }
    },
    invoke: async (args) => {
        if (!db) {
            await initializeDB();
        }
        const { operation, key } = args;
        try {
            switch (operation) {
                case 'read':
                    const row = await dbGet('SELECT value FROM memory WHERE key = ? AND (ttl IS NULL OR timestamp >= datetime("now", "-" || ttl))', key);
                    if (row) {
                        return {
                            result: {
                                success: true,
                                value: JSON.parse(row.value)
                            }
                        };
                    }
                    else {
                        return {
                            result: {
                                success: false,
                                message: `Key ${key} not found or expired`
                            }
                        };
                    }
                case 'write':
                    // Check confidence if provided
                    if (args.confidence !== undefined && args.confidence < 0.8) {
                        return {
                            result: {
                                success: false,
                                message: 'Memory write rejected: confidence too low (< 0.8)'
                            }
                        };
                    }
                    // Insert or update the memory entry
                    await dbRun(`
            INSERT OR REPLACE INTO memory (key, value, provenance, confidence, ttl, timestamp)
            VALUES (?, ?, ?, ?, ?, datetime('now'))
          `, key, JSON.stringify(args.value), args.provenance ? JSON.stringify(args.provenance) : null, args.confidence || null, args.ttl || 'P90D' // Default TTL is 90 days
                    );
                    return {
                        result: {
                            success: true,
                            message: `Key ${key} written successfully`
                        }
                    };
                case 'forget':
                    await dbRun('DELETE FROM memory WHERE key = ?', key);
                    return {
                        result: {
                            success: true,
                            message: `Key ${key} deleted`
                        }
                    };
                default:
                    return {
                        result: {
                            success: false,
                            message: `Unknown operation: ${operation}`
                        }
                    };
            }
        }
        catch (error) {
            return {
                result: {
                    success: false,
                    message: `Error performing operation: ${error.message}`
                }
            };
        }
    }
};
//# sourceMappingURL=mesh.mem.sqlite.js.map