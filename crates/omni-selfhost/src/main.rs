use omni_selfhost::bootstrap::{run_self_host_pipeline, verify_stage0_works};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("verify");

    match command {
        "verify" => {
            if let Err(error) = verify_stage0_works() {
                eprintln!("Stage-0 verification failed: {error}");
                std::process::exit(1);
            }
            println!("Stage-0 Rust bootstrap verification passed");
        }
        "self-host" | "pipeline" => {
            if let Err(error) = run_self_host_pipeline() {
                eprintln!("Self-hosting unavailable: {error}");
                std::process::exit(2);
            }
        }
        "help" | "--help" | "-h" => {
            println!("Omni bootstrap verification");
            println!();
            println!("Commands:");
            println!("  verify      Build/smoke-test the current Rust Stage-0 compiler");
            println!("  self-host   Explicitly report whether real Stage-1/Stage-2 self-hosting is qualified");
            println!("  pipeline    Alias for self-host");
            println!();
            println!(
                "v0.1.4 does not claim self-hosting. The canonical compiler is still Rust-hosted."
            );
        }
        other => {
            eprintln!("Unknown command: {other}");
            eprintln!("Run with 'help' for usage");
            std::process::exit(1);
        }
    }
}
