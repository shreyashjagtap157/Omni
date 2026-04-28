use omni_selfhost::bootstrap::{run_self_host_pipeline, verify_stage0_works, SelfHostError};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "verify" => {
                if let Err(e) = verify_stage0_works() {
                    eprintln!("Verification failed: {}", e);
                    std::process::exit(1);
                }
            }
            "self-host" | "pipeline" => {
                if let Err(e) = run_self_host_pipeline() {
                    eprintln!("Self-hosting pipeline failed: {}", e);
                    std::process::exit(1);
                }
            }
            "help" | "--help" | "-h" => {
                println!("Omni Self-Host Pipeline");
                println!();
                println!("Commands:");
                println!("  verify      - Verify Stage0 (Rust compiler) works");
                println!(
                    "  self-host   - Run full self-hosting pipeline (Stage0 -> Stage1 -> Stage2)"
                );
                println!("  pipeline    - Alias for self-host");
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
                eprintln!("Run with 'help' for usage");
                std::process::exit(1);
            }
        }
    } else {
        // Default: run verification then self-host pipeline
        if let Err(e) = verify_stage0_works() {
            eprintln!("Verification failed: {}", e);
            std::process::exit(1);
        }

        if let Err(e) = run_self_host_pipeline() {
            eprintln!("Self-hosting pipeline failed: {}", e);
            std::process::exit(1);
        }

        eprintln!("\n=== Self-hosting pipeline completed successfully ===");
    }
}
