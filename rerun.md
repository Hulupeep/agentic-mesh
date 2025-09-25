 

You are an expert Rust developer continuing work on the Agentic Mesh Protocol (AMP), an orchestration layer for AI agents. Your previous session was interrupted by context window limits. Your task is to pick up exactly where the last session left off.

All necessary context is in the provided files.

### 1. Project Mission & Core Concepts

AMP makes AI workflows explicit, auditable, and reliable. The key concepts are:
* **Plans as Code**: Workflows are declarative JSON, not imperative scripts.
* **ToolSpec ABI**: Tools have strict contracts for I/O, cost, and latency.
* **Evidence & Memory**: Outputs are verified, and trusted facts persist with provenance.
* **Budget Enforcement**: Plans execute within strict cost and latency guardrails.
* **Self-Evolution**: The long-term goal is for platforms to adapt based on signals, as detailed in `prd_self_evovolving.md`.

### 2. Current Status & Interruption Point

We have successfully completed Phases 0 through 3 of our roadmap (`fix1.md` and `tickets.md`). This includes:
* Fixing core tool invocation and argument resolution.
* Implementing a dynamic tool registry that loads from `config/tools.json` and fetches `ToolSpec`s.
* Enhancing the evidence system to produce detailed verification summaries.

**You were interrupted mid-way through implementing Phase 4: Constraint & Policy Enforcement.**

The last set of actions recorded in `transcript.md` involved:
1.  **Adding telemetry fields** (`total_latency_ms`, `total_cost_usd`, `total_tokens`) to `ExecutionContext`.
2.  Creating a `record_tool_usage` method to track the cost/latency of each tool call.
3.  Implementing `check_budget_overrun` to halt execution if a plan's signals (`cost_cap_usd`, `latency_budget_ms`) are exceeded.
4.  Instrumenting `execute_call`, `execute_map`, `execute_mem_read/write`, `execute_verify`, and `execute_retry` to time their execution and call `record_tool_usage`.
5.  Adding a final `budget_summary` trace event at the end of a plan's execution.
6.  Extending the `PolicyEngine` to parse this `budget_summary` trace and create violations if budgets were breached.

The work is likely incomplete and may have compilation errors.

### 3. Key Reference Documents

* **`tickets.md`**: This is your primary task list. We are focused on the tickets in "Phase 4".
* **`fix1_execution_plan.md`**: A more detailed view of the roadmap progress.
* **`transcript.md`**: The log of the previous session. Refer to the end of this file to see the exact code changes that were being made before the context limit was hit.
* **`prd_self_evovolving.md`**: The product vision that motivates features like budget enforcement.

### 4. Your Immediate Action Plan

1.  **Verify the state of the code.** Review the changes made at the end of `transcript.md`, focusing on `kernel/src/internal/exec/scheduler.rs` and `kernel/src/internal/policy/policy.rs`.
2.  **Run `cargo test`** inside the `kernel/` directory to establish a baseline of current compilation errors and test failures.
3.  **Complete the implementation of all Phase 4 tickets** from `tickets.md`. This means:
    * Ensure telemetry instrumentation is robust across *all* tool-invoking operations.
    * Confirm that budget enforcement correctly halts a plan by returning a `BudgetExceeded` error.
    * Add a new integration test that uses a plan with a very low budget and asserts that execution fails with the correct error.
    * Ensure the `PolicyEngine` and trace outputs function as designed.
4.  Once all Phase 4 work is complete and all tests pass, **update the checkboxes** for Phase 4 in `tickets.md` and `fix1_execution_plan.md` to reflect the completed work.

Begin by running the test suite to assess the current state.
