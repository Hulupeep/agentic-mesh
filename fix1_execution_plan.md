# Execution Plan for `fix1.md`

This plan tracks every roadmap item from `fix1.md`, mapping them to concrete actions, cross-linking to supporting references (e.g., `llm_technical_guide.md`), and providing phase explainers.

## Phase 0 – Alignment & Safety Net
- [x] Establish baseline integration test covering tool invocation, evidence flow, and memory ops (`kernel/tests/integration_tests.rs`).
- [ ] Add CI gates for formatting (`rustfmt`, `pnpm lint`), `cargo clippy`, and adapter linting.

**Explainer**: Per `llm_technical_guide.md`, reliable orchestration demands deterministic validation. Phase 0 provides automated guardrails so subsequent fixes can be verified continuously.

## Phase 1 – Plans as Code (Baseline Functionality)
- [x] Fix HTTP mismatch (`/invoke` vs `/invoke/:name`) in kernel ToolClient and adapter shim.
- [x] Implement JSONPath-style argument resolution for nested bindings.
- [x] Extend `Plan::validate` to ensure referenced tools exist and outputs are bound.

**Explainer**: Restores minimal viable execution so plans authored via Planner Helper or manual JSON reflect reality. Aligns with “Primary Operations” section in `llm_technical_guide.md`.

## Phase 2 – ToolSpec ABI & Marketplace
- [x] Externalise tool registry (config/service) and populate `ExecutionContext.tool_urls` dynamically.
- [x] Fetch/cache ToolSpecs during plan execution.
- [x] Design lightweight registry service (registration & discovery endpoints).

**Explainer**: Unlocks marketplace dynamics described in `industries.md` and `llm_technical_guide.md`, allowing capability-based orchestration and provider swaps without code changes.

## Phase 3 – Evidence as First-Class
- [x] Return full memory metadata (value, provenance, confidence, ttl, timestamp) from adapters; parse in kernel.
- [x] Enhance verifier to handle multiple supports/contradictions with explicit errors.
- [x] Enforce evidence thresholds before assertions/memory writes, storing summaries in traces.

**Explainer**: Delivers “evidence as currency,” enabling regulated industries (see `industries.md`) to trust outputs.
`PolicyEngine` now consumes verification summaries emitted by the scheduler and fails plans that lack evidence or fall below confidence thresholds, with unit and integration coverage in `kernel/tests/policy_tests.rs` and `kernel/tests/integration_tests.rs`.

## Phase 4 – Constraint & Policy Enforcement
- [x] Track actual cost/latency/tokens per node and enforce budgets.
- [x] Apply ToolSpec policy fields during execution (deny rules, attribution).
- [x] Extend `PolicyEngine` to flag budget violations and missing provenance.

**Explainer**: Telemetry now hydrates ToolSpecs automatically, records consumption via `record_tool_usage`, and aborts plans on `BudgetExceeded` (see `kernel/tests/integration_tests.rs:test_plan_fails_when_cost_budget_exceeded`). Policy-aware traces and enforcement block disallowed tool invocations (`deny_if`) and surface violations through `PolicyEngine`, exercised by new policy/unit tests.

## Phase 5 – Orchestration Intelligence
- [x] Implement capability-based tool selection.
- [x] Introduce plan optimiser (parallelisation, reordering).
- [x] Emit trace annotations capturing scheduling decisions.

**Explainer**: Scheduler now hydrates ToolSpec capabilities and selects tools dynamically (`kernel/tests/integration_tests.rs:test_capability_routing_selects_registered_tool`), emitting `capability_route` traces for audit. A deterministic optimiser ranks nodes via predicted cost/latency before execution, recording the decision tree in `plan_optimizer` traces and ensuring dependency-safe reordering.

## Phase 6 – Memory as Moat
- [x] Enforce TTL/provenance structure in memory store (DB schema updates).
- [x] Build analytics ToolSpec for insight extraction.
- [ ] Support replay bundles containing memory slices with trace signatures.

**Explainer**: Aligns with industries that depend on institutional knowledge (healthcare, education, robotics) to learn safely over time. `mesh.mem.sqlite` now enforces non-empty provenance, TTL validation, and evidence metadata end-to-end (kernel + adapter), while the new `mesh.mem.analytics` tool surfaces live summary/by-key insights exercised by scheduler and API integration tests.

## Phase 7 – Production Polish
- [ ] Implement `create_bundle` API to output tar.gz bundles (plan, specs, traces, memory diff).
- [ ] Harden adapters (health checks, logging, config) and provide Docker overlays.
- [ ] Update documentation (README, FAQ, developer guide) and expand test coverage.

**Explainer**: Prepares AMP for enterprise rollout—closing compliance and ops loops.



## Phase 8 – Selfevolving / Self-healing Orchestration

- [ ] review prd_self_evolving.md and build out a detailed tdd based plan to add this feature as a primitive

  ## Phase 9 – Verified answers use case
  
  - [ ] the goal is to show amp used in existing systems as a way to provide verified answers. See verified_answers_usecase.md  great for regulated envs



---
**Roadmap Reference**: Each checkbox maps directly to the roadmap in [`fix1.md`](fix1.md). For deeper technical execution details, refer to `llm_technical_guide.md` sections: “Primary Operations,” “Adoption Checklist,” “Improvement Hooks,” and “Migration Playbook`.
