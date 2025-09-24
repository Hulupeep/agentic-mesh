# Evidence and Verification

AMP incorporates a sophisticated evidence system to ensure grounding and reliability of results.

## Evidence Structure

Evidence objects contain:

- `claims`: Statements to be verified
- `supports`: Sources that support claims
- `contradicts`: Sources that contradict claims
- `verdicts`: Final verdicts on each claim with confidence scores

## Verification Process

The `ground.verify` tool implements verification by:

1. Taking claims and source documents as input
2. Computing similarity scores between claims and sources
3. Generating support/contradiction relationships
4. Producing verdicts with confidence scores

## Confidence Thresholds

- Memory writes require `confidence >= 0.8`
- Plan execution can be gated by minimum confidence thresholds
- Policy enforcement occurs based on evidence quality

## Provenance Tracking

When tools have `provenance.attribution_required=true`, the kernel ensures that responses include appropriate citations to source documents that supported the answer.

## Evidence Schema

See `schemas/Evidence.schema.json` for the complete schema definition.