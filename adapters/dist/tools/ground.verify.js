"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.groundVerify = void 0;
// Simple BM25 implementation constants
const k1 = 1.5; // BM25 free parameter
const b = 0.75; // BM25 free parameter
// Helper function to calculate word frequency
function wordFreq(text, word) {
    const words = text.toLowerCase().split(/\W+/);
    return words.filter(w => w === word.toLowerCase()).length;
}
// Helper function to calculate BM25 score
function calculateBM25Score(query, document, avgDocLength) {
    const queryWords = query.toLowerCase().split(/\W+/).filter(w => w.length > 0);
    const docWords = document.toLowerCase().split(/\W+/).filter(w => w.length > 0);
    let score = 0;
    const docLength = docWords.length;
    for (const word of queryWords) {
        const freqInDoc = wordFreq(document, word);
        if (freqInDoc === 0)
            continue;
        // For simplicity, assume df (document frequency) is 1 for all terms
        const idf = Math.log(1 + (1000 - 1 + 0.5) / (1 + 0.5)); // 1000 is approx collection size
        const tf = (freqInDoc * (k1 + 1)) / (freqInDoc + k1 * (1 - b + b * (docLength / avgDocLength)));
        score += idf * tf;
    }
    return score;
}
exports.groundVerify = {
    spec: {
        name: 'ground.verify',
        description: 'Simple evidence verifier using BM25 and exact phrase matching',
        io: {
            input: {
                type: 'object',
                properties: {
                    claims: {
                        type: 'array',
                        items: { type: 'string' }
                    },
                    sources: {
                        type: 'array',
                        items: {
                            type: 'object',
                            properties: {
                                id: { type: 'string' },
                                uri: { type: 'string' },
                                score: { type: 'number' },
                                snippet: { type: 'string' },
                                stamp: { type: 'string', format: 'date-time' }
                            },
                            required: ['id', 'uri', 'score', 'snippet', 'stamp']
                        }
                    },
                    min_confidence: { type: 'number', default: 0.75 }
                },
                required: ['claims', 'sources']
            },
            output: {
                type: 'object',
                properties: {
                    claims: { type: 'array', items: { type: 'string' } },
                    supports: {
                        type: 'array',
                        items: {
                            type: 'object',
                            properties: {
                                claim_id: { type: 'string' },
                                source: { type: 'string' },
                                confidence: { type: 'number', minimum: 0, maximum: 1 },
                                explanation: { type: 'string' }
                            },
                            required: ['claim_id', 'source', 'confidence', 'explanation']
                        }
                    },
                    contradicts: {
                        type: 'array',
                        items: {
                            type: 'object',
                            properties: {
                                claim_id: { type: 'string' },
                                source: { type: 'string' },
                                confidence: { type: 'number', minimum: 0, maximum: 1 },
                                explanation: { type: 'string' }
                            },
                            required: ['claim_id', 'source', 'confidence', 'explanation']
                        }
                    },
                    verdicts: {
                        type: 'array',
                        items: {
                            type: 'object',
                            properties: {
                                claim_id: { type: 'string' },
                                verdict: { type: 'string', enum: ['supported', 'contradicted', 'neutral'] },
                                confidence: { type: 'number', minimum: 0, maximum: 1 },
                                needs_citation: { type: 'boolean' }
                            },
                            required: ['claim_id', 'verdict', 'confidence', 'needs_citation']
                        }
                    }
            }
        }
    },
    capabilities: ['evidence.verify'],
    constraints: {
        latency_p50_ms: 200,
        cost_per_call_usd: 0.0002,
        side_effects: false
    }
},
    invoke: async (args) => {
        const { claims, sources, min_confidence = 0.75 } = args;
        // Calculate average document length for BM25
        const totalLength = sources.reduce((sum, hit) => sum + hit.snippet.split(/\W+/).length, 0);
        const avgDocLength = sources.length > 0 ? totalLength / sources.length : 100;
        const allVerdicts = [];
        const allSupports = [];
        let allContradicts = [];
        for (let i = 0; i < claims.length; i++) {
            const claim = claims[i];
            const claimId = `claim_${i}`;
            // Score each source against the claim
            let bestSupport = null;
            let bestContradiction = null;
            for (const source of sources) {
                // Calculate BM25 score between claim and source snippet
                const bm25Score = calculateBM25Score(claim, source.snippet, avgDocLength);
                // Check for exact phrase matches to determine support vs contradiction
                let confidence = Math.min(bm25Score / 10, 1); // Normalize to 0-1 range
                // Check if the source supports or contradicts the claim
                // For this simple implementation, we'll consider exact matches as strong support
                const claimWords = claim.toLowerCase().split(/\W+/).filter(w => w.length > 2);
                const snippetWords = source.snippet.toLowerCase().split(/\W+/).filter(w => w.length > 2);
                let matches = 0;
                for (const word of claimWords) {
                    if (snippetWords.includes(word)) {
                        matches++;
                    }
                }
                // Adjust confidence based on word matches
                if (matches > 0) {
                    confidence = Math.min(confidence + (matches / claimWords.length) * 0.3, 1);
                }
                // Simple logic: if confidence > threshold, consider it support, else neutral
                // In a real implementation, you'd have more sophisticated logic to detect contradictions
                if (confidence >= min_confidence) {
                    if (!bestSupport || confidence > bestSupport.confidence) {
                        bestSupport = {
                            source: source,
                            confidence: confidence,
                            explanation: `Claim "${claim}" is supported by source content: "${source.snippet.substring(0, 100)}..."`
                        };
                    }
                }
            }
            // Generate verdict based on best support/contradiction
            let verdict = {
                claim_id: claimId,
                verdict: bestSupport ? 'supported' : 'neutral',
                confidence: bestSupport ? bestSupport.confidence : 0.1,
                needs_citation: !!bestSupport
            };
            allVerdicts.push(verdict);
            // Add supports/contradicts
            if (bestSupport) {
                allSupports.push({
                    claim_id: claimId,
                    source: bestSupport.source.id,
                    confidence: bestSupport.confidence,
                    explanation: bestSupport.explanation
                });
            }
        }
        return {
            result: {
                claims,
                supports: allSupports,
                contradicts: allContradicts,
                verdicts: allVerdicts
            }
        };
    }
};
//# sourceMappingURL=ground.verify.js.map
