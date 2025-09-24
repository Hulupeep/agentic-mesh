#!/bin/bash

# Example script to run AMP with the refund policy plan

set -e

echo "Starting AMP example: Refund Policy Query"

# Check if required tools are running
echo "Checking if adapters are running..."
if ! curl -s http://localhost:7401/spec/doc.search.local > /dev/null; then
    echo "Error: doc.search.local adapter not found at http://localhost:7401"
    echo "Please start the adapters using: cd adapters && npm run build && node dist/server.js"
    exit 1
fi

echo "Adapters are running, proceeding with plan execution..."

# Run the plan using ampctl
echo "Executing plan..."
cargo run --bin ampctl -- run --plan-file examples/plan.refund.json --out examples/output.json

echo "Plan executed successfully. Output written to examples/output.json"
echo "Contents of output:"
cat examples/output.json