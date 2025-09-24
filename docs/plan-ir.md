# Plan IR Specification

The Plan IR (Intermediate Representation) is a JSON-based format that defines execution workflows in AMP.

## Structure

A plan consists of:

- `signals`: Global constraints and parameters
- `nodes`: Individual execution steps
- `edges`: Dependencies between nodes
- `stop_conditions`: Termination criteria

## Signals

```json
{
  "signals": {
    "latency_budget_ms": 5000,
    "cost_cap_usd": 1.0,
    "risk": 0.1
  }
}
```

- `latency_budget_ms`: Maximum time allowed for plan execution
- `cost_cap_usd`: Maximum cost allowed for plan execution
- `risk`: Risk tolerance (0.0 to 1.0)

## Nodes

Each node defines an operation to execute:

```json
{
  "id": "node_identifier",
  "op": "call",
  "tool": "tool.name",
  "args": {},
  "bind": {},
  "out": {}
}
```

### Operations

- `call`: Execute a single tool call
- `map`: Apply tool to each item in a collection
- `reduce`: Combine multiple results into one
- `branch`: Conditional execution path
- `assert`: Validate a condition
- `spawn`: Start parallel execution
- `mem.read`: Read from memory store
- `mem.write`: Write to memory store
- `verify`: Verify evidence
- `retry`: Execute with retry logic

## Edges

Define dependencies between nodes:

```json
{
  "edges": [
    {
      "from": "node_a",
      "to": "node_b"
    }
  ]
}
```

## Variables

Values can be passed between nodes using variable references:

- In `args`: `{"query": "$query_var"}` - references variable `query_var`
- In `out`: `{"result": "result"}` - stores result in variable `result`

## Example

See `examples/plan.refund.json` for a complete working example.