use codegen_llvm::{can_use_real_llvm, get_llvm_version, is_llvm_available};

#[test]
fn llvm_detection_reports_status() {
    let available = is_llvm_available();
    let version = get_llvm_version();

    println!("LLVM available: {}", available);
    println!("LLVM version: {}", version);
}

#[test]
fn can_check_real_llvm_capability() {
    let can_use = can_use_real_llvm();
    println!("Can use real LLVM: {}", can_use);
}
