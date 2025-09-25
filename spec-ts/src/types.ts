import { z } from 'zod';

// ToolSpec Schema
export const ToolSpecSchema = z.object({
  name: z.string(),
  description: z.string().optional(),
  io: z.object({
    input: z.record(z.any()),
    output: z.record(z.any()),
  }),
  capabilities: z.array(z.string()).optional(),
  constraints: z.object({
    input_tokens_max: z.number().int().nonnegative().optional(),
    latency_p50_ms: z.number().int().nonnegative().optional(),
    cost_per_call_usd: z.number().nonnegative().optional(),
    rate_limit_qps: z.number().int().nonnegative().optional(),
    side_effects: z.boolean().optional(),
  }).optional(),
  provenance: z.object({
    attribution_required: z.boolean().optional(),
  }).optional(),
  quality: z.object({
    freshness_window: z.string().regex(/^P([0-9]+Y)?([0-9]+M)?([0-9]+D)?(T([0-9]+H)?([0-9]+M)?([0-9]+S)?)?$/).optional(),
    coverage_tags: z.array(z.string()).optional(),
  }).optional(),
  policy: z.object({
    deny_if: z.array(z.string()).optional(),
  }).optional(),
});

// Plan Schema
export const PlanSchema = z.object({
  signals: z.object({
    latency_budget_ms: z.number().int().nonnegative().optional(),
    cost_cap_usd: z.number().nonnegative().optional(),
    risk: z.number().min(0).max(1).optional(),
  }).optional(),
});

const toolRequiredOps = new Set([ 'call', 'map', 'reduce', 'verify', 'mem.read', 'mem.write', 'retry' ]);

const PlanNodeSchema = z.object({
    id: z.string(),
    op: z.enum(['call', 'map', 'reduce', 'branch', 'assert', 'spawn', 'mem.read', 'mem.write', 'verify', 'retry']),
    tool: z.string().optional(),
    capability: z.string().optional(),
    args: z.record(z.any()).optional(),
    bind: z.record(z.string()).optional(),
    out: z.record(z.string()).optional(),
}).superRefine((node, ctx) => {
  if (toolRequiredOps.has(node.op) && !node.tool && !node.capability) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: `Node ${node.id} requires either a tool or capability`,
    });
  }
});

export const PlanSchema = z.object({
  signals: z.object({
    latency_budget_ms: z.number().int().nonnegative().optional(),
    cost_cap_usd: z.number().nonnegative().optional(),
    risk: z.number().min(0).max(1).optional(),
  }).optional(),
  nodes: z.array(PlanNodeSchema),
  edges: z.array(z.object({
    from: z.string(),
    to: z.string(),
  })).optional(),
  stop_conditions: z.object({
    max_nodes: z.number().int().nonnegative().optional(),
    min_confidence: z.number().min(0).max(1).optional(),
  }).optional(),
});

// Evidence Schema
export const EvidenceSchema = z.object({
  claims: z.array(z.string()).optional(),
  supports: z.array(z.object({
    claim_id: z.string(),
    source: z.string(),
    confidence: z.number().min(0).max(1),
    explanation: z.string().optional(),
  })).optional(),
  contradicts: z.array(z.object({
    claim_id: z.string(),
    source: z.string(),
    confidence: z.number().min(0).max(1),
    explanation: z.string().optional(),
  })).optional(),
  verdicts: z.array(z.object({
    claim_id: z.string(),
    verdict: z.enum(['supported', 'contradicted', 'neutral']),
    confidence: z.number().min(0).max(1),
    needs_citation: z.boolean(),
  })).optional(),
});

// Memory Schema
export const MemorySchema = z.object({
  key: z.string(),
  value: z.record(z.any()),
  provenance: z.array(z.string()).optional(),
  confidence: z.number().min(0).max(1).optional(),
  ttl: z.string().regex(/^P([0-9]+Y)?([0-9]+M)?([0-9]+D)?(T([0-9]+H)?([0-9]+M)?([0-9]+S)?)?$/).optional(),
  timestamp: z.string().datetime(),
});

// Trace Schema
export const TraceSchema = z.object({
  plan_id: z.string(),
  step_id: z.string(),
  ts: z.string().datetime(),
  event_type: z.enum(['step_start', 'step_end', 'tool_invoke', 'constraint_check', 'policy_violation', 'evidence_check', 'memory_op', 'capability_route', 'plan_optimizer']),
  cost_usd: z.number().nonnegative().optional(),
  tokens_in: z.number().int().nonnegative().optional(),
  tokens_out: z.number().int().nonnegative().optional(),
  citations: z.array(z.string()).optional(),
  signature: z.string().optional(),
  data: z.record(z.any()).optional(),
});

// Type exports
export type ToolSpec = z.infer<typeof ToolSpecSchema>;
export type Plan = z.infer<typeof PlanSchema>;
export type Evidence = z.infer<typeof EvidenceSchema>;
export type Memory = z.infer<typeof MemorySchema>;
export type Trace = z.infer<typeof TraceSchema>;

// Validation functions
export const validateToolSpec = (json: unknown): ToolSpec => ToolSpecSchema.parse(json);
export const validatePlan = (json: unknown): Plan => PlanSchema.parse(json);
export const validateEvidence = (json: unknown): Evidence => EvidenceSchema.parse(json);
export const validateMemory = (json: unknown): Memory => MemorySchema.parse(json);
export const validateTrace = (json: unknown): Trace => TraceSchema.parse(json);
