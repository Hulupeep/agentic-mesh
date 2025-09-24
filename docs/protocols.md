# AMP Protocol Specifications

## Core Concepts

The Agentic Mesh Protocol (AMP) enables orchestration of tools through a declarative Plan IR, with built-in evidence verification, memory management, and constraint enforcement.

## Architecture

### Plan IR
- Declarative execution graphs
- Support for call, map, reduce, branch, assert, and memory operations
- Variable binding and referencing with `$var` syntax

### ToolSpec ABI
- Standardized interface for all tools
- Schema-based input/output validation
- Constraint specifications (latency, cost, tokens)
- Provenance and quality metadata

### Evidence System
- Claims verification with confidence scoring
- Support/contradiction tracking
- Policy enforcement based on evidence quality

### Memory System
- Key-value storage with provenance tracking
- Confidence-based write protection
- TTL-based expiration

## Protocol Evolution

The AMP protocol maintains backward compatibility through semantic versioning of the ToolSpec ABI. Core schemas will maintain stable fields, with new optional fields added in minor versions.

## Security Model

- Ed25519 signing of all trace events
- Policy enforcement at kernel level
- Constraint checking for cost, latency, and tokens