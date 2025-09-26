# AMP Roadmap Tickets

This tracker mirrors `fix1.md` and the self-evolving PRD, breaking remaining milestones into actionable tickets. Status keys: ✅ done, 🔄 in progress, ⏳ todo.

## Phase 0 – Alignment & Safety Net
- ⏳ **P0-1 CI Guardrails** – Add rustfmt, clippy, adapter lint (pnpm) to CI pipeline; fail builds on formatting or lint errors.

## Phase 1 – Plans as Code (Baseline)
- ✅ **P1-1 Tool Invocation Fix**
- ✅ **P1-2 JSONPath Resolver**
- ✅ **P1-3 Plan Validation With Tool Registry**

## Phase 2 – ToolSpec ABI & Marketplace
- ✅ **P2-1 Dynamic Tool Registry** (config/env + API/CLI integration)
- ✅ **P2-2 ToolSpec Prefetch & Caching**
- ✅ **P2-3 Registry Service MVP** (register/list endpoints, test coverage)

## Phase 3 – Evidence as First-Class
- ✅ **P3-1 Memory Metadata Roundtrip**
- ✅ **P3-2 Enhanced Evidence Verifier** (per-claim summaries, support requirement)
- ✅ **P3-3 Evidence Summaries in Scheduler** (verify/assert/mem.write trace data)
- ✅ **P3-4 Evidence-Aware Policy Hooks** – Extend `PolicyEngine` to consume verification summaries and fail closed when evidence data is missing or below policy thresholds.

## Phase 4 – Constraint & Policy Enforcement
- ✅ **P4-1 Telemetry Instrumentation** – Track per-node cost, latency, and token usage within `ExecutionContext`; emit to traces.
- ✅ **P4-2 Budget Enforcement** – Subtract telemetry from plan signals and halt when budgets exceeded; expose failure reason.
- ✅ **P4-3 Tool Policy Enforcement** – Honor ToolSpec policy fields (`deny_if`, attribution) during execution and surface violations via `PolicyEngine`.
- ✅ **P4-4 Budget-aware Trace Outputs** – Include budget consumption snapshots in trace events and replay bundles.

## Phase 5 – Orchestration Intelligence
- ✅ **P5-1 Capability Routing** – Allow plan nodes to specify capability tags; scheduler selects optimal tool based on specs/constraints.
- ✅ **P5-2 Plan Optimizer** – Pre-execution optimiser for parallelisation and safe node reordering using cost/latency heuristics.
- ✅ **P5-3 Decision Trace Annotations** – Record optimizer/routing decisions with rationale in trace payloads.

## Phase 6 – Memory as Moat
- ✅ **P6-1 Memory Schema Enforcement** – Add TTL/provenance tables & migration; enforce at adapter + kernel level.
- ✅ **P6-2 Memory Analytics ToolSpec** – Provide analytics tooling (top contradicted claims, lifecycle reports) consumable by plans.
- ⏳ **P6-3 Replay Bundles with Memory** – Finish `/v1/replay/bundle` to include plan, ToolSpecs, traces, and relevant memory diffs.

## Phase 7 – Production Polish
- ⏳ **P7-1 Bundle API Completion** – Implement real tar.gz bundle assembly with signatures.
- ⏳ **P7-2 Adapter Hardening** – Health endpoints, structured logging, configuration hygiene, Docker overlays.
- ⏳ **P7-3 Documentation Refresh** – Update README/FAQ/LLM guide for new workflow, registry setup, evidence policies; add troubleshooting.
- ⏳ **P7-4 PRD Alignment** – Implement change proposal lifecycle (Observe→Diagnose→Propose) and validation harness stubs per PRD.

## Self-Evolving Loop (Cross-Cutting Initiatives)
- ⏳ **SE-1 Change Proposal Generator** – Use telemetry/evidence to auto-generate structured proposals with expected deltas, risk tiers, rollback rules.
- ⏳ **SE-2 Validation Harness** – Shadow/A-B infrastructure tied to proposals; programmable pass/fail gates.
- ⏳ **SE-3 Risk-Tiered Deployment** – Auto-deploy low-risk proposals, approval workflow for medium/high risk, including emergency stop.
- ⏳ **SE-4 Observability Dashboards** – Surface SLOs (quality, latency, cost), evolution history, and evidence health in one pane of glass.

---

## Current Focus
Next up: begin **P5-1 Capability Routing** to leverage the new telemetry stack for adaptive scheduling decisions.
