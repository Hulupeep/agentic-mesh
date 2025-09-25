import { z } from 'zod';
export declare const ToolSpecSchema: z.ZodObject<{
    name: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    io: z.ZodObject<{
        input: z.ZodRecord<z.ZodString, z.ZodAny>;
        output: z.ZodRecord<z.ZodString, z.ZodAny>;
    }, "strip", z.ZodTypeAny, {
        input: Record<string, any>;
        output: Record<string, any>;
    }, {
        input: Record<string, any>;
        output: Record<string, any>;
    }>;
    capabilities: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    constraints: z.ZodOptional<z.ZodObject<{
        input_tokens_max: z.ZodOptional<z.ZodNumber>;
        latency_p50_ms: z.ZodOptional<z.ZodNumber>;
        cost_per_call_usd: z.ZodOptional<z.ZodNumber>;
        rate_limit_qps: z.ZodOptional<z.ZodNumber>;
        side_effects: z.ZodOptional<z.ZodBoolean>;
    }, "strip", z.ZodTypeAny, {
        input_tokens_max?: number | undefined;
        latency_p50_ms?: number | undefined;
        cost_per_call_usd?: number | undefined;
        rate_limit_qps?: number | undefined;
        side_effects?: boolean | undefined;
    }, {
        input_tokens_max?: number | undefined;
        latency_p50_ms?: number | undefined;
        cost_per_call_usd?: number | undefined;
        rate_limit_qps?: number | undefined;
        side_effects?: boolean | undefined;
    }>>;
    provenance: z.ZodOptional<z.ZodObject<{
        attribution_required: z.ZodOptional<z.ZodBoolean>;
    }, "strip", z.ZodTypeAny, {
        attribution_required?: boolean | undefined;
    }, {
        attribution_required?: boolean | undefined;
    }>>;
    quality: z.ZodOptional<z.ZodObject<{
        freshness_window: z.ZodOptional<z.ZodString>;
        coverage_tags: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    }, "strip", z.ZodTypeAny, {
        freshness_window?: string | undefined;
        coverage_tags?: string[] | undefined;
    }, {
        freshness_window?: string | undefined;
        coverage_tags?: string[] | undefined;
    }>>;
    policy: z.ZodOptional<z.ZodObject<{
        deny_if: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    }, "strip", z.ZodTypeAny, {
        deny_if?: string[] | undefined;
    }, {
        deny_if?: string[] | undefined;
    }>>;
}, "strip", z.ZodTypeAny, {
    name: string;
    io: {
        input: Record<string, any>;
        output: Record<string, any>;
    };
    description?: string | undefined;
    capabilities?: string[] | undefined;
    constraints?: {
        input_tokens_max?: number | undefined;
        latency_p50_ms?: number | undefined;
        cost_per_call_usd?: number | undefined;
        rate_limit_qps?: number | undefined;
        side_effects?: boolean | undefined;
    } | undefined;
    provenance?: {
        attribution_required?: boolean | undefined;
    } | undefined;
    quality?: {
        freshness_window?: string | undefined;
        coverage_tags?: string[] | undefined;
    } | undefined;
    policy?: {
        deny_if?: string[] | undefined;
    } | undefined;
}, {
    name: string;
    io: {
        input: Record<string, any>;
        output: Record<string, any>;
    };
    description?: string | undefined;
    capabilities?: string[] | undefined;
    constraints?: {
        input_tokens_max?: number | undefined;
        latency_p50_ms?: number | undefined;
        cost_per_call_usd?: number | undefined;
        rate_limit_qps?: number | undefined;
        side_effects?: boolean | undefined;
    } | undefined;
    provenance?: {
        attribution_required?: boolean | undefined;
    } | undefined;
    quality?: {
        freshness_window?: string | undefined;
        coverage_tags?: string[] | undefined;
    } | undefined;
    policy?: {
        deny_if?: string[] | undefined;
    } | undefined;
}>;
export declare const PlanSchema: z.ZodObject<{
    signals: z.ZodOptional<z.ZodObject<{
        latency_budget_ms: z.ZodOptional<z.ZodNumber>;
        cost_cap_usd: z.ZodOptional<z.ZodNumber>;
        risk: z.ZodOptional<z.ZodNumber>;
    }, "strip", z.ZodTypeAny, {
        latency_budget_ms?: number | undefined;
        cost_cap_usd?: number | undefined;
        risk?: number | undefined;
    }, {
        latency_budget_ms?: number | undefined;
        cost_cap_usd?: number | undefined;
        risk?: number | undefined;
    }>>;
    nodes: z.ZodArray<z.ZodObject<{
        id: z.ZodString;
        op: z.ZodEnum<["call", "map", "reduce", "branch", "assert", "spawn", "mem.read", "mem.write", "verify", "retry"]>;
        tool: z.ZodOptional<z.ZodString>;
        capability: z.ZodOptional<z.ZodString>;
        args: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
        bind: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodString>>;
        out: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodString>>;
    }, "strip", z.ZodTypeAny, {
        id: string;
        op: "map" | "reduce" | "call" | "branch" | "assert" | "spawn" | "mem.read" | "mem.write" | "verify" | "retry";
        tool?: string | undefined;
        capability?: string | undefined;
        args?: Record<string, any> | undefined;
        bind?: Record<string, string> | undefined;
        out?: Record<string, string> | undefined;
    }, {
        id: string;
        op: "map" | "reduce" | "call" | "branch" | "assert" | "spawn" | "mem.read" | "mem.write" | "verify" | "retry";
        tool?: string | undefined;
        capability?: string | undefined;
        args?: Record<string, any> | undefined;
        bind?: Record<string, string> | undefined;
        out?: Record<string, string> | undefined;
    }>, "many">;
    edges: z.ZodOptional<z.ZodArray<z.ZodObject<{
        from: z.ZodString;
        to: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        from: string;
        to: string;
    }, {
        from: string;
        to: string;
    }>, "many">>;
    stop_conditions: z.ZodOptional<z.ZodObject<{
        max_nodes: z.ZodOptional<z.ZodNumber>;
        min_confidence: z.ZodOptional<z.ZodNumber>;
    }, "strip", z.ZodTypeAny, {
        max_nodes?: number | undefined;
        min_confidence?: number | undefined;
    }, {
        max_nodes?: number | undefined;
        min_confidence?: number | undefined;
    }>>;
}, "strip", z.ZodTypeAny, {
    nodes: {
        id: string;
        op: "map" | "reduce" | "call" | "branch" | "assert" | "spawn" | "mem.read" | "mem.write" | "verify" | "retry";
        tool?: string | undefined;
        args?: Record<string, any> | undefined;
        bind?: Record<string, string> | undefined;
        out?: Record<string, string> | undefined;
    }[];
    signals?: {
        latency_budget_ms?: number | undefined;
        cost_cap_usd?: number | undefined;
        risk?: number | undefined;
    } | undefined;
    edges?: {
        from: string;
        to: string;
    }[] | undefined;
    stop_conditions?: {
        max_nodes?: number | undefined;
        min_confidence?: number | undefined;
    } | undefined;
}, {
    nodes: {
        id: string;
        op: "map" | "reduce" | "call" | "branch" | "assert" | "spawn" | "mem.read" | "mem.write" | "verify" | "retry";
        tool?: string | undefined;
        args?: Record<string, any> | undefined;
        bind?: Record<string, string> | undefined;
        out?: Record<string, string> | undefined;
    }[];
    signals?: {
        latency_budget_ms?: number | undefined;
        cost_cap_usd?: number | undefined;
        risk?: number | undefined;
    } | undefined;
    edges?: {
        from: string;
        to: string;
    }[] | undefined;
    stop_conditions?: {
        max_nodes?: number | undefined;
        min_confidence?: number | undefined;
    } | undefined;
}>;
export declare const EvidenceSchema: z.ZodObject<{
    claims: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    supports: z.ZodOptional<z.ZodArray<z.ZodObject<{
        claim_id: z.ZodString;
        source: z.ZodString;
        confidence: z.ZodNumber;
        explanation: z.ZodOptional<z.ZodString>;
    }, "strip", z.ZodTypeAny, {
        claim_id: string;
        source: string;
        confidence: number;
        explanation?: string | undefined;
    }, {
        claim_id: string;
        source: string;
        confidence: number;
        explanation?: string | undefined;
    }>, "many">>;
    contradicts: z.ZodOptional<z.ZodArray<z.ZodObject<{
        claim_id: z.ZodString;
        source: z.ZodString;
        confidence: z.ZodNumber;
        explanation: z.ZodOptional<z.ZodString>;
    }, "strip", z.ZodTypeAny, {
        claim_id: string;
        source: string;
        confidence: number;
        explanation?: string | undefined;
    }, {
        claim_id: string;
        source: string;
        confidence: number;
        explanation?: string | undefined;
    }>, "many">>;
    verdicts: z.ZodOptional<z.ZodArray<z.ZodObject<{
        claim_id: z.ZodString;
        verdict: z.ZodEnum<["supported", "contradicted", "neutral"]>;
        confidence: z.ZodNumber;
        needs_citation: z.ZodBoolean;
    }, "strip", z.ZodTypeAny, {
        claim_id: string;
        confidence: number;
        verdict: "supported" | "contradicted" | "neutral";
        needs_citation: boolean;
    }, {
        claim_id: string;
        confidence: number;
        verdict: "supported" | "contradicted" | "neutral";
        needs_citation: boolean;
    }>, "many">>;
}, "strip", z.ZodTypeAny, {
    claims?: string[] | undefined;
    supports?: {
        claim_id: string;
        source: string;
        confidence: number;
        explanation?: string | undefined;
    }[] | undefined;
    contradicts?: {
        claim_id: string;
        source: string;
        confidence: number;
        explanation?: string | undefined;
    }[] | undefined;
    verdicts?: {
        claim_id: string;
        confidence: number;
        verdict: "supported" | "contradicted" | "neutral";
        needs_citation: boolean;
    }[] | undefined;
}, {
    claims?: string[] | undefined;
    supports?: {
        claim_id: string;
        source: string;
        confidence: number;
        explanation?: string | undefined;
    }[] | undefined;
    contradicts?: {
        claim_id: string;
        source: string;
        confidence: number;
        explanation?: string | undefined;
    }[] | undefined;
    verdicts?: {
        claim_id: string;
        confidence: number;
        verdict: "supported" | "contradicted" | "neutral";
        needs_citation: boolean;
    }[] | undefined;
}>;
export declare const MemorySchema: z.ZodObject<{
    key: z.ZodString;
    value: z.ZodRecord<z.ZodString, z.ZodAny>;
    provenance: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    confidence: z.ZodOptional<z.ZodNumber>;
    ttl: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodString;
}, "strip", z.ZodTypeAny, {
    value: Record<string, any>;
    key: string;
    timestamp: string;
    provenance?: string[] | undefined;
    confidence?: number | undefined;
    ttl?: string | undefined;
}, {
    value: Record<string, any>;
    key: string;
    timestamp: string;
    provenance?: string[] | undefined;
    confidence?: number | undefined;
    ttl?: string | undefined;
}>;
export declare const TraceSchema: z.ZodObject<{
    plan_id: z.ZodString;
    step_id: z.ZodString;
    ts: z.ZodString;
    event_type: z.ZodEnum<["step_start", "step_end", "tool_invoke", "constraint_check", "policy_violation", "evidence_check", "memory_op", "capability_route", "plan_optimizer"]>;
    cost_usd: z.ZodOptional<z.ZodNumber>;
    tokens_in: z.ZodOptional<z.ZodNumber>;
    tokens_out: z.ZodOptional<z.ZodNumber>;
    citations: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    signature: z.ZodOptional<z.ZodString>;
    data: z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodAny>>;
}, "strip", z.ZodTypeAny, {
    plan_id: string;
    step_id: string;
    ts: string;
    event_type: "step_start" | "step_end" | "tool_invoke" | "constraint_check" | "policy_violation" | "evidence_check" | "memory_op";
    cost_usd?: number | undefined;
    tokens_in?: number | undefined;
    tokens_out?: number | undefined;
    citations?: string[] | undefined;
    signature?: string | undefined;
    data?: Record<string, any> | undefined;
}, {
    plan_id: string;
    step_id: string;
    ts: string;
    event_type: "step_start" | "step_end" | "tool_invoke" | "constraint_check" | "policy_violation" | "evidence_check" | "memory_op";
    cost_usd?: number | undefined;
    tokens_in?: number | undefined;
    tokens_out?: number | undefined;
    citations?: string[] | undefined;
    signature?: string | undefined;
    data?: Record<string, any> | undefined;
}>;
export type ToolSpec = z.infer<typeof ToolSpecSchema>;
export type Plan = z.infer<typeof PlanSchema>;
export type Evidence = z.infer<typeof EvidenceSchema>;
export type Memory = z.infer<typeof MemorySchema>;
export type Trace = z.infer<typeof TraceSchema>;
export declare const validateToolSpec: (json: unknown) => ToolSpec;
export declare const validatePlan: (json: unknown) => Plan;
export declare const validateEvidence: (json: unknown) => Evidence;
export declare const validateMemory: (json: unknown) => Memory;
export declare const validateTrace: (json: unknown) => Trace;
//# sourceMappingURL=types.d.ts.map
