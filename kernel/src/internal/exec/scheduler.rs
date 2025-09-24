use crate::internal::{
    plan::ir::{Node, Operation, Plan},
    tools::spec::{ToolClient, ToolSpec},
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tokio::time::{timeout, Duration};

#[derive(Debug)]
pub struct ExecutionContext {
    pub variables: HashMap<String, Value>,
    pub tool_client: ToolClient,
    pub tool_specs: HashMap<String, ToolSpec>,
    pub tool_urls: HashMap<String, String>, // tool name to url mapping
    pub signals: Option<crate::internal::plan::ir::Signals>,
    pub trace_events: Vec<crate::internal::trace::trace::Trace>,
    pub completed_nodes: HashSet<String>,
    pub running_nodes: HashSet<String>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            tool_client: ToolClient::new(),
            tool_specs: HashMap::new(),
            tool_urls: HashMap::new(),
            signals: None,
            trace_events: vec![],
            completed_nodes: HashSet::new(),
            running_nodes: HashSet::new(),
        }
    }

    pub fn has_budget_remaining(&self) -> bool {
        // Check if we have remaining budget based on signals
        if let Some(signals) = &self.signals {
            // For now, just return true - we'll implement budget tracking later
            return true;
        }
        true
    }

    pub fn resolve_args(&self, args: Option<&HashMap<String, String>>) -> Option<Value> {
        if let Some(args_map) = args {
            let mut resolved = serde_json::Map::new();
            for (key, value) in args_map {
                if value.starts_with('$') {
                    // This is a variable reference
                    let var_name = &value[1..];
                    if let Some(var_value) = self.variables.get(var_name) {
                        resolved.insert(key.clone(), var_value.clone());
                    } else {
                        // If variable is not found, use the literal string
                        resolved.insert(key.clone(), Value::String(value.clone()));
                    }
                } else {
                    // This is a literal value
                    resolved.insert(key.clone(), Value::String(value.clone()));
                }
            }
            Some(Value::Object(resolved))
        } else {
            None
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error(\"Plan validation failed: {0}\")]
    ValidationError(String),
    #[error(\"Tool execution failed: {0}\")]
    ToolExecutionError(String),
    #[error(\"Timeout error: {0}\")]
    TimeoutError(String),
    #[error(\"Budget exceeded\")]
    BudgetExceeded,
}

pub struct Scheduler;

impl Scheduler {
    pub async fn execute_plan(&self, mut ctx: ExecutionContext, plan: &Plan) -> Result<ExecutionContext, ExecutionError> {
        // Validate the plan first
        plan.validate()
            .map_err(|e| ExecutionError::ValidationError(e.to_string()))?;

        // Process nodes in order respecting dependencies
        let mut remaining_nodes: Vec<&Node> = plan.nodes.iter().collect();
        let mut processed_count = 0;
        
        while !remaining_nodes.is_empty() && processed_count < 100 { // Prevent infinite loops
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
                    // Find all incoming edges to this node
                    let dependencies: Vec<&String> = edges
                        .iter()
                        .filter(|edge| edge.to == node.id)
                        .map(|edge| &edge.from)
                        .collect();
                    
                    // Check if all dependencies are completed
                    dependencies.iter().all(|dep| ctx.completed_nodes.contains(dep))
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
                        tracing::error!(\"Node {} execution failed: {}\", node.id, e);
                        return Err(e);
                    }
                }
            }
            
            if !executed_this_round {
                // No progress made, probably a circular dependency or missing dependencies
                return Err(ExecutionError::ValidationError(
                    \"No executable nodes found - possible circular dependency\".to_string()
                ));
            }
            
            // Check if we still have budget
            if !ctx.has_budget_remaining() {
                return Err(ExecutionError::BudgetExceeded);
            }
        }
        
        Ok(ctx)
    }

    async fn execute_node(&self, ctx: &mut ExecutionContext, node: &Node) -> Result<(), ExecutionError> {
        match &node.op {
            crate::internal::plan::ir::Operation::Call => {
                self.execute_call(ctx, node).await
            }
            crate::internal::plan::ir::Operation::Map => {
                self.execute_map(ctx, node).await
            }
            crate::internal::plan::ir::Operation::Reduce => {
                self.execute_reduce(ctx, node).await
            }
            crate::internal::plan::ir::Operation::Branch => {
                self.execute_branch(ctx, node).await
            }
            crate::internal::plan::ir::Operation::Assert => {
                self.execute_assert(ctx, node).await
            }
            crate::internal::plan::ir::Operation::Spawn => {
                self.execute_spawn(ctx, node).await
            }
            crate::internal::plan::ir::Operation::MemRead => {
                self.execute_mem_read(ctx, node).await
            }
            crate::internal::plan::ir::Operation::MemWrite => {
                self.execute_mem_write(ctx, node).await
            }
            crate::internal::plan::ir::Operation::Verify => {
                self.execute_verify(ctx, node).await
            }
            crate::internal::plan::ir::Operation::Retry => {
                self.execute_retry(ctx, node).await
            }
        }
    }

    async fn execute_call(&self, ctx: &mut ExecutionContext, node: &Node) -> Result<(), ExecutionError> {
        let tool_name = node.tool.as_ref().ok_or_else(|| {
            ExecutionError::ValidationError(format!(\"Node {} has no tool specified\", node.id))
        })?;
        
        let tool_url = ctx.tool_urls.get(tool_name).ok_or_else(|| {
            ExecutionError::ValidationError(format!(\"Tool {} not found in tool URLs\", tool_name))
        })?;
        
        let args = ctx.resolve_args(node.args.as_ref());
        
        // Add trace event
        let trace_event = crate::internal::trace::trace::Trace::new(
            \"step_start\".to_string(),
            node.id.clone(),
            format!(\"Calling tool: {}\", tool_name),
        );
        ctx.trace_events.push(trace_event);
        
        // Invoke the tool
        let result = timeout(
            Duration::from_secs(30), // 30 second timeout
            ctx.tool_client.invoke_tool(tool_url, args)
        )
        .await
        .map_err(|_| ExecutionError::TimeoutError(format!(\"Tool call {} timed out\", tool_name)))?
        .map_err(|e| ExecutionError::ToolExecutionError(e.to_string()))?;
        
        // Store the result in variables as specified by 'out' mapping
        if let Some(out_map) = &node.out {
            for (var_name, result_path) in out_map {
                // For now, store the full result
                // In a full implementation, we would extract specific fields based on result_path
                ctx.variables.insert(var_name.clone(), result.clone());
            }
        }
        
        // Add trace event
        let trace_event = crate::internal::trace::trace::Trace::new(
            \"step_end\".to_string(),
            node.id.clone(),
            format!(\"Tool {} call completed\", tool_name),
        );
        ctx.trace_events.push(trace_event);
        
        Ok(())
    }

    async fn execute_map(&self, ctx: &mut ExecutionContext, node: &Node) -> Result<(), ExecutionError> {
        // For map operations, we iterate over an input collection and apply the tool to each item
        let tool_name = node.tool.as_ref().ok_or_else(|| {
            ExecutionError::ValidationError(format!(\"Node {} has no tool specified\", node.id))
        })?;
        
        let tool_url = ctx.tool_urls.get(tool_name).ok_or_else(|| {
            ExecutionError::ValidationError(format!(\"Tool {} not found in tool URLs\", tool_name))
        })?;
        
        // Get the collection to map over
        let collection_var = node.args.as_ref()
            .and_then(|args| args.get(\"collection\"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ExecutionError::ValidationError(\"Map operation requires a 'collection' argument\".to_string())
            })?;
        
        let collection = ctx.variables.get(collection_var)
            .ok_or_else(|| {
                ExecutionError::ValidationError(format!(\"Collection variable {} not found\", collection_var))
            })?;
        
        if let Value::Array(items) = collection {
            let mut results = Vec::new();
            
            for (index, item) in items.iter().enumerate() {
                // Prepare arguments for this iteration
                let mut args = node.args.clone().unwrap_or_default();
                args.insert(\"item\".to_string(), item.clone());
                args.insert(\"index\".to_string(), Value::Number(index.into()));
                
                // Invoke the tool
                let result = timeout(
                    Duration::from_secs(30),
                    ctx.tool_client.invoke_tool(tool_url, Some(Value::Object(args)))
                )
                .await
                .map_err(|_| ExecutionError::TimeoutError(format!(\"Map operation item {} timed out\", index)))?
                .map_err(|e| ExecutionError::ToolExecutionError(e.to_string()))?;
                
                results.push(result);
            }
            
            // Store results
            if let Some(out_map) = &node.out {
                for (var_name, _) in out_map {
                    ctx.variables.insert(var_name.clone(), Value::Array(results));
                }
            }
        } else {
            return Err(ExecutionError::ValidationError(\"Map operation requires an array input\".to_string()));
        }
        
        Ok(())
    }

    async fn execute_reduce(&self, ctx: &mut ExecutionContext, node: &Node) -> Result<(), ExecutionError> {
        // For reduce operations, we combine multiple values into a single value
        let collection_var = node.args.as_ref()
            .and_then(|args| args.get(\"collection\"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ExecutionError::ValidationError(\"Reduce operation requires a 'collection' argument\".to_string())
            })?;
        
        let collection = ctx.variables.get(collection_var)
            .ok_or_else(|| {
                ExecutionError::ValidationError(format!(\"Collection variable {} not found\", collection_var))
            })?;
        
        if let Value::Array(items) = collection {
            // For now, perform a simple concatenation of string representations
            let mut result = String::new();
            for item in items {
                result.push_str(&item.to_string());
                result.push('\\n');
            }
            
            if let Some(out_map) = &node.out {
                for (var_name, _) in out_map {
                    ctx.variables.insert(var_name.clone(), Value::String(result));
                }
            }
        } else {
            return Err(ExecutionError::ValidationError(\"Reduce operation requires an array input\".to_string()));
        }
        
        Ok(())
    }

    async fn execute_branch(&self, _ctx: &mut ExecutionContext, _node: &Node) -> Result<(), ExecutionError> {
        // For now, just execute the branch operation without actual branching logic
        Ok(())
    }

    async fn execute_assert(&self, ctx: &mut ExecutionContext, node: &Node) -> Result<(), ExecutionError> {
        // Check an assertion about the current state
        let condition = node.args.as_ref()
            .and_then(|args| args.get(\"condition\"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ExecutionError::ValidationError(\"Assert operation requires a 'condition' argument\".to_string())
            })?;
        
        // For now, just log the assertion
        tracing::info!(\"Assertion: {}\", condition);
        
        // Check if the assertion passes
        // In a real implementation, we would evaluate the condition against the current context
        if condition == \"true\" {
            Ok(())
        } else {
            Err(ExecutionError::ValidationError(format!(\"Assertion failed: {}\", condition)))
        }
    }

    async fn execute_spawn(&self, _ctx: &mut ExecutionContext, _node: &Node) -> Result<(), ExecutionError> {
        // For now, just log the spawn operation
        tracing::info!(\"Spawn operation executed\");
        Ok(())
    }

    async fn execute_mem_read(&self, ctx: &mut ExecutionContext, node: &Node) -> Result<(), ExecutionError> {
        let key = node.args.as_ref()
            .and_then(|args| args.get("key"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ExecutionError::ValidationError("Memory read operation requires a 'key' argument".to_string())
            })?;

        // Get the memory store URL
        let mem_url = ctx.tool_urls.get("mesh.mem.sqlite")
            .ok_or_else(|| {
                ExecutionError::ValidationError("Memory tool 'mesh.mem.sqlite' not found in tool URLs".to_string())
            })?;

        // Create memory store client and read
        let mem_store = crate::internal::mem::store::MemoryStore::new();
        let result = mem_store.read(mem_url, key).await
            .map_err(|e| ExecutionError::ToolExecutionError(format!("Memory read failed: {}", e)))?;

        if let Some(entry) = result {
            // Store the value in the output variable
            if let Some(out_map) = &node.out {
                for (var_name, _) in out_map {
                    ctx.variables.insert(var_name.clone(), entry.value);
                }
            }
        } else {
            // If key not found, we can either error or set to null/undefined
            if let Some(out_map) = &node.out {
                for (var_name, _) in out_map {
                    ctx.variables.insert(var_name.clone(), serde_json::Value::Null);
                }
            }
        }

        Ok(())
    }

    async fn execute_mem_write(&self, ctx: &mut ExecutionContext, node: &Node) -> Result<(), ExecutionError> {
        let key = node.args.as_ref()
            .and_then(|args| args.get("key"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ExecutionError::ValidationError("Memory write operation requires a 'key' argument".to_string())
            })?;

        let value = node.args.as_ref()
            .and_then(|args| args.get("value"))
            .ok_or_else(|| {
                ExecutionError::ValidationError("Memory write operation requires a 'value' argument".to_string())
            })?;

        let provenance = node.args.as_ref()
            .and_then(|args| args.get("provenance"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
            });

        let confidence = node.args.as_ref()
            .and_then(|args| args.get("confidence"))
            .and_then(|v| v.as_f64());

        // Get the memory store URL
        let mem_url = ctx.tool_urls.get("mesh.mem.sqlite")
            .ok_or_else(|| {
                ExecutionError::ValidationError("Memory tool 'mesh.mem.sqlite' not found in tool URLs".to_string())
            })?;

        // If evidence is provided, validate it using the evidence verifier
        if let Some(evidence_str) = node.args.as_ref()
            .and_then(|args| args.get("evidence"))
            .and_then(|v| v.as_str()) {
            // Parse and validate the evidence
            let evidence: crate::internal::evidence::verify::Evidence = 
                serde_json::from_str(evidence_str)
                    .map_err(|e| ExecutionError::ValidationError(
                        format!("Invalid evidence format: {}", e)
                    ))?;
                    
            let verifier = crate::internal::evidence::verify::EvidenceVerifier;
            if let Err(e) = verifier.validate_evidence_for_storage(&evidence, 0.8) {
                return Err(ExecutionError::ValidationError(
                    format!("Evidence validation failed: {}", e)
                ));
            }
        } else if let Some(conf) = confidence {
            // If no evidence is provided, just check confidence threshold
            if conf < 0.8 {
                return Err(ExecutionError::ValidationError(
                    format!("Memory write rejected: confidence {} < 0.8 threshold", conf)
                ));
            }
        }

        // Create memory store client and write
        let mem_store = crate::internal::mem::store::MemoryStore::new();
        mem_store.write(mem_url, key, value, provenance.as_ref(), confidence, None).await
            .map_err(|e| ExecutionError::ToolExecutionError(format!("Memory write failed: {}", e)))?;

        Ok(())
    }

    async fn execute_verify(&self, ctx: &mut ExecutionContext, node: &Node) -> Result<(), ExecutionError> {
        // Get claims and sources from arguments
        let claims = node.args.as_ref()
            .and_then(|args| args.get("claims"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ExecutionError::ValidationError("Verify operation requires a 'claims' argument".to_string())
            })?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<String>>();

        let sources = node.args.as_ref()
            .and_then(|args| args.get("sources"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ExecutionError::ValidationError("Verify operation requires a 'sources' argument".to_string())
            })?;

        // Get the verify tool URL
        let verify_url = ctx.tool_urls.get("ground.verify")
            .ok_or_else(|| {
                ExecutionError::ValidationError("Verification tool 'ground.verify' not found in tool URLs".to_string())
            })?;

        // Prepare arguments for the verify tool
        let verify_args = serde_json::json!({
            "claims": claims,
            "sources": sources
        });

        // Invoke the verification tool
        let result = timeout(
            Duration::from_secs(30),
            ctx.tool_client.invoke_tool(verify_url, Some(verify_args))
        )
        .await
        .map_err(|_| ExecutionError::TimeoutError("Verification tool call timed out".to_string()))?
        .map_err(|e| ExecutionError::ToolExecutionError(format!("Verification failed: {}", e)))?;

        // Store the verification result in output variables
        if let Some(out_map) = &node.out {
            for (var_name, _) in out_map {
                ctx.variables.insert(var_name.clone(), result.clone());
            }
        }

        Ok(())
    }

    async fn execute_retry(&self, ctx: &mut ExecutionContext, node: &Node) -> Result<(), ExecutionError> {
        // Retry operation - execute the tool with retries
        let tool_name = node.tool.as_ref().ok_or_else(|| {
            ExecutionError::ValidationError(format!(\"Node {} has no tool specified\", node.id))
        })?;
        
        let tool_url = ctx.tool_urls.get(tool_name).ok_or_else(|| {
            ExecutionError::ValidationError(format!(\"Tool {} not found in tool URLs\", tool_name))
        })?;
        
        let args = ctx.resolve_args(node.args.as_ref());
        
        // Try up to 3 times
        let mut attempts = 0;
        let max_attempts = 3;
        
        loop {
            match timeout(
                Duration::from_secs(30),
                ctx.tool_client.invoke_tool(tool_url, args.clone())
            )
            .await
            {
                Ok(Ok(result)) => {
                    // Store the result in variables as specified by 'out' mapping
                    if let Some(out_map) = &node.out {
                        for (var_name, result_path) in out_map {
                            ctx.variables.insert(var_name.clone(), result.clone());
                        }
                    }
                    return Ok(());
                }
                Ok(Err(e)) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(ExecutionError::ToolExecutionError(e.to_string()));
                    }
                    tracing::warn!(\"Attempt {} failed for tool {}, retrying: {}\", attempts, tool_name, e);
                    tokio::time::sleep(Duration::from_millis(500)).await; // Wait before retry
                }
                Err(_) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(ExecutionError::TimeoutError(format!(\"Tool call {} timed out after {} attempts\", tool_name, max_attempts)));
                    }
                    tracing::warn!(\"Attempt {} timed out for tool {}, retrying\", attempts, tool_name);
                    tokio::time::sleep(Duration::from_millis(500)).await; // Wait before retry
                }
            }
        }
    }
}