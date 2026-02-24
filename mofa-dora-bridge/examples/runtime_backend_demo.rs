use mofa_dora_bridge::{DataflowController, RuntimeBackend};
use std::env;
use std::process;

fn parse_backend(value: &str) -> Result<RuntimeBackend, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dora" | "dora-cli" => Ok(RuntimeBackend::DoraCli),
        "mofa" | "mofa-native" => Ok(RuntimeBackend::MofaNative),
        other => Err(format!(
            "unsupported backend '{}', expected one of: dora-cli, mofa-native",
            other
        )),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let dataflow_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "apps/mofa-fm/dataflow/voice-chat.yml".to_string());
    let backend_str = env::var("MOFA_RUNTIME_BACKEND").unwrap_or_else(|_| "dora-cli".to_string());
    let start_requested = args.iter().any(|arg| arg == "--start");

    let backend = match parse_backend(&backend_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Invalid backend: {}", e);
            process::exit(2);
        }
    };

    println!("Runtime backend: {}", backend.as_str());
    println!("Dataflow path: {}", dataflow_path);

    let mut controller = match DataflowController::new_with_runtime(&dataflow_path, backend) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create DataflowController: {}", e);
            process::exit(1);
        }
    };

    if let Some(parsed) = controller.parsed() {
        println!(
            "Parsed dataflow: nodes={}, mofa_nodes={}, env_requirements={}",
            parsed.nodes.len(),
            parsed.mofa_nodes.len(),
            parsed.env_requirements.len()
        );
    }

    if !start_requested {
        println!("Dry run complete. Pass '--start' to execute lifecycle start/stop.");
        return;
    }

    println!("Starting dataflow...");
    match controller.start() {
        Ok(dataflow_id) => {
            println!("Dataflow started: {}", dataflow_id);
            println!("Stopping dataflow...");
            if let Err(e) = controller.stop() {
                eprintln!("Failed to stop dataflow: {}", e);
                process::exit(1);
            }
            println!("Dataflow stopped cleanly.");
        }
        Err(e) => {
            eprintln!("Failed to start dataflow: {}", e);
            process::exit(1);
        }
    }
}
