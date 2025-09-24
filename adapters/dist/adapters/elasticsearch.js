"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.elasticsearchAdapter = void 0;
exports.elasticsearchAdapter = {
    spec: {
        name: 'search.elasticsearch',
        description: 'Elasticsearch adapter (stub implementation)',
        io: {
            input: {
                type: 'object',
                properties: {
                    index: { type: 'string' },
                    query: { type: 'object' },
                    size: { type: 'number' }
                },
                required: ['index', 'query']
            },
            output: {
                type: 'object',
                properties: {
                    hits: {
                        type: 'array',
                        items: { type: 'object' }
                    },
                    total: { type: 'number' }
                }
            }
        }
    },
    invoke: async (args) => {
        // Stub implementation - in real implementation, this would call Elasticsearch API
        console.log('Elasticsearch adapter called with:', args);
        // Return mock data
        return {
            result: {
                hits: [],
                total: 0
            }
        };
    }
};
//# sourceMappingURL=elasticsearch.js.map