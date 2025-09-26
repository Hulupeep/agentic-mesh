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
  confidence: number;
  ttl: string;
  timestamp: string;
  expires_at: string;
  evidence_summary?: any;
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

const dbAll = (sql: string, ...params: any[]) => {
  return new Promise<any[]>((resolve, reject) => {
    if (!db) {
      reject(new Error('Database not initialized'));
      return;
    }
    db.all(sql, params, (err, rows) => {
      if (err) reject(err);
      else resolve(rows);
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
      else resolve(this);
    });
  });
};

const TTL_REGEX = /^P(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?$/;

function parseTtl(ttl?: string): { ttl: string; expiresEpoch: number } {
  const canonical = ttl && ttl.trim().length > 0 ? ttl.trim().toUpperCase() : 'P90D';
  const match = TTL_REGEX.exec(canonical);
  if (!match) {
    throw new Error(`Invalid TTL format: ${ttl}`);
  }

  const days = match[1] ? parseInt(match[1], 10) : 0;
  const hours = match[2] ? parseInt(match[2], 10) : 0;
  const minutes = match[3] ? parseInt(match[3], 10) : 0;
  const seconds = match[4] ? parseInt(match[4], 10) : 0;

  const totalSeconds = days * 24 * 3600 + hours * 3600 + minutes * 60 + seconds;
  if (totalSeconds <= 0) {
    throw new Error(`TTL duration must be positive: ${ttl}`);
  }

  const expiresEpoch = Math.floor(Date.now() / 1000) + totalSeconds;
  return { ttl: canonical, expiresEpoch };
}

function ensureProvenance(raw: any): string[] {
  if (!Array.isArray(raw) || raw.length === 0) {
    throw new Error('Memory write requires non-empty provenance array');
  }
  const cleaned = raw.map((item) => {
    if (typeof item !== 'string' || item.trim().length === 0) {
      throw new Error('Provenance entries must be non-empty strings');
    }
    return item.trim();
  });
  return cleaned;
}

// Initialize the database
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
        db!.run(
          `CREATE TABLE IF NOT EXISTS memory_entries (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            confidence REAL NOT NULL CHECK(confidence >= 0.0 AND confidence <= 1.0),
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
        case 'read': {
          const row = await dbGet(
            `SELECT e.value, e.confidence, e.ttl, e.created_at, e.expires_at, e.evidence_summary, p.sources
             FROM memory_entries e
             LEFT JOIN memory_provenance p ON e.key = p.key
             WHERE e.key = ? AND e.expires_at > strftime('%s','now')`,
            key
          );

          if (!row) {
            return {
              result: {
                success: false,
                message: `Key ${key} not found or expired`
              }
            };
          }

          const provenance = row.sources ? JSON.parse(row.sources) : undefined;
          const value = JSON.parse(row.value);
          const expiresAtIso = new Date(row.expires_at * 1000).toISOString();
          const evidenceSummary = row.evidence_summary ? JSON.parse(row.evidence_summary) : undefined;

          return {
            result: {
              success: true,
              entry: {
                key,
                value,
                provenance,
                confidence: row.confidence,
                ttl: row.ttl,
                timestamp: row.created_at,
                expires_at: expiresAtIso,
                evidence_summary: evidenceSummary
              }
            }
          };
        }

        case 'write': {
          if (typeof args.value === 'undefined') {
            return {
              result: {
                success: false,
                message: 'Memory write requires a value'
              }
            };
          }

          if (typeof args.confidence !== 'number' || args.confidence < 0.8) {
            return {
              result: {
                success: false,
                message: 'Memory write rejected: confidence must be >= 0.8'
              }
            };
          }

          const provenanceArray = ensureProvenance(args.provenance);
          const ttlInfo = parseTtl(args.ttl);
          const createdAt = new Date().toISOString();
          const evidenceSummary = args.evidence_summary
            ? JSON.stringify(args.evidence_summary)
            : null;

          try {
            await dbRun('BEGIN IMMEDIATE');
            await dbRun(
              `INSERT INTO memory_entries (key, value, confidence, ttl, created_at, expires_at, evidence_summary)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(key) DO UPDATE SET
                 value = excluded.value,
                 confidence = excluded.confidence,
                 ttl = excluded.ttl,
                 created_at = excluded.created_at,
                 expires_at = excluded.expires_at,
                 evidence_summary = excluded.evidence_summary`,
              key,
              JSON.stringify(args.value),
              args.confidence,
              ttlInfo.ttl,
              createdAt,
              ttlInfo.expiresEpoch,
              evidenceSummary
            );
            await dbRun(
              `INSERT INTO memory_provenance (key, sources)
               VALUES (?, ?)
               ON CONFLICT(key) DO UPDATE SET sources = excluded.sources`,
              key,
              JSON.stringify(provenanceArray)
            );
            await dbRun('COMMIT');
          } catch (error: any) {
            await dbRun('ROLLBACK').catch(() => undefined);
            throw error;
          }

          return {
            result: {
              success: true,
              entry: {
                key,
                value: args.value,
                provenance: provenanceArray,
                confidence: args.confidence,
                ttl: ttlInfo.ttl,
                timestamp: createdAt,
                expires_at: new Date(ttlInfo.expiresEpoch * 1000).toISOString(),
                evidence_summary: args.evidence_summary
              }
            }
          };
        }

        case 'forget':
          await dbRun('DELETE FROM memory_entries WHERE key = ?', key);
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
