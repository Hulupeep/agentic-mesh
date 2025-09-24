// Kernel library entry point

pub mod internal {
    pub mod plan {
        pub mod ir;
    }
    pub mod tools {
        pub mod spec;
    }
    pub mod exec {
        pub mod scheduler;
        pub mod constraints;
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
}

// Re-export key types for external use
pub use internal::plan::ir::{Plan, Node, Operation, Signals, PlanValidationError};
pub use internal::tools::spec::{ToolSpec, ToolClient, ToolError};
pub use internal::exec::scheduler::{Scheduler, ExecutionContext, ExecutionError};
pub use internal::exec::constraints::{ConstraintChecker, Budget, ConstraintError};
pub use internal::evidence::verify::{Evidence, EvidenceVerifier, EvidenceValidationError};
pub use internal::mem::store::{MemoryStore, MemoryEntry, MemoryError};
pub use internal::trace::trace::{Trace, TraceSigner, TraceError};
pub use internal::policy::policy::{PolicyEngine, PolicyContext, PolicyResult, PolicyError};