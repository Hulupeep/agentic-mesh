# AMP Technical Guide for LLM Executors

## Canonical Concepts
- **Plan IR**: Declarative JSON graph. Mandatory fields: `signals` (latency, cost, risk), `nodes` (id, op, tool/capability, args, bind/out), `edges`, `stop_conditions` (max_nodes, min_confidence). Validation: `Plan::validate` + custom checks (tool existence, output bindings, evidence gates).
- **ToolSpec ABI**: HTTP envelope exposing `/spec` (IO schema, constraints, provenance, policy) and `/invoke`. Each tool may wrap LLMs, scripts, services. Contracts define budgets and policy hints.
- **ExecutionContext**: Runtime state (variables, ToolSpecs, tool URLs, signals, trace events, completed/running nodes). Budget logic must subtract actual cost/latency tokens per tool.
- **Evidence System**: `ground.verify` (or custom) to validate claims. Evidence structures: claims, supports, contradicts, verdicts with confidence; stored in memory with provenance.
- **Memory Store**: Key-value with provenance/confidence/TTL. Writes require confidence >= 0.8 or validated evidence. Reads must return full metadata.
- **Trace Layer**: Signed trace events (step_start, tool_invoke, step_end, etc.) enabling replay/audit.

## Primary Operations
- `call`, `map`, `reduce`, `branch`, `assert`, `spawn`, `mem.read`, `mem.write`, `verify`, `retry`. Each must respect ToolSpec contracts and plan constraints.
- Argument resolution: convert `$var` or JSONPath-like references into structured arguments from context.
- Budget enforcement: compare cumulative cost/latency/token usage against `signals`; halt on exceed.

## Integration Flow (LLM-Friendly)
1. Load ToolSpec registry (path or service). Cache names, IO schemas, constraints.
2. Accept intent (natural language or structured). If raw prompt, run Planner Helper to generate Plan IR.
3. Validate plan; patch missing budgets/evidence nodes.
4. Populate ExecutionContext (initial variables from inputs/memory, tool URLs, ToolSpecs).
5. Execute nodes in DAG order. Handle parallelisation where edges permit. Use `retry` for failure tolerance.
6. After each node: update variables, subtract budgets, append trace, enforce policy (provenance, confidence, deny rules).
7. On completion: write outputs to memory if requested, package evidence bundle, sign traces, expose replay bundle.

## Adoption Checklist
- Wrap every external capability as ToolSpec. Ensure `/spec` returns accurate constraints/policies.
- Externalise tool registry (JSON, DB, discovery service). Hard-coded URLs are anti-pattern.
- Ensure Planner Helper (or equivalent) accessible to translate intents into plans. Reference `plannerhelper.md` for PRD.
- Implement end-to-end tests spanning plan execution (use `examples/plan.refund.json` as template). Extend with domain-specific fixtures.
- Fix known gaps per roadmap in [`fix1.md`](fix1.md): tool invocation mismatch, JSONPath resolution, memory metadata, dynamic discovery, budget enforcement, optimisation, replay bundles.

## Use Case Templates (Domain Mapping)
- **Healthcare Diagnostic Workflow**: nodes -> `mem.read` patient history → `call` doc search → `call` reasoning tool → `verify` guidelines → `call` treatment recommender → `mem.write` high-confidence plan. Budgets enforce latency (real-time triage) and cost (API charges). Evidence stored for compliance.
- **Financial Underwriting**: `call` KYC service → `call` risk model → `assert` policy thresholds → `verify` supporting documents → `mem.write` decision + provenance. Trace required for regulators.
- **Incident Response**: `call` alert triage → `branch` severity → `call` remediation script → `verify` rollback success → `mem.write` postmortem notes. Cost/token budgets manage compute spend.
- **Robotics Inspection**: `spawn` parallel sensor analyses → `verify` anomaly claims → `assert` safety thresholds → `mem.write` inspection log. ToolSpecs encode physical constraints.
- **Content Localization**: `map` translator tool over locales → `verify` legal compliance → `reduce` summary → `mem.write` release notes → `call` publish API.

## Industry Alignment
- Regulated (healthcare, finance, energy) require audit-ready traces and provenance; AMP’s evidence/memory solves compliance.
- Safety-critical (aerospace, autonomous robotics) benefit from deterministic plan execution under defined constraints.
- Human-centric services (education, neurodiversity support) leverage memory with confidence to preserve accommodations.
- Industrial automation (manufacturing, logistics) uses budgets and policy enforcement to avoid downtime and hazards.
- Public sector/emergency response: multi-agency coordination with transparent evidence, controlled by plan stop conditions.

## Improvement Hooks for LLM Developers
- **Planner Helper Enhancements**: improve schema understanding, tool selection heuristics, repair prompts, integration with memory context.
- **Constraint Reasoning**: predict tool cost/latency from ToolSpecs to optimise plan layout (parallelization, ordering).
- **Evidence reasoning**: incorporate structured evaluation (contradiction detection, multi-source weighting) to raise confidence accuracy.
- **Memory analytics**: build ToolSpecs for summarizing memory with provenance filters, feeding insights back into plans.
- **Policy enforcement**: extend PolicyEngine to evaluate narrative outputs for required citations/formatting.

## Validation & Testing Framework
- Use harness specified in `plannerhelper.md` for plan generation; extend to end-to-end plan execution tests under `kernel/tests`.
- Mock ToolSpecs for deterministic testing (simulate latency/cost usage, evidence responses).
- Ensure CI runs `Plan::validate`, dataset of canonical plans, and regression tests for budget enforcement.

## Observability & Operations
- Capture logs for each `/invoke` request/response (excluding sensitive data). Tag with plan ID, node ID.
- Monitor budget breaches, policy violations, evidence failures; surface to ops dashboards.
- Expose replay bundles (plan + ToolSpecs + traces + relevant memory slice) for audits; ensure `create_bundle` API returns tar.gz.
- Instrument Planner Helper and kernel with metrics (latency, retries, validation failures) for continuous improvement.

## Extension Patterns
- Add capability-based routing (plans specify capability; scheduler selects ToolSpec meeting constraints).
- Implement plan optimizer component to reorder/parallelize nodes based on budgets and dependencies.
- Support streaming updates via trace endpoint for live monitoring.
- Integrate with external governance tools (model risk, compliance) via new ToolSpecs or policy hooks.

## Migration Playbook
1. Inventory existing agent flows (LangChain graphs, Claude Flow DAGs).
2. Define ToolSpecs for each action (IO, cost, latency, policy, provenance requirements).
3. Use Planner Helper to draft initial plan; align with domain constraints.
4. Validate with harness; adjust budgets, evidence nodes, memory interactions.
5. Run in shadow mode (execute plan while legacy system runs) to compare outputs.
6. Promote to production once deterministic behaviour confirmed; retire ad-hoc orchestration.

## Reference Roadmap
- Follow detailed phased roadmap in [`fix1.md`](fix1.md) for closing current repo gaps: foundational fixes, marketplace, evidence-first enforcement, orchestration intelligence, memory analytics, production polish.

