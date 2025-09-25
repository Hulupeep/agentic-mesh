"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.validateTrace = exports.validateMemory = exports.validateEvidence = exports.validatePlan = exports.validateToolSpec = exports.TraceSchema = exports.MemorySchema = exports.EvidenceSchema = exports.PlanSchema = exports.ToolSpecSchema = void 0;
const zod_1 = require("zod");
// ToolSpec Schema
exports.ToolSpecSchema = zod_1.z.object({
    name: zod_1.z.string(),
    description: zod_1.z.string().optional(),
    io: zod_1.z.object({
        input: zod_1.z.record(zod_1.z.any()),
        output: zod_1.z.record(zod_1.z.any()),
    }),
    capabilities: zod_1.z.array(zod_1.z.string()).optional(),
    constraints: zod_1.z.object({
        input_tokens_max: zod_1.z.number().int().nonnegative().optional(),
        latency_p50_ms: zod_1.z.number().int().nonnegative().optional(),
        cost_per_call_usd: zod_1.z.number().nonnegative().optional(),
        rate_limit_qps: zod_1.z.number().int().nonnegative().optional(),
        side_effects: zod_1.z.boolean().optional(),
    }).optional(),
    provenance: zod_1.z.object({
        attribution_required: zod_1.z.boolean().optional(),
    }).optional(),
    quality: zod_1.z.object({
        freshness_window: zod_1.z.string().regex(/^P([0-9]+Y)?([0-9]+M)?([0-9]+D)?(T([0-9]+H)?([0-9]+M)?([0-9]+S)?)?$/).optional(),
        coverage_tags: zod_1.z.array(zod_1.z.string()).optional(),
    }).optional(),
    policy: zod_1.z.object({
        deny_if: zod_1.z.array(zod_1.z.string()).optional(),
    }).optional(),
});
const toolRequiredOps = new Set(['call', 'map', 'reduce', 'verify', 'mem.read', 'mem.write', 'retry']);
const PlanNodeSchema = zod_1.z.object({
    id: zod_1.z.string(),
    op: zod_1.z.enum(['call', 'map', 'reduce', 'branch', 'assert', 'spawn', 'mem.read', 'mem.write', 'verify', 'retry']),
    tool: zod_1.z.string().optional(),
    capability: zod_1.z.string().optional(),
    args: zod_1.z.record(zod_1.z.any()).optional(),
    bind: zod_1.z.record(zod_1.z.string()).optional(),
    out: zod_1.z.record(zod_1.z.string()).optional(),
}).superRefine((node, ctx) => {
    if (toolRequiredOps.has(node.op) && !node.tool && !node.capability) {
        ctx.addIssue({
            code: zod_1.z.ZodIssueCode.custom,
            message: `Node ${node.id} requires either a tool or capability`,
        });
    }
});
// Plan Schema
exports.PlanSchema = zod_1.z.object({
    signals: zod_1.z.object({
        latency_budget_ms: zod_1.z.number().int().nonnegative().optional(),
        cost_cap_usd: zod_1.z.number().nonnegative().optional(),
        risk: zod_1.z.number().min(0).max(1).optional(),
    }).optional(),
    nodes: zod_1.z.array(PlanNodeSchema),
    edges: zod_1.z.array(zod_1.z.object({
        from: zod_1.z.string(),
        to: zod_1.z.string(),
    })).optional(),
    stop_conditions: zod_1.z.object({
        max_nodes: zod_1.z.number().int().nonnegative().optional(),
        min_confidence: zod_1.z.number().min(0).max(1).optional(),
    }).optional(),
});
// Evidence Schema
exports.EvidenceSchema = zod_1.z.object({
    claims: zod_1.z.array(zod_1.z.string()).optional(),
    supports: zod_1.z.array(zod_1.z.object({
        claim_id: zod_1.z.string(),
        source: zod_1.z.string(),
        confidence: zod_1.z.number().min(0).max(1),
        explanation: zod_1.z.string().optional(),
    })).optional(),
    contradicts: zod_1.z.array(zod_1.z.object({
        claim_id: zod_1.z.string(),
        source: zod_1.z.string(),
        confidence: zod_1.z.number().min(0).max(1),
        explanation: zod_1.z.string().optional(),
    })).optional(),
    verdicts: zod_1.z.array(zod_1.z.object({
        claim_id: zod_1.z.string(),
        verdict: zod_1.z.enum(['supported', 'contradicted', 'neutral']),
        confidence: zod_1.z.number().min(0).max(1),
        needs_citation: zod_1.z.boolean(),
    })).optional(),
});
// Memory Schema
exports.MemorySchema = zod_1.z.object({
    key: zod_1.z.string(),
    value: zod_1.z.record(zod_1.z.any()),
    provenance: zod_1.z.array(zod_1.z.string()).optional(),
    confidence: zod_1.z.number().min(0).max(1).optional(),
    ttl: zod_1.z.string().regex(/^P([0-9]+Y)?([0-9]+M)?([0-9]+D)?(T([0-9]+H)?([0-9]+M)?([0-9]+S)?)?$/).optional(),
    timestamp: zod_1.z.string().datetime(),
});
// Trace Schema
exports.TraceSchema = zod_1.z.object({
    plan_id: zod_1.z.string(),
    step_id: zod_1.z.string(),
    ts: zod_1.z.string().datetime(),
    event_type: zod_1.z.enum(['step_start', 'step_end', 'tool_invoke', 'constraint_check', 'policy_violation', 'evidence_check', 'memory_op', 'capability_route', 'plan_optimizer']),
    cost_usd: zod_1.z.number().nonnegative().optional(),
    tokens_in: zod_1.z.number().int().nonnegative().optional(),
    tokens_out: zod_1.z.number().int().nonnegative().optional(),
    citations: zod_1.z.array(zod_1.z.string()).optional(),
    signature: zod_1.z.string().optional(),
    data: zod_1.z.record(zod_1.z.any()).optional(),
});
// Validation functions
const validateToolSpec = (json) => exports.ToolSpecSchema.parse(json);
exports.validateToolSpec = validateToolSpec;
const validatePlan = (json) => exports.PlanSchema.parse(json);
exports.validatePlan = validatePlan;
const validateEvidence = (json) => exports.EvidenceSchema.parse(json);
exports.validateEvidence = validateEvidence;
const validateMemory = (json) => exports.MemorySchema.parse(json);
exports.validateMemory = validateMemory;
const validateTrace = (json) => exports.TraceSchema.parse(json);
exports.validateTrace = validateTrace;
//# sourceMappingURL=types.js.map
