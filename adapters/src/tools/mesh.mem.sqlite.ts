/**
 * mesh.mem.sqlite - Typed memory store on SQLite
 */
import sqlite3 from 'sqlite3';
import { promisify } from 'util';
import { ToolHandler, ToolSpec } from '../common/toolshim';

interface MemoryEntry {
  key: string;
  value: any;
  provenance?: string[];
  confidence?: number;
  ttl?: string; // ISO 8601 duration
  timestamp: string; // ISO 8601 datetime
}

let db: sqlite3.Database | null = null;

// Promisify database methods
const dbGet = (sql: string, ...params: any[]) => {
  return new Promise<any>((resolve, reject) => {
    if (!db) {
      reject(new Error('Database not initialized'));
      return;
    }
    db.get(sql, params, (err, row) => {
      if (err) reject(err);
      else resolve(row);
    });
  });
};

const dbRun = (sql: string, ...params: any[]) => {
  return new Promise<any>((resolve, reject) => {
    if (!db) {
      reject(new Error('Database not initialized'));
      return;
    }
    db.run(sql, params, function(err) {
      if (err) reject(err);
      else resolve(this); // 'this' contains lastID and changes
    });
  });
};

// Initialize the database
async function initializeDB(): Promise<void> {
  return new Promise((resolve, reject) => {
    db = new sqlite3.Database('./memory.db', (err) => {
      if (err) {
        reject(err);
        return;
      }

      // Create the memory table
      db!.exec(`
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
        } else {
          resolve();
        }
      });
    });
  });
}

export const meshMemSqlite: ToolHandler = {
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
    },
    capabilities: ['memory.read', 'memory.write', 'memory.forget'],
    constraints: {
      latency_p50_ms: 80,
      cost_per_call_usd: 0.00005,
      side_effects: false
    }
  },

  invoke: async (args: any): Promise<any> => {
    if (!db) {
      await initializeDB();
    }

    const { operation, key } = args;

    try {
      switch (operation) {
        case 'read':
          const row = await dbGet(
            'SELECT key, value, provenance, confidence, ttl, timestamp FROM memory WHERE key = ? AND (ttl IS NULL OR timestamp >= datetime("now", "-" || ttl))',
            key
          );

          if (row) {
            const provenance = row.provenance ? JSON.parse(row.provenance) : undefined;
            const value = JSON.parse(row.value);

            return {
              result: {
                success: true,
                entry: {
                  key,
                  value,
                  provenance,
                  confidence: row.confidence ?? undefined,
                  ttl: row.ttl ?? undefined,
                  timestamp: row.timestamp
                }
              }
            };
          }

          return {
            result: {
              success: false,
              message: `Key ${key} not found or expired`
            }
          };

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
          await dbRun(
            `
            INSERT OR REPLACE INTO memory (key, value, provenance, confidence, ttl, timestamp)
            VALUES (?, ?, ?, ?, ?, datetime('now'))
          `,
            key,
            JSON.stringify(args.value),
            args.provenance ? JSON.stringify(args.provenance) : null,
            args.confidence ?? null,
            args.ttl || 'P90D' // Default TTL is 90 days
          );

          const stored = await dbGet(
            'SELECT key, value, provenance, confidence, ttl, timestamp FROM memory WHERE key = ?',
            key
          );

          return {
            result: {
              success: true,
              entry: {
                key,
                value: args.value,
                provenance: args.provenance || undefined,
                confidence: args.confidence ?? undefined,
                ttl: stored?.ttl ?? args.ttl || 'P90D',
                timestamp: stored?.timestamp || new Date().toISOString()
              }
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
    } catch (error: any) {
      return { 
        result: { 
          success: false, 
          message: `Error performing operation: ${error.message}` 
        } 
      };
    }
  }
};
