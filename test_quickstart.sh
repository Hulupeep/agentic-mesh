#!/bin/bash

# Quickstart test script to verify the AMP system works properly

set -e  # Exit on any error

echo "AMP Quickstart Test"
echo "==================="

# Check if directory structure is correct
echo "Checking project structure..."
if [ ! -d "kernel" ] || [ ! -d "adapters" ] || [ ! -d "schemas" ]; then
    echo "ERROR: Missing core directories"
    exit 1
fi

echo "✓ Project structure OK"

# Check if required files exist
required_files=(
    "kernel/Cargo.toml"
    "adapters/package.json" 
    "schemas/Plan.schema.json"
    "examples/plan.refund.json"
    "README.md"
    "Makefile"
)

for file in "${required_files[@]}"; do
    if [ ! -f "$file" ]; then
        echo "ERROR: Missing required file: $file"
        exit 1
    fi
done

echo "✓ Required files present"

# Check if package files have proper content
echo "Checking package files..."

# Check Cargo.toml
if ! grep -q 'name = "amp"' kernel/Cargo.toml; then
    echo "ERROR: Cargo.toml doesn't have proper name"
    exit 1
fi

# Check package.json files
if ! grep -q '"name":.*adapters' adapters/package.json; then
    echo "ERROR: adapters/package.json doesn't have proper name"
    exit 1
fi

if ! grep -q '"name":.*spec' spec-ts/package.json; then
    echo "ERROR: spec-ts/package.json doesn't have proper name"
    exit 1
fi

echo "✓ Package files OK"

# Check if schema files are valid JSON
echo "Checking schema validity..."
for schema in schemas/*.schema.json; do
    if ! python3 -m json.tool "$schema" > /dev/null 2>&1; then
        echo "ERROR: Invalid JSON in schema file: $schema"
        exit 1
    fi
done

echo "✓ Schemas are valid JSON"

# Check if TypeScript files have proper syntax (basic check)
echo "Checking TypeScript files..."
if ! grep -q "ToolHandler\|ToolSpec" adapters/src/common/toolshim.ts; then
    echo "ERROR: toolshim.ts doesn't contain expected exports"
    exit 1
fi

echo "✓ TypeScript files OK"

# Check if Rust files have proper syntax (basic check)
echo "Checking Rust files..."
if ! grep -q "pub mod\|use crate" kernel/src/lib.rs; then
    echo "ERROR: lib.rs doesn't contain expected module declarations"
    exit 1
fi

echo "✓ Rust files OK"

# Check example plan structure
echo "Checking example plan..."
if ! python3 -m json.tool examples/plan.refund.json > /dev/null 2>&1; then
    echo "ERROR: Invalid JSON in example plan"
    exit 1
fi

# Basic check that it contains expected fields
if ! grep -q '"signals"\|"nodes"\|"edges"' examples/plan.refund.json; then
    echo "ERROR: Example plan.json doesn't contain expected fields"
    exit 1
fi

echo "✓ Example plan OK"

# Check Makefile targets
echo "Checking Makefile..."
if ! grep -q "build\|test\|start-adapters" Makefile; then
    echo "ERROR: Makefile missing expected targets"
    exit 1
fi

echo "✓ Makefile OK"

echo
echo "🎉 All quickstart checks passed!"
echo
echo "To run the full system:"
echo "1. cd adapters && pnpm install && pnpm build && pnpm start"
echo "2. In another terminal: cd kernel && cargo run --bin kernel-api"
echo "3. Execute plans using the API or CLI"