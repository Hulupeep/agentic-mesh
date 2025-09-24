/**
 * HTTP ToolSpec shim - uniform interface for all tools
 */
import express from 'express';

export interface ToolSpec {
  name: string;
  description?: string;
  io: {
    input: any;
    output: any;
  };
  constraints?: {
    input_tokens_max?: number;
    latency_p50_ms?: number;
    cost_per_call_usd?: number;
    rate_limit_qps?: number;
    side_effects?: boolean;
  };
  provenance?: {
    attribution_required?: boolean;
  };
  quality?: {
    freshness_window?: string;
    coverage_tags?: string[];
  };
  policy?: {
    deny_if?: string[];
  };
}

export interface ToolResult {
  result: any;
  error?: string;
}

export interface ToolHandler {
  invoke: (args: any) => Promise<ToolResult>;
  spec: ToolSpec;
}

export function createToolServer(tools: Record<string, ToolHandler>, port: number): void {
  const app = express();
  app.use(express.json());

  // Endpoint to get all tool specs
  app.get('/specs', (req, res) => {
    const specs = Object.keys(tools).map(name => tools[name].spec);
    res.json(specs);
  });

  // Endpoint to get a specific tool spec
  app.get('/spec/:name', (req, res) => {
    const toolName = req.params.name;
    const tool = tools[toolName];

    if (!tool) {
      res.status(404).json({ error: `Tool ${toolName} not found` });
      return;
    }
    
    res.json(tool.spec);
  });

  // Endpoint to invoke a tool
  app.post('/invoke/:name', async (req, res) => {
    const toolName = req.params.name;
    const tool = tools[toolName];
    
    if (!tool) {
      res.status(404).json({ error: `Tool ${toolName} not found` });
      return;
    }

    try {
      const result = await tool.invoke(req.body.args || {});
      res.json(result);
    } catch (error: any) {
      res.status(500).json({ error: error.message || 'Tool invocation failed' });
    }
  });

  app.listen(port, () => {
    console.log(`Tool server running on port ${port}`);
    console.log(`Available tools: ${Object.keys(tools).join(', ')}`);
  });
}