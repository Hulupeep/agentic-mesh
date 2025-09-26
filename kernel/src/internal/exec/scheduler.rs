use crate::internal::{
    plan::ir::{Node, Plan},
    tools::spec::{ToolClient, ToolSpec},
};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use tokio::time::{timeout, Duration};

#[derive(Debug)]
pub struct ExecutionContext {
    pub variables: HashMap<String, Value>,
    pub tool_client: ToolClient,
    pub tool_specs: HashMap<String, ToolSpec>,
    pub tool_urls: HashMap<String, String>, // tool name to url mapping
    pub capability_index: HashMap<String, Vec<String>>,
    pub signals: Option<crate::internal::plan::ir::Signals>,
    pub trace_events: Vec<crate::internal::trace::trace::Trace>,
    pub completed_nodes: HashSet<String>,
    pub running_nodes: HashSet<String>,
    pub total_latency_ms: f64,
    pub total_cost_usd: f64,
    pub total_tokens: u64,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            tool_client: ToolClient::new(),
            tool_specs: HashMap::new(),
            tool_urls: HashMap::new(),
            capability_index: HashMap::new(),
            signals: None,
            trace_events: vec![],
            completed_nodes: HashSet::new(),
            running_nodes: HashSet::new(),
            total_latency_ms: 0.0,
            total_cost_usd: 0.0,
            total_tokens: 0,
        }
    }

    pub fn has_budget_remaining(&self) -> bool {
        self.check_budget_overrun().is_ok()
    }

    pub fn resolve_args(&self, args: Option<&HashMap<String, Value>>) -> Option<Value> {
        args.map(|map| Value::Object(self.resolve_map(map)))
    }

    fn resolve_map(&self, map: &HashMap<String, Value>) -> serde_json::Map<String, Value> {
        let mut resolved = serde_json::Map::new();
        for (key, value) in map {
            resolved.insert(key.clone(), self.resolve_value(value));
        }
        resolved
    }

    fn resolve_value(&self, value: &Value) -> Value {
        match value {
            Value::String(s) if s.starts_with('$') => {
                let reference = &s[1..];
                self.resolve_reference(reference)
                    .unwrap_or_else(|| Value::String(s.clone()))
            }
            Value::Array(items) => {
                Value::Array(items.iter().map(|item| self.resolve_value(item)).collect())
            }
            Value::Object(obj) => {
                let mut resolved_obj = serde_json::Map::new();
                for (k, v) in obj {
                    resolved_obj.insert(k.clone(), self.resolve_value(v));
                }
                Value::Object(resolved_obj)
            }
            _ => value.clone(),
        }
    }

    fn resolve_reference(&self, reference: &str) -> Option<Value> {
        if reference.is_empty() {
            return None;
        }

        let mut chars = reference.chars().peekable();
        let mut root = String::new();

        while let Some(&c) = chars.peek() {
            if c == '.' || c == '[' {
                break;
            }
            root.push(c);
            chars.next();
        }

        if root.is_empty() {
            return None;
        }

        let mut current = self.variables.get(&root)?.clone();

        while let Some(&c) = chars.peek() {
            match c {
                '.' => {
                    chars.next();
                    let mut key = String::new();
                    while let Some(&next) = chars.peek() {
                        if next == '.' || next == '[' {
                            break;
                        }
                        key.push(next);
                        chars.next();
                    }

                    if key.is_empty() {
                        return None;
                    }

                    match current {
                        Value::Object(ref map) => {
                            if let Some(value) = map.get(&key) {
                                current = value.clone();
                            } else {
                                return None;
                            }
                        }
                        _ => return None,
                    }
                }
                '[' => {
                    chars.next();
                    let mut index_str = String::new();
                    while let Some(&next) = chars.peek() {
                        if next == ']' {
                            break;
                        }
                        index_str.push(next);
                        chars.next();
                    }

                    if chars.peek() == Some(&']') {
                        chars.next();
                    } else {
                        return None;
                    }

                    let index: usize = index_str.parse().ok()?;

                    match current {
                        Value::Array(ref arr) => {
                            current = arr.get(index)?.clone();
                        }
                        _ => return None,
                    }
                }
                _ => return None,
            }
        }

        Some(current)
    }

    pub fn record_tool_usage(
        &mut self,
        tool_name: &str,
        spec: Option<&ToolSpec>,
        actual_latency_ms: f64,
        tokens_used: Option<u64>,
    ) -> Result<UsageRecord, ExecutionError> {
        let mut consumed_latency = actual_latency_ms;
        let mut consumed_cost = 0.0;
        let mut consumed_tokens = tokens_used.unwrap_or(0);

        if let Some(spec) = spec {
            if let Some(constraints) = &spec.constraints {
                if let Some(latency) = constraints.latency_p50_ms {
                    consumed_latency = consumed_latency.max(latency as f64);
                }
                if let Some(cost) = constraints.cost_per_call_usd {
                    consumed_cost += cost;
                }
                if consumed_tokens == 0 {
                    if let Some(tokens) = constraints.input_tokens_max {
                        consumed_tokens = consumed_tokens.max(tokens as u64);
                    }
                }
            }
        }

        self.total_latency_ms += consumed_latency;
        self.total_cost_usd += consumed_cost;
        self.total_tokens = self.total_tokens.saturating_add(consumed_tokens);

        if let Err(e) = self.check_budget_overrun() {
            // Record a summary trace before surfacing the budget error so downstream
            // policy evaluators have access to the final telemetry snapshot.
            self.push_budget_summary_trace();
            return Err(e);
        }

        Ok(UsageRecord {
            tool_name: tool_name.to_string(),
            latency_ms: consumed_latency,
            cost_usd: consumed_cost,
            tokens: consumed_tokens,
        })
    }

    pub fn register_tool_spec(&mut self, tool_name: String, spec: ToolSpec) {
        self.tool_specs.insert(tool_name, spec);
        self.rebuild_capability_index();
    }

    fn rebuild_capability_index(&mut self) {
        self.capability_index.clear();
        for (tool_name, spec) in &self.tool_specs {
            if let Some(capabilities) = &spec.capabilities {
                for capability in capabilities {
                    let entry = self
                        .capability_index
                        .entry(capability.clone())
                        .or_insert_with(Vec::new);
                    if !entry.iter().any(|existing| existing == tool_name) {
                        entry.push(tool_name.clone());
                    }
                }
            }
        }
    }

    fn select_tool_for_capability(&self, capability: &str) -> Option<CapabilityRouteDecision> {
        let candidates = self.capability_index.get(capability)?;
        if candidates.is_empty() {
            return None;
        }

        let mut ranked: Option<(f64, f64, String)> = None;
        let mut candidate_data = Vec::new();

        let remaining_cost = self
            .signals
            .as_ref()
            .and_then(|signals| signals.cost_cap_usd)
            .map(|cap| (cap - self.total_cost_usd).max(0.0));
        let remaining_latency = self
            .signals
            .as_ref()
            .and_then(|signals| signals.latency_budget_ms)
            .map(|budget| (budget as f64 - self.total_latency_ms).max(0.0));

        for tool_name in candidates {
            if !self.tool_urls.contains_key(tool_name) {
                continue;
            }

            let spec = match self.tool_specs.get(tool_name) {
                Some(spec) => spec,
                None => continue,
            };

            let (cost, latency) = spec
                .constraints
                .as_ref()
                .map(|constraints| {
                    (
                        constraints.cost_per_call_usd.unwrap_or(0.0),
                        constraints.latency_p50_ms.map(|v| v as f64).unwrap_or(0.0),
                    )
                })
                .unwrap_or((0.0, 0.0));

            let cost_headroom = remaining_cost
                .map(|remaining| remaining >= cost)
                .unwrap_or(true);
            let latency_headroom = remaining_latency
                .map(|remaining| remaining >= latency)
                .unwrap_or(true);

            candidate_data.push(serde_json::json!({
                "tool": tool_name,
                "cost_per_call_usd": cost,
                "latency_p50_ms": latency,
                "budget_cost_headroom": cost_headroom,
                "budget_latency_headroom": latency_headroom,
            }));

            let score = (cost, latency, tool_name.clone());
            if ranked
                .as_ref()
                .map(|current| (score.0, score.1, &score.2) < (current.0, current.1, &current.2))
                .unwrap_or(true)
            {
                ranked = Some((score.0, score.1, score.2.clone()));
            }
        }

        let (_, _, selected_tool) = ranked?;
        let rationale = serde_json::json!({
            "capability": capability,
            "selected_tool": selected_tool,
            "candidates": candidate_data,
        });

        Some(CapabilityRouteDecision {
            tool_name: selected_tool,
            rationale,
        })
    }

    fn resolve_tool(&mut self, node: &Node) -> Result<ToolResolution, ExecutionError> {
        if let Some(tool_name) = &node.tool {
            let tool_url = self.tool_urls.get(tool_name).ok_or_else(|| {
                ExecutionError::ValidationError(format!(
                    "Tool {} not found in tool URLs",
                    tool_name
                ))
            })?;
            let spec = self.tool_specs.get(tool_name).cloned();
            return Ok(ToolResolution {
                tool_name: tool_name.clone(),
                tool_url: tool_url.clone(),
                spec,
                capability: None,
            });
        }

        let capability = node.capability.as_ref().ok_or_else(|| {
            ExecutionError::ValidationError(format!(
                "Node {} requires a tool or capability",
                node.id
            ))
        })?;

        let decision = self.select_tool_for_capability(capability).ok_or_else(|| {
            ExecutionError::ValidationError(format!(
                "No tool available for capability {}",
                capability
            ))
        })?;

        let tool_url = self.tool_urls.get(&decision.tool_name).ok_or_else(|| {
            ExecutionError::ValidationError(format!(
                "Tool {} not found in tool URLs",
                decision.tool_name
            ))
        })?;

        let spec = self.tool_specs.get(&decision.tool_name).cloned();

        let mut trace = crate::internal::trace::trace::Trace::new(
            "capability_route".to_string(),
            node.id.clone(),
            format!(
                "Capability {} routed to tool {}",
                capability, decision.tool_name
            ),
        );
        trace.data = Some(decision.rationale.clone());
        self.trace_events.push(trace);

        Ok(ToolResolution {
            tool_name: decision.tool_name,
            tool_url: tool_url.clone(),
            spec,
            capability: Some(capability.clone()),
        })
    }

    pub fn enforce_tool_policy(
        &mut self,
        tool_name: &str,
        args: Option<&Value>,
    ) -> Result<(), ExecutionError> {
        if let Some(spec) = self.tool_specs.get(tool_name) {
            if let Some(policy) = &spec.policy {
                if let Some(deny_patterns) = &policy.deny_if {
                    if !deny_patterns.is_empty() {
                        let args_serialised = args
                            .map(|value| value.to_string().to_lowercase())
                            .unwrap_or_else(|| "null".to_string());

                        for pattern in deny_patterns {
                            let pattern_lower = pattern.to_lowercase();
                            if !pattern_lower.is_empty() && args_serialised.contains(&pattern_lower)
                            {
                                let message = format!(
                                    "Tool {} invocation blocked by policy pattern '{}'",
                                    tool_name, pattern
                                );

                                let mut trace = crate::internal::trace::trace::Trace::new(
                                    "policy_violation".to_string(),
                                    tool_name.to_string(),
                                    message.clone(),
                                );
                                trace.data = Some(serde_json::json!({
                                    "description": message,
                                    "pattern": pattern,
                                    "args": args
                                        .map(|v| v.clone())
                                        .unwrap_or(Value::Null),
                                }));
                                self.trace_events.push(trace);

                                return Err(ExecutionError::ToolExecutionError(message));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn push_budget_summary_trace(&mut self) {
        let latency_budget = self
            .signals
            .as_ref()
            .and_then(|s| s.latency_budget_ms)
            .map(|v| v as f64);
        let cost_cap = self.signals.as_ref().and_then(|s| s.cost_cap_usd);

        let summary = serde_json::json!({
            "total_latency_ms": self.total_latency_ms,
            "latency_budget_ms": latency_budget,
            "total_cost_usd": self.total_cost_usd,
            "cost_cap_usd": cost_cap,
            "total_tokens": self.total_tokens,
        });

        let mut trace = crate::internal::trace::trace::Trace::new(
            "budget_summary".to_string(),
            "plan".to_string(),
            "Plan budget summary".to_string(),
        );
        trace.cost_usd = Some(self.total_cost_usd);
        trace.tokens_out = Some(self.total_tokens);
        trace.data = Some(summary);
        self.trace_events.push(trace);
    }

    fn check_budget_overrun(&self) -> Result<(), ExecutionError> {
        if let Some(signals) = &self.signals {
            if let Some(latency_budget) = signals.latency_budget_ms {
                if self.total_latency_ms > latency_budget as f64 {
                    return Err(ExecutionError::BudgetExceeded(format!(
                        "Latency budget exceeded: {:.2}ms > {}ms",
                        self.total_latency_ms, latency_budget
                    )));
                }
            }
            if let Some(cost_cap) = signals.cost_cap_usd {
                if self.total_cost_usd > cost_cap {
                    return Err(ExecutionError::BudgetExceeded(format!(
                        "Cost budget exceeded: ${:.4} > ${:.4}",
                        self.total_cost_usd, cost_cap
                    )));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct UsageRecord {
    pub tool_name: String,
    pub latency_ms: f64,
    pub cost_usd: f64,
    pub tokens: u64,
}

#[derive(Debug)]
struct CapabilityRouteDecision {
    tool_name: String,
    rationale: serde_json::Value,
}

#[derive(Debug)]
struct ToolResolution {
    tool_name: String,
    tool_url: String,
    spec: Option<ToolSpec>,
    capability: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Plan validation failed: {0}")]
    ValidationError(String),
    #[error("Tool execution failed: {0}")]
    ToolExecutionError(String),
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    #[error("Budget exceeded: {0}")]
    BudgetExceeded(String),
}

pub struct Scheduler;

impl Scheduler {
    pub async fn execute_plan(
        &self,
        mut ctx: ExecutionContext,
        plan: &Plan,
    ) -> Result<ExecutionContext, ExecutionError> {
        if ctx.tool_urls.is_empty() {
            plan.validate()
                .map_err(|e| ExecutionError::ValidationError(e.to_string()))?;
        } else {
            plan.validate_with_tools(ctx.tool_urls.keys().map(|k| k.as_str()))
                .map_err(|e| ExecutionError::ValidationError(e.to_string()))?;
        }

        if ctx.signals.is_none() {
            ctx.signals = plan.signals.clone();
        }

        // Ensure ToolSpecs are available for all known tools so capability routing has metadata.
        let tool_entries: Vec<(String, String)> = ctx
            .tool_urls
            .iter()
            .map(|(name, url)| (name.clone(), url.clone()))
            .collect();
        if !tool_entries.is_empty() {
            let client = ctx.tool_client.clone();
            for (tool_name, url) in tool_entries {
                if ctx.tool_specs.contains_key(&tool_name) {
                    continue;
                }

                match client.get_tool_spec(&url, &tool_name).await {
                    Ok(spec) => {
                        ctx.register_tool_spec(tool_name.clone(), spec);
                        tracing::debug!(tool = %tool_name, "Hydrated ToolSpec for scheduler execution");
                    }
                    Err(e) => {
                        tracing::warn!(
                            tool = %tool_name,
                            url = %url,
                            error = %e,
                            "Failed to fetch ToolSpec"
                        );
                    }
                }
            }
        }

        // Process nodes in order respecting dependencies
        let mut remaining_nodes: Vec<&Node> = Self::optimized_node_order(&mut ctx, plan);
        let mut processed_count = 0;

        while !remaining_nodes.is_empty() && processed_count < 100 {
            // Prevent infinite loops
            let mut executed_this_round = false;

            // Find nodes that can be executed (dependencies satisfied)
            let mut executable_nodes = Vec::new();
            let mut remaining_next = Vec::new();

            for node in remaining_nodes {
                if ctx.completed_nodes.contains(&node.id) {
                    continue; // Already completed
                }

                if ctx.running_nodes.contains(&node.id) {
                    remaining_next.push(node); // Still running
                    continue;
                }

                // Check if all dependencies are met
                let can_execute = if let Some(edges) = &plan.edges {
                    edges
                        .iter()
                        .filter(|edge| edge.to == node.id)
                        .all(|edge| ctx.completed_nodes.contains(&edge.from))
                } else {
                    // No edges, all nodes are independent
                    true
                };

                if can_execute {
                    executable_nodes.push(node);
                } else {
                    remaining_next.push(node);
                }
            }

            remaining_nodes = remaining_next;

            // Execute all executable nodes
            for node in executable_nodes {
                ctx.running_nodes.insert(node.id.clone());

                let result = self.execute_node(&mut ctx, node).await;

                // Remove from running set and add to completed
                ctx.running_nodes.remove(&node.id);
                ctx.completed_nodes.insert(node.id.clone());

                match result {
                    Ok(_) => {
                        executed_this_round = true;
                        processed_count += 1;
                    }
                    Err(e) => {
                        tracing::error!("Node {} execution failed: {}", node.id, e);
                        return Err(e);
                    }
                }
            }

            if !executed_this_round {
                // No progress made, probably a circular dependency or missing dependencies
                return Err(ExecutionError::ValidationError(
                    "No executable nodes found - possible circular dependency".to_string(),
                ));
            }

            // Check if we still have budget
            if let Err(e) = ctx.check_budget_overrun() {
                return Err(e);
            }
        }

        ctx.push_budget_summary_trace();

        Ok(ctx)
    }

    async fn execute_node(
        &self,
        ctx: &mut ExecutionContext,
        node: &Node,
    ) -> Result<(), ExecutionError> {
        match &node.op {
            crate::internal::plan::ir::Operation::Call => self.execute_call(ctx, node).await,
            crate::internal::plan::ir::Operation::Map => self.execute_map(ctx, node).await,
            crate::internal::plan::ir::Operation::Reduce => self.execute_reduce(ctx, node).await,
            crate::internal::plan::ir::Operation::Branch => self.execute_branch(ctx, node).await,
            crate::internal::plan::ir::Operation::Assert => self.execute_assert(ctx, node).await,
            crate::internal::plan::ir::Operation::Spawn => self.execute_spawn(ctx, node).await,
            crate::internal::plan::ir::Operation::MemRead => self.execute_mem_read(ctx, node).await,
            crate::internal::plan::ir::Operation::MemWrite => {
                self.execute_mem_write(ctx, node).await
            }
            crate::internal::plan::ir::Operation::Verify => self.execute_verify(ctx, node).await,
            crate::internal::plan::ir::Operation::Retry => self.execute_retry(ctx, node).await,
        }
    }

    async fn execute_call(
        &self,
        ctx: &mut ExecutionContext,
        node: &Node,
    ) -> Result<(), ExecutionError> {
        let resolution = ctx.resolve_tool(node)?;

        let args = ctx.resolve_args(node.args.as_ref());
        ctx.enforce_tool_policy(&resolution.tool_name, args.as_ref())?;
        let spec = resolution.spec.clone();

        // Add trace event
        let trace_event = crate::internal::trace::trace::Trace::new(
            "step_start".to_string(),
            node.id.clone(),
            format!("Calling tool: {}", resolution.tool_name),
        );
        ctx.trace_events.push(trace_event);

        // Invoke the tool
        let start = std::time::Instant::now();
        let result = timeout(
            Duration::from_secs(30), // 30 second timeout
            ctx.tool_client
                .invoke_tool(&resolution.tool_url, &resolution.tool_name, args),
        )
        .await
        .map_err(|_| {
            ExecutionError::TimeoutError(format!("Tool call {} timed out", resolution.tool_name))
        })?
        .map_err(|e| ExecutionError::ToolExecutionError(e.to_string()))?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        let usage =
            ctx.record_tool_usage(&resolution.tool_name, spec.as_ref(), elapsed_ms, None)?;

        // Store the result in variables as specified by 'out' mapping
        if let Some(out_map) = &node.out {
            for (var_name, _result_path) in out_map {
                // For now, store the full result
                // In a full implementation, we would extract specific fields based on result_path
                ctx.variables.insert(var_name.clone(), result.clone());
            }
        }

        // Add trace event
        let mut trace_event = crate::internal::trace::trace::Trace::new(
            "step_end".to_string(),
            node.id.clone(),
            format!("Tool {} call completed", usage.tool_name),
        );
        trace_event.cost_usd = Some(usage.cost_usd);
        trace_event.tokens_out = Some(usage.tokens);
        trace_event.data = Some(serde_json::json!({
            "tool": usage.tool_name,
            "capability": resolution.capability,
            "latency_ms": usage.latency_ms,
            "total_latency_ms": ctx.total_latency_ms,
            "total_cost_usd": ctx.total_cost_usd,
            "total_tokens": ctx.total_tokens,
        }));
        ctx.trace_events.push(trace_event);

        Ok(())
    }

    async fn execute_map(
        &self,
        ctx: &mut ExecutionContext,
        node: &Node,
    ) -> Result<(), ExecutionError> {
        let resolution = ctx.resolve_tool(node)?;
        let spec = resolution.spec.clone();

        let collection_value = node
            .args
            .as_ref()
            .and_then(|args| args.get("collection"))
            .map(|value| ctx.resolve_value(value))
            .ok_or_else(|| {
                ExecutionError::ValidationError(
                    "Map operation requires a 'collection' argument".to_string(),
                )
            })?;

        let items = match collection_value {
            Value::Array(items) => items,
            _ => {
                return Err(ExecutionError::ValidationError(
                    "Map operation requires an array input".to_string(),
                ))
            }
        };

        let mut results = Vec::new();

        for (index, item) in items.iter().enumerate() {
            let mut iteration_args = node.args.clone().unwrap_or_default();
            iteration_args.insert("item".to_string(), item.clone());
            iteration_args.insert("index".to_string(), Value::Number(index.into()));

            let resolved_args = ctx.resolve_args(Some(&iteration_args));
            ctx.enforce_tool_policy(&resolution.tool_name, resolved_args.as_ref())?;

            let start = std::time::Instant::now();
            let result = timeout(
                Duration::from_secs(30),
                ctx.tool_client.invoke_tool(
                    &resolution.tool_url,
                    &resolution.tool_name,
                    resolved_args,
                ),
            )
            .await
            .map_err(|_| {
                ExecutionError::TimeoutError(format!("Map operation item {} timed out", index))
            })?
            .map_err(|e| ExecutionError::ToolExecutionError(e.to_string()))?;
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            ctx.record_tool_usage(&resolution.tool_name, spec.as_ref(), elapsed_ms, None)?;

            results.push(result);
        }

        if let Some(out_map) = &node.out {
            for (var_name, _) in out_map {
                ctx.variables
                    .insert(var_name.clone(), Value::Array(results.clone()));
            }
        }

        Ok(())
    }

    async fn execute_reduce(
        &self,
        ctx: &mut ExecutionContext,
        node: &Node,
    ) -> Result<(), ExecutionError> {
        let collection_value = node
            .args
            .as_ref()
            .and_then(|args| args.get("collection"))
            .map(|value| ctx.resolve_value(value))
            .ok_or_else(|| {
                ExecutionError::ValidationError(
                    "Reduce operation requires a 'collection' argument".to_string(),
                )
            })?;

        let items = match collection_value {
            Value::Array(items) => items,
            _ => {
                return Err(ExecutionError::ValidationError(
                    "Reduce operation requires an array input".to_string(),
                ))
            }
        };

        let mut result = String::new();
        for item in items {
            result.push_str(&item.to_string());
            result.push('\n');
        }

        if let Some(out_map) = &node.out {
            for (var_name, _) in out_map {
                ctx.variables
                    .insert(var_name.clone(), Value::String(result.clone()));
            }
        }

        Ok(())
    }

    async fn execute_branch(
        &self,
        _ctx: &mut ExecutionContext,
        _node: &Node,
    ) -> Result<(), ExecutionError> {
        // For now, just execute the branch operation without actual branching logic
        Ok(())
    }

    async fn execute_assert(
        &self,
        ctx: &mut ExecutionContext,
        node: &Node,
    ) -> Result<(), ExecutionError> {
        // Check an assertion about the current state
        let condition = node
            .args
            .as_ref()
            .and_then(|args| args.get("condition"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ExecutionError::ValidationError(
                    "Assert operation requires a 'condition' argument".to_string(),
                )
            })?;

        if let Some(evidence_value) = node.args.as_ref().and_then(|args| args.get("evidence")) {
            let resolved = ctx.resolve_value(evidence_value);
            if let Value::String(evidence_str) = resolved {
                let evidence: crate::internal::evidence::verify::Evidence =
                    serde_json::from_str(&evidence_str).map_err(|e| {
                        ExecutionError::ValidationError(format!("Invalid evidence format: {}", e))
                    })?;
                let verifier = crate::internal::evidence::verify::EvidenceVerifier;
                let summary = verifier.verify_evidence(&evidence);
                verifier
                    .validate_evidence_for_storage(&evidence, 0.8)
                    .map_err(|e| ExecutionError::ValidationError(e.to_string()))?;

                if let Ok(json) = serde_json::to_value(&summary) {
                    let mut trace = crate::internal::trace::trace::Trace::new(
                        "evidence_summary".to_string(),
                        node.id.clone(),
                        format!("Assertion evidence summary for {}", node.id),
                    );
                    trace.data = Some(json);
                    ctx.trace_events.push(trace);
                }
            }
        }

        // For now, just log the assertion
        tracing::info!("Assertion: {}", condition);

        // Check if the assertion passes
        // In a real implementation, we would evaluate the condition against the current context
        if condition == "true" {
            Ok(())
        } else {
            Err(ExecutionError::ValidationError(format!(
                "Assertion failed: {}",
                condition
            )))
        }
    }

    async fn execute_spawn(
        &self,
        _ctx: &mut ExecutionContext,
        _node: &Node,
    ) -> Result<(), ExecutionError> {
        // For now, just log the spawn operation
        tracing::info!("Spawn operation executed");
        Ok(())
    }

    async fn execute_mem_read(
        &self,
        ctx: &mut ExecutionContext,
        node: &Node,
    ) -> Result<(), ExecutionError> {
        let key_value = node
            .args
            .as_ref()
            .and_then(|args| args.get("key"))
            .map(|value| ctx.resolve_value(value))
            .ok_or_else(|| {
                ExecutionError::ValidationError(
                    "Memory read operation requires a 'key' argument".to_string(),
                )
            })?;

        let key = key_value.as_str().ok_or_else(|| {
            ExecutionError::ValidationError("Memory read key must resolve to a string".to_string())
        })?;

        let resolution = ctx.resolve_tool(node)?;

        let mem_store = crate::internal::mem::store::MemoryStore::new();
        let start = std::time::Instant::now();
        let result = mem_store
            .read(&resolution.tool_url, key)
            .await
            .map_err(|e| {
                ExecutionError::ToolExecutionError(format!("Memory read failed: {}", e))
            })?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        ctx.record_tool_usage(
            &resolution.tool_name,
            resolution.spec.as_ref(),
            elapsed_ms,
            None,
        )?;

        if let Some(entry) = result {
            if let Some(out_map) = &node.out {
                for (var_name, _) in out_map {
                    ctx.variables.insert(var_name.clone(), entry.value.clone());
                }
            }
        } else {
            // If key not found, we can either error or set to null/undefined
            if let Some(out_map) = &node.out {
                for (var_name, _) in out_map {
                    ctx.variables
                        .insert(var_name.clone(), serde_json::Value::Null);
                }
            }
        }

        Ok(())
    }

    async fn execute_mem_write(
        &self,
        ctx: &mut ExecutionContext,
        node: &Node,
    ) -> Result<(), ExecutionError> {
        let key_value = node
            .args
            .as_ref()
            .and_then(|args| args.get("key"))
            .map(|value| ctx.resolve_value(value))
            .ok_or_else(|| {
                ExecutionError::ValidationError(
                    "Memory write operation requires a 'key' argument".to_string(),
                )
            })?;

        let key = key_value.as_str().ok_or_else(|| {
            ExecutionError::ValidationError("Memory write key must resolve to a string".to_string())
        })?;

        let value = node
            .args
            .as_ref()
            .and_then(|args| args.get("value"))
            .map(|value| ctx.resolve_value(value))
            .ok_or_else(|| {
                ExecutionError::ValidationError(
                    "Memory write operation requires a 'value' argument".to_string(),
                )
            })?;

        let provenance = node
            .args
            .as_ref()
            .and_then(|args| args.get("provenance"))
            .map(|value| ctx.resolve_value(value))
            .and_then(|value| {
                value.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<String>>()
                })
            });

        let mut confidence = node
            .args
            .as_ref()
            .and_then(|args| args.get("confidence"))
            .map(|value| ctx.resolve_value(value))
            .and_then(|value| value.as_f64());

        let ttl = node
            .args
            .as_ref()
            .and_then(|args| args.get("ttl"))
            .map(|value| ctx.resolve_value(value))
            .and_then(|value| value.as_str().map(|s| s.to_string()));

        let resolution = ctx.resolve_tool(node)?;
        let resolved_args = ctx.resolve_args(node.args.as_ref());
        ctx.enforce_tool_policy(&resolution.tool_name, resolved_args.as_ref())?;

        let mut evidence_summary_json = None;

        if let Some(evidence_value) = node.args.as_ref().and_then(|args| args.get("evidence")) {
            let evidence_resolved = ctx.resolve_value(evidence_value);
            if let Value::String(evidence_str) = evidence_resolved {
                let evidence: crate::internal::evidence::verify::Evidence =
                    serde_json::from_str(&evidence_str).map_err(|e| {
                        ExecutionError::ValidationError(format!("Invalid evidence format: {}", e))
                    })?;

                let verifier = crate::internal::evidence::verify::EvidenceVerifier;
                let summary = verifier.verify_evidence(&evidence);
                evidence_summary_json = serde_json::to_value(&summary).ok();
                if let Err(e) = verifier.validate_evidence_for_storage(&evidence, 0.8) {
                    return Err(ExecutionError::ValidationError(format!(
                        "Evidence validation failed: {}",
                        e
                    )));
                }

                if summary.mean_confidence.is_finite() && summary.mean_confidence > 0.0 {
                    confidence = Some(summary.mean_confidence);
                }
            }
        } else if let Some(conf) = confidence {
            if conf < 0.8 {
                return Err(ExecutionError::ValidationError(format!(
                    "Memory write rejected: confidence {} < 0.8 threshold",
                    conf
                )));
            }
        }

        let provenance = provenance.ok_or_else(|| {
            ExecutionError::ValidationError(
                "Memory write operation requires non-empty provenance".to_string(),
            )
        })?;

        if provenance.is_empty() {
            return Err(ExecutionError::ValidationError(
                "Memory write operation requires non-empty provenance".to_string(),
            ));
        }

        let confidence = confidence.ok_or_else(|| {
            ExecutionError::ValidationError(
                "Memory write operation requires confidence >= 0.8".to_string(),
            )
        })?;

        if confidence < 0.8 {
            return Err(ExecutionError::ValidationError(format!(
                "Memory write rejected: confidence {} < 0.8 threshold",
                confidence
            )));
        }

        let mem_store = crate::internal::mem::store::MemoryStore::new();
        let start = std::time::Instant::now();
        mem_store
            .write(
                &resolution.tool_url,
                key,
                &value,
                Some(&provenance),
                Some(confidence),
                ttl.as_deref(),
                evidence_summary_json.as_ref(),
            )
            .await
            .map_err(|e| {
                ExecutionError::ToolExecutionError(format!("Memory write failed: {}", e))
            })?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        ctx.record_tool_usage(
            &resolution.tool_name,
            resolution.spec.as_ref(),
            elapsed_ms,
            None,
        )?;

        if let Some(json) = evidence_summary_json {
            let mut trace = crate::internal::trace::trace::Trace::new(
                "evidence_summary".to_string(),
                node.id.clone(),
                format!("Memory write evidence summary for {}", key),
            );
            trace.data = Some(json);
            ctx.trace_events.push(trace);
        }
        Ok(())
    }

    async fn execute_verify(
        &self,
        ctx: &mut ExecutionContext,
        node: &Node,
    ) -> Result<(), ExecutionError> {
        // Get claims and sources from arguments
        let claims_value = node
            .args
            .as_ref()
            .and_then(|args| args.get("claims"))
            .map(|value| ctx.resolve_value(value))
            .ok_or_else(|| {
                ExecutionError::ValidationError(
                    "Verify operation requires a 'claims' argument".to_string(),
                )
            })?;

        let claims = claims_value
            .as_array()
            .ok_or_else(|| {
                ExecutionError::ValidationError(
                    "Verify claims must resolve to an array".to_string(),
                )
            })?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<String>>();

        let sources_value = node
            .args
            .as_ref()
            .and_then(|args| args.get("sources"))
            .map(|value| ctx.resolve_value(value))
            .ok_or_else(|| {
                ExecutionError::ValidationError(
                    "Verify operation requires a 'sources' argument".to_string(),
                )
            })?;

        let sources = sources_value.as_array().ok_or_else(|| {
            ExecutionError::ValidationError("Verify sources must resolve to an array".to_string())
        })?;

        let resolution = ctx.resolve_tool(node)?;

        // Prepare arguments for the verify tool
        let verify_args = serde_json::json!({
            "claims": claims,
            "sources": sources.clone()
        });

        ctx.enforce_tool_policy(&resolution.tool_name, Some(&verify_args))?;

        let start_trace = crate::internal::trace::trace::Trace::new(
            "step_start".to_string(),
            node.id.clone(),
            "Verification step start".to_string(),
        );
        ctx.trace_events.push(start_trace);

        // Invoke the verification tool
        let start = std::time::Instant::now();
        let result = timeout(
            Duration::from_secs(30),
            ctx.tool_client.invoke_tool(
                &resolution.tool_url,
                &resolution.tool_name,
                Some(verify_args),
            ),
        )
        .await
        .map_err(|_| ExecutionError::TimeoutError("Verification tool call timed out".to_string()))?
        .map_err(|e| ExecutionError::ToolExecutionError(format!("Verification failed: {}", e)))?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        let usage = ctx.record_tool_usage(
            &resolution.tool_name,
            resolution.spec.as_ref(),
            elapsed_ms,
            None,
        )?;

        if let Ok(parsed_evidence) =
            serde_json::from_value::<crate::internal::evidence::verify::Evidence>(result.clone())
        {
            let verifier = crate::internal::evidence::verify::EvidenceVerifier;
            let summary = verifier.verify_evidence(&parsed_evidence);
            let summary_json = serde_json::to_value(&summary).ok();
            if let Some(json) = &summary_json {
                if let Some(out_map) = &node.out {
                    for (var_name, _) in out_map {
                        ctx.variables
                            .insert(format!("{}_summary", var_name), json.clone());
                    }
                }

                let mut trace = crate::internal::trace::trace::Trace::new(
                    "evidence_summary".to_string(),
                    node.id.clone(),
                    format!("Verification summary for {}", node.id),
                );
                trace.data = summary_json;
                ctx.trace_events.push(trace);
            }
        }

        // Store the verification result in output variables
        if let Some(out_map) = &node.out {
            for (var_name, _) in out_map {
                ctx.variables.insert(var_name.clone(), result.clone());
            }
        }

        let mut end_trace = crate::internal::trace::trace::Trace::new(
            "step_end".to_string(),
            node.id.clone(),
            "Verification step complete".to_string(),
        );
        end_trace.cost_usd = Some(usage.cost_usd);
        end_trace.tokens_out = Some(usage.tokens);
        end_trace.data = Some(serde_json::json!({
            "tool": usage.tool_name,
            "capability": resolution.capability,
            "latency_ms": usage.latency_ms,
            "total_latency_ms": ctx.total_latency_ms,
            "total_cost_usd": ctx.total_cost_usd,
            "total_tokens": ctx.total_tokens,
        }));
        ctx.trace_events.push(end_trace);

        Ok(())
    }

    async fn execute_retry(
        &self,
        ctx: &mut ExecutionContext,
        node: &Node,
    ) -> Result<(), ExecutionError> {
        // Retry operation - execute the tool with retries
        let resolution = ctx.resolve_tool(node)?;
        let tool_name = resolution.tool_name.clone();
        let tool_url = resolution.tool_url.clone();
        let spec = resolution.spec.clone();

        let args = ctx.resolve_args(node.args.as_ref());
        ctx.enforce_tool_policy(&tool_name, args.as_ref())?;

        // Try up to 3 times
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            let start = std::time::Instant::now();
            let invocation = timeout(
                Duration::from_secs(30),
                ctx.tool_client
                    .invoke_tool(&tool_url, &tool_name, args.clone()),
            )
            .await;
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            match invocation {
                Ok(Ok(result)) => {
                    ctx.record_tool_usage(&tool_name, spec.as_ref(), elapsed_ms, None)?;
                    // Store the result in variables as specified by 'out' mapping
                    if let Some(out_map) = &node.out {
                        for (var_name, _result_path) in out_map {
                            ctx.variables.insert(var_name.clone(), result.clone());
                        }
                    }
                    return Ok(());
                }
                Ok(Err(e)) => {
                    ctx.record_tool_usage(&tool_name, spec.as_ref(), elapsed_ms, None)?;
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(ExecutionError::ToolExecutionError(e.to_string()));
                    }
                    tracing::warn!(
                        "Attempt {} failed for tool {}, retrying: {}",
                        attempts,
                        tool_name,
                        e
                    );
                    tokio::time::sleep(Duration::from_millis(500)).await; // Wait before retry
                }
                Err(_) => {
                    ctx.record_tool_usage(&tool_name, spec.as_ref(), elapsed_ms, None)?;
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(ExecutionError::TimeoutError(format!(
                            "Tool call {} timed out after {} attempts",
                            tool_name, max_attempts
                        )));
                    }
                    tracing::warn!(
                        "Attempt {} timed out for tool {}, retrying",
                        attempts,
                        tool_name
                    );
                    tokio::time::sleep(Duration::from_millis(500)).await; // Wait before retry
                }
            }
        }
    }

    fn optimized_node_order<'a>(ctx: &mut ExecutionContext, plan: &'a Plan) -> Vec<&'a Node> {
        let mut priorities: Vec<NodePriority<'a>> = plan
            .nodes
            .iter()
            .enumerate()
            .map(|(idx, node)| {
                let (cost, latency, selected_tool) = Scheduler::estimate_node_cost(ctx, node);
                NodePriority {
                    node,
                    estimated_cost: cost,
                    estimated_latency: latency,
                    selected_tool,
                    original_index: idx,
                }
            })
            .collect();

        priorities.sort_by(|a, b| match cmp_f64(a.estimated_cost, b.estimated_cost) {
            Ordering::Equal => match cmp_f64(a.estimated_latency, b.estimated_latency) {
                Ordering::Equal => a.original_index.cmp(&b.original_index),
                other => other,
            },
            other => other,
        });

        let ordered_nodes: Vec<&Node> = priorities.iter().map(|p| p.node).collect();

        let optimisation_data = serde_json::json!({
            "ordered_nodes": priorities
                .iter()
                .map(|p| serde_json::json!({
                    "node": p.node.id,
                    "op": format!("{:?}", p.node.op),
                    "capability": p.node.capability,
                    "selected_tool": p.selected_tool,
                    "estimated_cost_usd": p.estimated_cost,
                    "estimated_latency_ms": p.estimated_latency,
                    "original_index": p.original_index,
                }))
                .collect::<Vec<_>>(),
        });

        let mut trace = crate::internal::trace::trace::Trace::new(
            "plan_optimizer".to_string(),
            "plan".to_string(),
            "Plan optimizer determined execution order".to_string(),
        );
        trace.data = Some(optimisation_data);
        ctx.trace_events.push(trace);

        ordered_nodes
    }

    fn estimate_node_cost(ctx: &ExecutionContext, node: &Node) -> (f64, f64, Option<String>) {
        let requires_tool = matches!(
            node.op,
            crate::internal::plan::ir::Operation::Call
                | crate::internal::plan::ir::Operation::Map
                | crate::internal::plan::ir::Operation::Reduce
                | crate::internal::plan::ir::Operation::Verify
                | crate::internal::plan::ir::Operation::MemRead
                | crate::internal::plan::ir::Operation::MemWrite
                | crate::internal::plan::ir::Operation::Retry
        );

        if !requires_tool {
            return (0.0, 0.0, None);
        }

        if let Some(tool_name) = &node.tool {
            if let Some(spec) = ctx.tool_specs.get(tool_name) {
                let (cost, latency) = spec
                    .constraints
                    .as_ref()
                    .map(|constraints| {
                        (
                            constraints.cost_per_call_usd.unwrap_or(0.0),
                            constraints.latency_p50_ms.map(|v| v as f64).unwrap_or(0.0),
                        )
                    })
                    .unwrap_or((0.0, 0.0));
                return (cost, latency, Some(tool_name.clone()));
            }
        }

        if let Some(capability) = &node.capability {
            if let Some(decision) = ctx.select_tool_for_capability(capability) {
                if let Some(spec) = ctx.tool_specs.get(&decision.tool_name) {
                    let (cost, latency) = spec
                        .constraints
                        .as_ref()
                        .map(|constraints| {
                            (
                                constraints.cost_per_call_usd.unwrap_or(0.0),
                                constraints.latency_p50_ms.map(|v| v as f64).unwrap_or(0.0),
                            )
                        })
                        .unwrap_or((0.0, 0.0));
                    return (cost, latency, Some(decision.tool_name));
                }
            }

            return (f64::MAX, f64::MAX, None);
        }

        (f64::MAX, f64::MAX, None)
    }
}

fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

struct NodePriority<'a> {
    node: &'a Node,
    estimated_cost: f64,
    estimated_latency: f64,
    selected_tool: Option<String>,
    original_index: usize,
}
