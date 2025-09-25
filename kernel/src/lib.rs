// Kernel library entry point

pub mod internal {
    pub mod plan {
        pub mod ir;
    }
    pub mod tools {
        pub mod spec;
    }
    pub mod exec {
        pub mod constraints;
        pub mod scheduler;
    }
    pub mod evidence {
        pub mod verify;
    }
    pub mod mem {
        pub mod store;
    }
    pub mod trace {
        pub mod trace;
    }
    pub mod policy {
        pub mod policy;
    }
    pub mod api;
    pub mod registry;
}

// Re-export key types for external use
pub use internal::evidence::verify::{Evidence, EvidenceValidationError, EvidenceVerifier};
pub use internal::exec::constraints::{Budget, ConstraintChecker, ConstraintError};
pub use internal::exec::scheduler::{ExecutionContext, ExecutionError, Scheduler};
pub use internal::mem::store::{MemoryEntry, MemoryError, MemoryStore};
pub use internal::plan::ir::{Node, Operation, Plan, PlanValidationError, Signals};
pub use internal::policy::policy::{PolicyContext, PolicyEngine, PolicyError, PolicyResult};
pub use internal::tools::spec::{ToolClient, ToolError, ToolSpec};
pub use internal::trace::trace::{Trace, TraceError, TraceSigner};
