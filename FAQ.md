# AMP FAQ

This FAQ distills common questions from product builders, PMs, and engineers comparing the Agentic Mesh Protocol (AMP) to familiar agentic frameworks like Claude Flow or LangChain.

## What is AMP’s positioning compared to other agent frameworks?
AMP is an orchestration platform that makes AI workflows explicit, audit-ready, and repeatable. Plans are authored as JSON blueprints, tools are registered with strict IO and policy contracts (ToolSpecs), evidence is collected and verified at each step, and shared memory persists high-confidence knowledge. Think of it as the mesh you put in front of improvisational LLM agents when you need determinism, traceability, and budget enforcement. For a quick overview (including non-code examples), see “Philosophy” and “Beyond Code Generation” in the [README](README.md).



## What’s the mental model for AMP versus Claude Flow or LangChain?
- **Claude Flow / similar**: you state a goal, the LLM improvises a team of helper agents, and you hope it converges. The orchestration is implicit in hidden state.
- **AMP**: you submit a declarative plan describing the entire workflow: nodes, dependencies, budgets, policy rules. Tools (which can wrap LLMs) expose explicit contracts via ToolSpecs. The kernel executes the plan deterministically, enforces constraints, logs evidence, and stores outputs in memory with provenance. Use an LLM upstream to draft the plan, but AMP is the trusted runtime that executes the signed blueprint.

## What does “agent” mean in these different contexts?
- **Plain definition**: an autonomous worker that can pursue a goal using tools or knowledge without step-by-step human instructions.
- **Claude Flow**: agents are personas the model dreams up on demand ("researcher", "critic"). They are implicit, untyped, and may vary across runs.
- **AMP**: agents are registered services described by ToolSpecs. A ToolSpec might front an LLM loop, a scripted API, or a microservice. Plans reference these agents explicitly, so the kernel knows exactly which agent runs, in what order, with which budgets and evidence requirements. There is no “magic spin-up”; agents are declared and governed. See the “Reframing ‘Agents’” table in the [README](README.md) for a side-by-side comparison.

## Why use AMP if Claude Flow can already “just build it”?
Claude Flow is fantastic for rapid ideation. The cost is that coordination logic, guardrails, and memory are emergent. Rerunning tomorrow may produce a different flow. When you need reproducibility—shared memory across runs, verifiable citations, budget guarantees—you wrap the Claude-generated workflow in a Plan IR and let AMP execute it. AMP becomes the inspector and replay engine, while Claude (or another LLM) is the creative planner.

### How do I migrate an existing “it works!” workflow into AMP?
1. **Capture the flow**: Export whatever intermediate representation you have—Claude Flow DAG, LangGraph, or even a textual runbook.
2. **Model ToolSpecs**: For every external call (APIs, LLM prompts, scripts), wrap it as a ToolSpec with explicit IO, budgets, and policy fields.
3. **Author the plan**: Translate the original sequence into Plan IR (handwritten or via Planner Helper), making dependencies and stop conditions explicit.
4. **Inject evidence/budgets**: Add verification nodes where humans previously eyeballed results, and set cost/latency caps based on production constraints.
5. **Dry-run & validate**: Run the plan in AMP’s test harness with tools mocked to ensure variable bindings and control flow match the original behaviour.
6. **Execute deterministically**: Once validated, use AMP’s kernel to replay the workflow. You now gain traces, provenance, and predictable behaviour while preserving the original intent.

## Can AMP still feel as fast as “give it a prompt and go”? 
Yes, by chaining them: prompt Claude Flow to produce an AMP Plan IR for your task (“build a neurodivergent-friendly ToDo app”). Review the plan, tweak budgets/policies, then run it through AMP. For future iterations, feed the existing plan, traces, and memory back into Claude to propose improvements, but keep the final plan explicit so AMP can execute it under guardrails every time.

## Is AMP only for writing code?
No. Plans can orchestrate research briefs, incident runbooks, content localization pipelines, compliance reviews, clinical decision support, or robotics procedures. The kernel doesn’t care whether a node emits source code, a policy memo, or a control signal—so long as the ToolSpec contract is satisfied. Examples are summarised in the README’s “Beyond Code Generation” section, with industry specifics in `industries.md`.

## How does memory analytics fit the “memory as moat” promise?
Memory writes now require explicit provenance, TTL, and confidence. The `mesh.mem.analytics` tool (exposed via the ToolSpec marketplace) lets plans query totals, expiring knowledge, or by-key summaries before deciding what to refresh. Plans can fan out to analytics nodes to decide whether to regenerate insights, flag contradictory entries, or prove to auditors that a fact was still valid when used.

## Does AMP spin up agents automatically?
No. AMP expects you to register agent endpoints ahead of time via ToolSpecs (for example, `doc.search.local`, `ground.verify`, `code.generate_go`). Plans orchestrate those predefined agents. If you want the spontaneity of Claude Flow, use it to author plans or create new ToolSpecs, then let AMP run them deterministically. AMP trades improvisation for predictability and compliance.

## How does compliance and evidence work in AMP?
Plans can mark steps with minimum confidence thresholds or required citations. The kernel validates evidence using the `ground.verify` tool (or any registered verifier), and memory writes can be blocked unless evidence confidence exceeds a threshold. Provenance is saved alongside values in memory. Traces record each step, cost, tokens, and signatures, so you can audit exactly why a conclusion was accepted.

## What does a real AMP workflow look like?
Consider shipping a neurodivergent-friendly ToDo app:
1. **Plan authoring**: Use an LLM to draft a plan with research, clustering, ideation, verification, design synthesis. Edit it to add budgets, confidence gates, and the specific ToolSpecs to call.
2. **Execution**: AMP’s scheduler resolves dependencies, runs independent nodes in parallel, halts when budgets or evidence rules fail, and writes verified findings to memory.
3. **Iteration**: For the next sprint, the plan reads prior insights from memory, runs targeted research nodes, collects new evidence, and outputs an updated brief with citations.
4. **Audit**: Traces + bundled evidence let PMs or compliance replay the run, inspect citations, or branch the plan safely for experiments.

## How do I run my own agents in AMP?
Wrap them as ToolSpecs (HTTP services exposing `/spec` and `/invoke`). A tool can be a full multi-step agent internally—it just needs to respect its published contract. Plans then orchestrate those tools, benefiting from AMP’s budget tracking, evidence checks, and trace logging. This way, even if the internal implementation evolves (say you improve the reasoning loop), the plan sees a stable interface and consistent guarantees.

## Where does AMP fit into my stack?
- **Product builders / PMs**: Use LLMs for creativity, but insist on AMP plans for anything entering production or requiring audit trails.
- **Engineers**: Implement tools as self-contained services with ToolSpecs; rely on the scheduler to coordinate them and catch regressions via deterministic replays.
- **Ops / Compliance**: Leverage memory provenance, evidence thresholds, and trace bundles to prove what ran, why it was trusted, and how much it cost.

## What should I implement first if adopting AMP today?
Follow the roadmap in `fix1.md`: fix the tool invocation mismatch, upgrade argument resolution, return full memory metadata, and add end-to-end tests. Once the baseline is solid, move on to dynamic tool discovery, constraint enforcement, evidence-first policies, and eventually plan optimisation and replay bundles.

---
For deeper context or to contribute, review the roadmap in `fix1.md` and the current code under `kernel/` and `adapters/`. Pull requests that close the gaps between the vision and implementation are very welcome.
