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
export declare function createToolServer(tools: Record<string, ToolHandler>, port: number): void;
//# sourceMappingURL=toolshim.d.ts.map