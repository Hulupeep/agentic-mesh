"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/**
 * Server to start all adapter tools
 */
const toolshim_1 = require("./common/toolshim");
const doc_search_local_1 = require("./tools/doc.search.local");
const ground_verify_1 = require("./tools/ground.verify");
const mesh_mem_sqlite_1 = require("./tools/mesh.mem.sqlite");
const elasticsearch_1 = require("./adapters/elasticsearch");
const postgres_1 = require("./adapters/postgres");
const mcp_1 = require("./adapters/mcp");
// Define tools and their ports
const tools = {
    'doc.search.local': doc_search_local_1.docSearchLocal,
    'ground.verify': ground_verify_1.groundVerify,
    'mesh.mem.sqlite': mesh_mem_sqlite_1.meshMemSqlite,
    'search.elasticsearch': elasticsearch_1.elasticsearchAdapter,
    'search.postgres': postgres_1.postgresAdapter,
    'bridge.mcp': mcp_1.mcpAdapter
};
// Port assignments
const ports = {
    'doc.search.local': 7401,
    'ground.verify': 7402,
    'mesh.mem.sqlite': 7403,
    'search.elasticsearch': 7404,
    'search.postgres': 7405,
    'bridge.mcp': 7406
};
// Start servers for each tool
Object.keys(tools).forEach((toolName) => {
    const tool = tools[toolName];
    const port = ports[toolName];
    console.log(`Starting ${toolName} on port ${port}...`);
    // Create a separate server for each tool
    const singleTool = { [toolName]: tool };
    (0, toolshim_1.createToolServer)(singleTool, port);
});
//# sourceMappingURL=server.js.map