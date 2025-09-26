/**
 * mesh.mem.analytics - analytics/reporting over memory entries
 */
import sqlite3 from 'sqlite3';
import { ToolHandler } from '../common/toolshim';

let db: sqlite3.Database | null = null;

const dbGet = (sql: string, ...params: any[]) =>
  new Promise<any>((resolve, reject) => {
    if (!db) {
      reject(new Error('Database not initialized'));
      return;
    }
    db.get(sql, params, (err, row) => {
      if (err) reject(err);
      else resolve(row);
    });
  });

const dbAll = (sql: string, ...params: any[]) =>
  new Promise<any[]>((resolve, reject) => {
    if (!db) {
      reject(new Error('Database not initialized'));
      return;
    }
    db.all(sql, params, (err, rows) => {
      if (err) reject(err);
      else resolve(rows);
    });
  });

async function initializeDB(): Promise<void> {
  return new Promise((resolve, reject) => {
    db = new sqlite3.Database('./memory.db', (err) => {
      if (err) {
        reject(err);
        return;
      }
      db!.serialize(() => {
        db!.run('PRAGMA foreign_keys = ON;');
        db!.run('PRAGMA journal_mode = WAL;');
        // Ensure core tables exist (shared with mesh.mem.sqlite tool)
        db!.run(
          `CREATE TABLE IF NOT EXISTS memory_entries (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            confidence REAL NOT NULL,
            ttl TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            evidence_summary TEXT
          )`,
          (entriesErr) => {
            if (entriesErr) {
              reject(entriesErr);
              return;
            }
          }
        );
        db!.run(
          `CREATE TABLE IF NOT EXISTS memory_provenance (
            key TEXT PRIMARY KEY,
            sources TEXT NOT NULL,
            FOREIGN KEY(key) REFERENCES memory_entries(key) ON DELETE CASCADE
          )`,
          (provErr) => {
            if (provErr) {
              reject(provErr);
              return;
            }
            resolve();
          }
        );
      });
    });
  });
}

function ensureInitialized(): Promise<void> {
  if (db) {
    return Promise.resolve();
  }
  return initializeDB();
}

export const meshMemAnalytics: ToolHandler = {
  spec: {
    name: 'mesh.mem.analytics',
    description: 'Analytics tooling for AMP memory entries',
    io: {
      input: {
        type: 'object',
        properties: {
          operation: {
            type: 'string',
            enum: ['summary', 'list', 'by_key']
          },
          limit: {
            type: 'number',
            minimum: 1,
            maximum: 1000,
            default: 20
          },
          key: {
            type: 'string'
          }
        },
        required: ['operation']
      },
      output: {
        type: 'object',
        properties: {
          success: { type: 'boolean' },
          data: {},
          message: { type: 'string' }
        }
      }
    },
    capabilities: ['memory.analytics'],
    constraints: {
      latency_p50_ms: 60,
      cost_per_call_usd: 0.00002,
      side_effects: false
    }
  },

  invoke: async (args: any) => {
    await ensureInitialized();

    const operation = args.operation;
    try {
      switch (operation) {
        case 'summary': {
          const totals = await dbGet(
            `SELECT
               COUNT(*) AS total,
               SUM(CASE WHEN expires_at <= strftime('%s','now') THEN 1 ELSE 0 END) AS expired,
               SUM(CASE WHEN expires_at <= strftime('%s','now') + 86400 AND expires_at > strftime('%s','now') THEN 1 ELSE 0 END) AS expiring_24h
             FROM memory_entries`
          );

          return {
            result: {
              success: true,
              data: {
                total_entries: totals?.total ?? 0,
                expired_entries: totals?.expired ?? 0,
                expiring_next_24h: totals?.expiring_24h ?? 0
              }
            }
          };
        }
        case 'list': {
          const limit = Math.max(1, Math.min(args.limit ?? 20, 1000));
          const rows = await dbAll(
            `SELECT key, confidence, ttl, created_at, expires_at
             FROM memory_entries
             WHERE expires_at > strftime('%s','now')
             ORDER BY expires_at ASC
             LIMIT ?`,
            limit
          );
          const items = rows.map((row) => ({
            key: row.key,
            confidence: row.confidence,
            ttl: row.ttl,
            created_at: row.created_at,
            expires_at: new Date(row.expires_at * 1000).toISOString()
          }));
          return {
            result: {
              success: true,
              data: { entries: items }
            }
          };
        }
        case 'by_key': {
          if (typeof args.key !== 'string' || args.key.trim().length === 0) {
            return {
              result: {
                success: false,
                message: 'Key is required for by_key operation'
              }
            };
          }
          const row = await dbGet(
            `SELECT e.value, e.confidence, e.ttl, e.created_at, e.expires_at, e.evidence_summary, p.sources
             FROM memory_entries e
             LEFT JOIN memory_provenance p ON e.key = p.key
             WHERE e.key = ?`,
            args.key.trim()
          );
          if (!row) {
            return {
              result: {
                success: false,
                message: `Key ${args.key} not found`
              }
            };
          }
          return {
            result: {
              success: true,
              data: {
                key: args.key.trim(),
                value: JSON.parse(row.value),
                confidence: row.confidence,
                ttl: row.ttl,
                created_at: row.created_at,
                expires_at: new Date(row.expires_at * 1000).toISOString(),
                provenance: row.sources ? JSON.parse(row.sources) : [],
                evidence_summary: row.evidence_summary ? JSON.parse(row.evidence_summary) : undefined
              }
            }
          };
        }
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
          message: error.message || 'Analytics operation failed'
        }
      };
    }
  }
};
