# Fix Plan 1: Stabilize and Realize AMP's Promises

## Context
The Agentic Mesh Protocol (AMP) positions itself as a next-generation orchestration layer where plans are code, tools form a marketplace, evidence is currency, and memory is the moat. The current codebase partially sketches these ideas but key mechanics are missing or misaligned: the kernel cannot invoke adapter tools due to an endpoint mismatch, arguments only support flat `$var` substitutions, memory reads drop provenance and confidence, and tool discovery/budget enforcement are mostly stubs. Documentation and marketing overstate capabilities, so the first priority is to close the gap between claims and implementation.

This roadmap sequences work so that foundational fixes land before higher-level intelligence. Each phase delivers testable capabilities aimed directly at AMP's advertised differentiators.

## Phase 0 – Alignment & Safety Net
- Add an integration test that exercises tool invocation, evidence flow, and memory read/write using `examples/plan.refund.json`.
- Ensure CI enforces formatting, clippy, and adapter linting so regressions surface immediately.

## Phase 1 – Plans as Code (Baseline Functionality)
- Fix HTTP mismatches: either update the kernel client to hit `/invoke/:name` or expose `/invoke` endpoints in the adapters so calls succeed (`kernel/src/internal/tools/spec.rs`, `adapters/src/common/toolshim.ts`).
- Replace the string-only `$var` resolver with JSONPath-style traversal so nested values like `$ev1.verdicts[0].confidence` are usable (`kernel/src/internal/exec/scheduler.rs`).
- Extend `Plan::validate` to confirm that referenced tools exist in the registry and required outputs are bound to variables.

## Phase 2 – ToolSpec ABI & Marketplace
- Externalise tool definitions into a config or registry and populate `ExecutionContext.tool_urls` from that source instead of hard-coding (`kernel/src/internal/api.rs`).
- Fetch and cache ToolSpecs when executing a plan; use the cache as the canonical view of capabilities/constraints.
- Introduce a simple registry service or file-backed index with register/query endpoints to prepare for multi-host deployments.

## Phase 3 – Evidence as First-Class
- Return full memory rows (value, provenance, confidence, ttl, timestamp) from the SQLite adapter and parse them into `MemoryEntry` (`adapters/src/tools/mesh.mem.sqlite.ts`, `kernel/src/internal/mem/store.rs`).
- Expand the verifier to score multiple supports/contradictions and make failures explicit instead of generic communication errors.
- Enforce evidence thresholds before assertions or memory writes and attach summaries to trace events for auditability.

## Phase 4 – Constraint & Policy Enforcement
- Track actual latency, cost, and token usage per tool invocation; subtract from budgets and fail when limits are exceeded (`kernel/src/internal/exec/scheduler.rs`).
- Use ToolSpec constraint fields to vet tool choices and block calls that violate policy (`kernel/src/internal/exec/constraints.rs`).
- Extend `PolicyEngine` to flag budget breaches and missing provenance alongside existing warnings.

## Phase 5 – Orchestration Intelligence
- Allow plan nodes to specify a `capability`; choose the optimal tool at runtime based on cached specs and live budget telemetry.
- Add a pre-execution optimiser that parallelises independent nodes and reorders commutative steps when it reduces estimated latency or cost.
- Emit trace events capturing each scheduling decision so plan replays can follow or experiment with alternative strategies.

## Phase 6 – Memory as Moat
- Enforce TTL/provenance storage in SQLite (dedicated tables) and require evidence-backed writes.
- Build analytics routines (e.g., top confidences, contradiction hotspots) exposed via a new `mem.analytics` tool and feed insights back into plans.
- Support replayable bundles that package memory slices with trace signatures for downstream audits (`kernel/src/internal/api.rs`).

## Phase 7 – Production Polish
- Implement the replay bundle endpoint to emit a real tarball containing plan, specs, traces, and relevant memory snapshots.
- Harden adapters with health checks, structured logging, and configurable paths; provide Docker overlays for dev/prod parity.
- Update README and docs to describe the true workflow, registry setup, evidence rules, and architecture; add troubleshooting guidance.



## Phase 8 – Selfevolving / Self-healing Orchestration

- [ ] review prd_self_evolve.md and build out a detailed tdd based plan to add this feature as a primitive
- [ ] 

## Phase 9 – Verified answers use case

- [ ] the goal is to show amp used in existing systems as a way to provide verified answers. See verified_answers_usecase.md  great for regulated envs

## Immediate Next Steps
1. Ratify Phase 0 scope and define success metrics (e.g., end-to-end plan passes).
2. File tickets per task and assign owners; begin with the tool invocation fix because it unblocks all downstream testing.
