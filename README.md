# Agentic Mesh Protocol (AMP)

** Work in progress - please get in touch with your comments on LI linkedin.com/in/colmbyrne  **

 

## The Problem We Are Solving

Most agent stacks grew out of prototype scripts. They rely on prompt chains and glue code that hide the real workflow, so teams struggle to answer basic questions: Which tool version ran? Did we stay inside the budget? Why did yesterday’s output differ from today’s? As flows grow, each fix is a one-off patch and every new agent increases the blast radius.

This is especially painful for teams trying to ship AI SaaS. Customers expect deterministic behaviour, cost controls, and verifiable outputs, but current frameworks offer few guardrails. Without a shared contract for tools and plans, it is hard to onboard vendors, add monitoring, or let the system improve itself safely (see `prd_self_evovolving.md`).

## Mission

AMP exists to make multi-step AI work repeatable, auditable, and predictable. Plans should read like a blueprint, tools should declare their contract, and every run should leave evidence that can be checked later—so a product team can operate, monitor, and evolve an agentic SaaS with confidence.

## Approach: Protocol + Implementation

AMP is both a protocol and a working implementation.

- **Protocol**: A set of JSON schemas that define plans, tools, evidence, memory entries, and policy artefacts. These schemas describe how any compliant runtime should talk about workflows and results.
- **Implementation**: A Rust kernel and TypeScript adapters that execute those schemas. The kernel schedules plans, enforces budgets, verifies evidence, and writes trace data. Adapters expose real tools behind a consistent HTTP surface.

The protocol gives teams a stable interface for tools and plans; the implementation shows how to build deterministic orchestration on top of it.

## Building Agentic SaaS with AMP

The roadmap in `prd_self_evovolving.md` treats an agent platform like any other production service: observe → diagnose → propose → validate. AMP provides the raw materials for that loop:

- Plans are explicit artifacts that can be diffed, reviewed, and regenerated.
- ToolSpecs capture cost, latency, and policy hints so the platform can choose providers automatically.
- Trace, evidence, and policy outputs form the audit trail needed for automated change management.

This combination lets a SaaS team introduce guardrails first, and then add self-improving behaviour when they are ready.

## Overview

The reference implementation runs Plan IR (declarative JSON) against ToolSpecs discovered at runtime. It gathers telemetry for cost, latency, and tokens, verifies evidence summaries, and applies policy decisions before data is stored or returned.

### Key Capabilities

- **Plan IR**: Declarative execution graphs with support for complex workflows
- **ToolSpec ABI**: Standardized interface for all tools with schema validation
- **Evidence System**: Verification of claims with confidence scoring
- **Memory Management**: Key-value storage with provenance tracking
- **Constraint Enforcement**: Budget management for cost, latency, and tokens
- **Policy Engine**: Configurable policy enforcement for safety and reliability
- **Capability Routing**: Plans can target capabilities; the kernel selects compliant tools deterministically and records the routing decision.
- **Plan Optimiser**: A deterministic pass that reorders independent steps using ToolSpec telemetry and emits trace events for audit.
- **Self-Evolution Hooks**: Telemetry, evidence, and policy outputs are structured so higher-level services (e.g. the self-evolving loop in `prd_self_evovolving.md`) can propose, validate, and ship changes without manual babysitting.

## Quickstart in 90 Seconds

1. **Clone the repository**
   ```bash
   git clone https://github.com/acme/amp.git
   cd amp
   ```

2. **Install dependencies**
   ```bash
   make deps
   ```

3. **Build the system**
   ```bash
   make build
   ```

4. **Start the adapter tools** (in a separate terminal)
   ```bash
   make start-adapters
   ```

5. **Execute an example plan**
   ```bash
   cd kernel
   cargo run --bin ampctl -- run --plan-file ../examples/plan.refund.json --out output.json
   ```

## Example API Call

Once you have the kernel running as a server:

```bash
curl -X POST http://localhost:7777/v1/plan/execute \
  -H "Content-Type: application/json" \
  -d @examples/plan.refund.json
```

## Architecture

The AMP system consists of:

- **Kernel**: Rust-based orchestration engine with HTTP API
- **Adapters**: Node.js/TypeScript tools implementing the ToolSpec ABI
- **Schemas**: JSON Schema definitions for all protocol objects
- **CLI**: `ampctl` for plan execution and management

## Beyond Code Generation

AMP is not limited to software delivery. Any multi-step workflow that needs determinism, auditability, and guardrails can be encoded as a plan:

- **Research & Analysis**: Chain document retrieval, summarisation, validation, and insight storage with guaranteed citations.
- **Operational Runbooks**: Capture incident response or security triage steps as declarative playbooks with enforced ordering and budgets.
- **Content Pipelines**: Orchestrate translation, fact checking, compliance review, and publishing across multiple agents.
- **Decision Support**: Coordinate data ingestion, scoring, human approvals, and memory updates for healthcare or finance decisions with traceable provenance.
- **Robotics & Hardware**: Schedule perception, planning, safety checks, and reporting for autonomous systems, ensuring the same safe sequence every time.

See the [FAQ](FAQ.md) for additional scenarios and migration guidance.

## ToolSpec ABI Contract Stability

The ToolSpec ABI maintains backward compatibility through semantic versioning. Core interfaces remain stable while new optional fields may be added in minor versions.

## Adding a New Tool

To add a new tool:

1. Define your ToolSpec following the schema in `schemas/ToolSpec.schema.json`
2. Implement the tool using the interfaces in `adapters/src/common/toolshim.ts`
3. Register the tool in `adapters/src/server.ts`
4. Update `examples/plan.refund.json` to use your new tool

## Development

### Prerequisites

- Rust 1.70+
- Node.js 18+
- pnpm
- SQLite3

### Running Tests

```bash
make test
```

### Building Docker Images

```bash
make docker-build
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
