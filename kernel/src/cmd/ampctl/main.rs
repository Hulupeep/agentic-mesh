use clap::{Parser, Subcommand};
use amp::internal::{
    api::AppState,
    exec::scheduler::{ExecutionContext, Scheduler},
    plan::ir::Plan,
};
use std::fs;
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(name = "ampctl")]
#[command(about = "Agentic Mesh Protocol CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a plan file
    Run {
        /// Path to the plan file
        #[arg(short, long)]
        plan_file: String,
        
        /// Path to variables file (JSON)
        #[arg(short, long)]
        vars_file: Option<String>,
        
        /// Output file for results
        #[arg(short, long)]
        out: Option<String>,
    },
    /// Stream trace for a plan
    Trace {
        /// Plan ID to trace
        #[arg(long)]
        plan_id: String,
    },
    /// Create a replay bundle
    Bundle {
        /// Plan ID to bundle
        #[arg(long)]
        plan_id: String,
        
        /// Output file for bundle
        #[arg(short, long)]
        out: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Run { plan_file, vars_file, out } => {
            run_plan(plan_file, vars_file, out).await?;
        },
        Commands::Trace { plan_id } => {
            trace_plan(plan_id).await?;
        },
        Commands::Bundle { plan_id, out } => {
            create_bundle(plan_id, out).await?;
        }
    }
    
    Ok(())
}

async fn run_plan(plan_file: &str, vars_file: &Option<String>, out: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Read the plan file
    let plan_content = fs::read_to_string(plan_file)?;
    let plan: Plan = serde_json::from_str(&plan_content)?;
    
    // Read variables if provided
    let mut ctx = ExecutionContext::new();
    if let Some(vars_path) = vars_file {
        let vars_content = fs::read_to_string(vars_path)?;
        let vars: serde_json::Value = serde_json::from_str(&vars_content)?;
        if let serde_json::Value::Object(map) = vars {
            ctx.variables = map;
        }
    }
    
    // Set up tool URLs (these should match the adapter ports)
    ctx.tool_urls.insert("doc.search.local".to_string(), "http://localhost:7401".to_string());
    ctx.tool_urls.insert("ground.verify".to_string(), "http://localhost:7402".to_string());
    ctx.tool_urls.insert("mesh.mem.sqlite".to_string(), "http://localhost:7403".to_string());
    
    // Set signals from plan
    ctx.signals = plan.signals.clone();
    
    // Execute the plan
    let scheduler = Scheduler;
    let result = scheduler.execute_plan(ctx, &plan).await;
    
    match result {
        Ok(final_ctx) => {
            // Output result
            let output = serde_json::json!({
                "status": "completed",
                "variables": final_ctx.variables,
                "trace_count": final_ctx.trace_events.len(),
                "completed_nodes": final_ctx.completed_nodes,
            });
            
            if let Some(out_path) = out {
                fs::write(out_path, serde_json::to_string_pretty(&output)?)?;
                println!("Plan completed. Results written to {}", out_path);
            } else {
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
            
            Ok(())
        }
        Err(e) => {
            eprintln!("Plan execution failed: {}", e);
            Err(Box::new(e))
        }
    }
}

async fn trace_plan(plan_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // In a real implementation, this would connect to a running kernel instance
    // For now, let's just indicate this functionality
    println!("Tracing plan: {}", plan_id);
    println!("This would connect to the kernel API to stream trace events for plan ID: {}", plan_id);
    
    Ok(())
}

async fn create_bundle(plan_id: &str, out: &str) -> Result<(), Box<dyn std::error::Error>> {
    // In a real implementation, this would connect to a running kernel instance
    // and create a bundle of plan + traces + toolspecs
    println!("Creating bundle for plan: {}", plan_id);
    println!("Bundle would be written to: {}", out);
    
    Ok(())
}