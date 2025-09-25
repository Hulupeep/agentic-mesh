use amp::internal::evidence::verify::{
    Contradiction, Evidence, EvidenceValidationError, EvidenceVerifier, Support, Verdict,
    VerdictType,
};

#[test]
fn test_evidence_verifier_summary() {
    let verifier = EvidenceVerifier;
    let evidence = Evidence {
        claims: Some(vec!["claim_a".into(), "claim_b".into()]),
        supports: Some(vec![Support {
            claim_id: "claim_a".into(),
            source: "source_a".into(),
            confidence: 0.9,
            explanation: None,
        }]),
        contradicts: Some(vec![Contradiction {
            claim_id: "claim_b".into(),
            source: "source_b".into(),
            confidence: 0.4,
            explanation: None,
        }]),
        verdicts: Some(vec![
            Verdict {
                claim_id: "claim_a".into(),
                verdict: VerdictType::Supported,
                confidence: 0.88,
                needs_citation: true,
            },
            Verdict {
                claim_id: "claim_b".into(),
                verdict: VerdictType::Contradicted,
                confidence: 0.42,
                needs_citation: false,
            },
        ]),
    };

    let summary = verifier.verify_evidence(&evidence);
    assert_eq!(summary.total_claims, 2);
    assert_eq!(summary.supported_claims, 1);
    assert_eq!(summary.contradicted_claims, 1);
    assert!(summary.mean_confidence > 0.6 && summary.mean_confidence < 1.0);
    assert!(summary.per_claim.contains_key("claim_a"));
    assert!(summary.per_claim.contains_key("claim_b"));
}

#[test]
fn test_evidence_validate_requires_support() {
    let verifier = EvidenceVerifier;
    let evidence = Evidence {
        claims: Some(vec!["claim_a".into()]),
        supports: None,
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_a".into(),
            verdict: VerdictType::Supported,
            confidence: 0.95,
            needs_citation: false,
        }]),
    };

    let result = verifier.validate_evidence_for_storage(&evidence, 0.8);
    assert!(matches!(
        result,
        Err(EvidenceValidationError::MissingSupport { .. })
    ));
}
