"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.postgresAdapter = void 0;
exports.postgresAdapter = {
    spec: {
        name: 'search.postgres',
        description: 'PostgreSQL adapter with full-text search (stub implementation)',
        io: {
            input: {
                type: 'object',
                properties: {
                    table: { type: 'string' },
                    query: { type: 'string' },
                    filters: { type: 'object' },
                    limit: { type: 'number' }
                },
                required: ['table', 'query']
            },
            output: {
                type: 'object',
                properties: {
                    results: {
                        type: 'array',
                        items: { type: 'object' }
                    },
                    count: { type: 'number' }
                }
            }
        }
    },
    invoke: async (args) => {
        // Stub implementation - in real implementation, this would call PostgreSQL
        console.log('PostgreSQL adapter called with:', args);
        // Return mock data
        return {
            result: {
                results: [],
                count: 0
            }
        };
    }
};
//# sourceMappingURL=postgres.js.map