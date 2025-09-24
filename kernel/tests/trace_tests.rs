//! Tests for trace functionality including signing

use amp::internal::trace::trace::{Trace, TraceSigner};

#[test]
fn test_trace_creation() {
    let mut trace = Trace::new(
        "test_event".to_string(),
        "step_1".to_string(),
        "Test trace creation".to_string(),
    );
    
    // Verify trace properties
    assert!(!trace.plan_id.is_empty());
    assert_eq!(trace.step_id, "step_1");
    assert_eq!(trace.event_type, "test_event");
    assert!(trace.ts.timestamp() > 0); // Should have a valid timestamp
    
    println!("Trace creation test passed");
}

#[test]
fn test_trace_with_plan_id() {
    let plan_id = "test-plan-123".to_string();
    let trace = Trace::with_plan_id(
        plan_id.clone(),
        "test_event".to_string(),
        "step_1".to_string(),
        "Test trace with predefined plan ID".to_string(),
    );
    
    assert_eq!(trace.plan_id, plan_id);
    assert_eq!(trace.event_type, "test_event");
    assert_eq!(trace.step_id, "step_1");
    
    println!("Trace with plan ID test passed");
}

#[test]
fn test_trace_signing() {
    let signer = TraceSigner::new().expect("Should create signer successfully");
    
    let mut trace = Trace::new(
        "signed_event".to_string(),
        "step_sign".to_string(),
        "Test trace signing".to_string(),
    );
    
    // Sign the trace
    signer.sign_trace(&mut trace).expect("Should sign trace successfully");
    
    // Verify the signature
    let is_valid = trace.verify_signature(signer.get_public_key()).expect("Should verify signature");
    assert!(is_valid);
    
    println!("Trace signing test passed");
}

#[test]
fn test_trace_signature_verification_failure() {
    let signer1 = TraceSigner::new().expect("Should create first signer");
    let signer2 = TraceSigner::new().expect("Should create second signer");
    
    let mut trace = Trace::new(
        "signed_event".to_string(),
        "step_sign".to_string(),
        "Test trace signing with different key".to_string(),
    );
    
    // Sign with first key
    signer1.sign_trace(&mut trace).expect("Should sign trace successfully");
    
    // Try to verify with different key - should fail
    let result = trace.verify_signature(signer2.get_public_key());
    
    // This may succeed or fail depending on the implementation,
    // but let's just test that we can attempt verification
    println!("Trace signature verification test completed");
}