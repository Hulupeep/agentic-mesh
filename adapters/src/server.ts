/**
 * Server to start all adapter tools
 */
import { createToolServer } from './common/toolshim';
import { docSearchLocal } from './tools/doc.search.local';
import { groundVerify } from './tools/ground.verify';
import { meshMemSqlite } from './tools/mesh.mem.sqlite';
import { meshMemAnalytics } from './tools/mesh.mem.analytics';
import { elasticsearchAdapter } from './adapters/elasticsearch';
import { postgresAdapter } from './adapters/postgres';
import { mcpAdapter } from './adapters/mcp';

// Define tools and their ports
const tools = {
  'doc.search.local': docSearchLocal,
  'ground.verify': groundVerify,
  'mesh.mem.sqlite': meshMemSqlite,
  'mesh.mem.analytics': meshMemAnalytics,
  'search.elasticsearch': elasticsearchAdapter,
  'search.postgres': postgresAdapter,
  'bridge.mcp': mcpAdapter
};

// Port assignments
const ports = {
  'doc.search.local': 7401,
  'ground.verify': 7402,
  'mesh.mem.sqlite': 7403,
  'mesh.mem.analytics': 7407,
  'search.elasticsearch': 7404,
  'search.postgres': 7405,
  'bridge.mcp': 7406
};

// Start servers for each tool
Object.keys(tools).forEach((toolName: string) => {
  const tool = (tools as any)[toolName];
  const port = (ports as any)[toolName];
  
  console.log(`Starting ${toolName} on port ${port}...`);
  
  // Create a separate server for each tool
  const singleTool = { [toolName]: tool };
  createToolServer(singleTool, port);
});
