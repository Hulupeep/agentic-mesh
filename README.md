# Agentic Mesh Protocol (AMP)

[![CI](https://github.com/acme/amp/actions/workflows/ci.yml/badge.svg)](https://github.com/acme/amp/actions/workflows/ci.yml)

A next-generation orchestration layer where tools are the compiler surface, plans are code, evidence is currency, and memory is the moat.

## Overview

The Agentic Mesh Protocol (AMP) provides a sophisticated orchestration kernel that executes Plan IR (JSON) against standardized ToolSpecs, with built-in constraint enforcement (latency, cost, tokens) and evidence verification.

### Key Features

- **Plan IR**: Declarative execution graphs with support for complex workflows
- **ToolSpec ABI**: Standardized interface for all tools with schema validation
- **Evidence System**: Verification of claims with confidence scoring
- **Memory Management**: Key-value storage with provenance tracking
- **Constraint Enforcement**: Budget management for cost, latency, and tokens
- **Policy Engine**: Configurable policy enforcement for safety and reliability

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