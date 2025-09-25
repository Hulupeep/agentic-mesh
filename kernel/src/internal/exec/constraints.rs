use crate::internal::{
    plan::ir::{Plan, Signals},
    tools::spec::ToolSpec,
};

#[derive(Debug, Clone)]
pub struct Budget {
    pub latency_remaining_ms: Option<u64>,
    pub cost_remaining_usd: Option<f64>,
    pub tokens_remaining: Option<u64>,
}

impl Budget {
    pub fn new(signals: Option<&Signals>) -> Self {
        Self {
            latency_remaining_ms: signals.as_ref().and_then(|s| s.latency_budget_ms),
            cost_remaining_usd: signals.as_ref().and_then(|s| s.cost_cap_usd),
            tokens_remaining: None, // We would calculate this based on inputs
        }
    }

    pub fn has_remaining(&self) -> bool {
        // Check if any budget constraint is exceeded
        if let Some(remaining) = self.latency_remaining_ms {
            if remaining == 0 {
                return false;
            }
        }

        if let Some(remaining) = self.cost_remaining_usd {
            if remaining <= 0.0 {
                return false;
            }
        }

        if let Some(remaining) = self.tokens_remaining {
            if remaining == 0 {
                return false;
            }
        }

        true
    }

    pub fn subtract_latency(&mut self, used_ms: u64) -> bool {
        if let Some(ref mut remaining) = self.latency_remaining_ms {
            if *remaining < used_ms {
                *remaining = 0;
                return false; // Budget exceeded
            }
            *remaining -= used_ms;
        }
        true // Within budget
    }

    pub fn subtract_cost(&mut self, used_usd: f64) -> bool {
        if let Some(ref mut remaining) = self.cost_remaining_usd {
            if *remaining < used_usd {
                *remaining = 0.0;
                return false; // Budget exceeded
            }
            *remaining -= used_usd;
        }
        true // Within budget
    }

    pub fn subtract_tokens(&mut self, used_tokens: u64) -> bool {
        if let Some(ref mut remaining) = self.tokens_remaining {
            if *remaining < used_tokens {
                *remaining = 0;
                return false; // Budget exceeded
            }
            *remaining -= used_tokens;
        }
        true // Within budget
    }
}

pub struct ConstraintChecker;

impl ConstraintChecker {
    pub fn check_plan_constraints(
        plan: &Plan,
        tool_specs: &[ToolSpec],
    ) -> Result<(), ConstraintError> {
        // Map tool names to their specs for quick lookup
        let tool_spec_map: std::collections::HashMap<_, _> =
            tool_specs.iter().map(|spec| (&spec.name, spec)).collect();

        // Calculate estimated resource usage
        let mut _est_tokens = 0u64;
        let mut est_cost = 0.0f64;
        let mut est_latency = 0u64;

        for node in &plan.nodes {
            if let Some(tool_name) = &node.tool {
                if let Some(tool_spec) = tool_spec_map.get(tool_name) {
                    if let Some(ref constraints) = tool_spec.constraints {
                        // Add estimated tokens
                        if let Some(tokens_max) = constraints.input_tokens_max {
                            _est_tokens += tokens_max as u64;
                        }

                        // Add estimated cost
                        if let Some(cost_per_call) = constraints.cost_per_call_usd {
                            est_cost += cost_per_call;
                        }

                        // Add estimated latency
                        if let Some(latency) = constraints.latency_p50_ms {
                            est_latency += latency as u64;
                        }
                    }
                }
            }
        }

        // Check against plan signals
        if let Some(ref signals) = plan.signals {
            if let Some(budget_ms) = signals.latency_budget_ms {
                if est_latency > budget_ms {
                    return Err(ConstraintError::LatencyBudgetExceeded {
                        estimated: est_latency,
                        budget: budget_ms,
                    });
                }
            }

            if let Some(budget_usd) = signals.cost_cap_usd {
                if est_cost > budget_usd {
                    return Err(ConstraintError::CostBudgetExceeded {
                        estimated: est_cost,
                        budget: budget_usd,
                    });
                }
            }

            // Risk check
            if let Some(risk_threshold) = signals.risk {
                if risk_threshold < 0.0 || risk_threshold > 1.0 {
                    return Err(ConstraintError::InvalidRiskValue(risk_threshold));
                }
            }
        }

        Ok(())
    }

    pub fn check_tool_constraints(
        tool_spec: &ToolSpec,
        args: &serde_json::Value,
    ) -> Result<(), ConstraintError> {
        if let Some(ref constraints) = tool_spec.constraints {
            // Check input token constraints
            if let Some(max_tokens) = constraints.input_tokens_max {
                let token_count = estimate_token_count(args)?;
                if token_count > max_tokens as u64 {
                    return Err(ConstraintError::InputTokensExceeded {
                        required: token_count,
                        max: max_tokens as u64,
                    });
                }
            }

            // Check rate limiting (this would require additional state tracking)
            // For now, we just acknowledge the constraint exists
        }

        Ok(())
    }

    pub fn estimate_remaining_budget(
        initial_budget: &Budget,
        tool_spec: &ToolSpec,
    ) -> Result<Budget, ConstraintError> {
        let mut new_budget = initial_budget.clone();

        if let Some(ref constraints) = tool_spec.constraints {
            // Subtract estimated cost
            if let Some(cost) = constraints.cost_per_call_usd {
                if !new_budget.subtract_cost(cost) {
                    return Err(ConstraintError::CostBudgetExceeded {
                        estimated: cost,
                        budget: initial_budget.cost_remaining_usd.unwrap_or(0.0),
                    });
                }
            }

            // Subtract estimated latency
            if let Some(latency) = constraints.latency_p50_ms {
                if !new_budget.subtract_latency(latency as u64) {
                    return Err(ConstraintError::LatencyBudgetExceeded {
                        estimated: latency as u64,
                        budget: initial_budget.latency_remaining_ms.unwrap_or(0),
                    });
                }
            }

            // Subtract estimated tokens
            if let Some(tokens) = constraints.input_tokens_max {
                if !new_budget.subtract_tokens(tokens as u64) {
                    return Err(ConstraintError::InputTokensExceeded {
                        required: tokens as u64,
                        max: initial_budget.tokens_remaining.unwrap_or(0),
                    });
                }
            }
        }

        Ok(new_budget)
    }
}

fn estimate_token_count(value: &serde_json::Value) -> Result<u64, ConstraintError> {
    // A very simple token estimation based on character count
    // In practice, you'd use a proper tokenizer
    let text = value.to_string();
    let chars = text.chars().count() as u64;

    // Rough estimation: 1 token ~ 4 characters
    Ok(chars / 4)
}

#[derive(Debug, thiserror::Error)]
pub enum ConstraintError {
    #[error("Latency budget exceeded: estimated {estimated}ms > budget {budget}ms")]
    LatencyBudgetExceeded { estimated: u64, budget: u64 },
    #[error("Cost budget exceeded: estimated ${estimated:.4} > budget ${budget:.4}")]
    CostBudgetExceeded { estimated: f64, budget: f64 },
    #[error("Input tokens exceeded: required {required} > max {max}")]
    InputTokensExceeded { required: u64, max: u64 },
    #[error("Invalid risk value: {0}, must be between 0.0 and 1.0")]
    InvalidRiskValue(f64),
    #[error("Token estimation error: {0}")]
    TokenEstimationError(String),
}
