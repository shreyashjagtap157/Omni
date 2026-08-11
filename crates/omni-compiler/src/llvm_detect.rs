use std::path::Path;

pub struct LlvConfig {
    pub version: String,
    pub prefix: Option<String>,
    pub inkwell_available: bool,
}

pub fn detect_llvm() -> Option<LlvConfig> {
    let prefixes = [
        std::env::var("LLVM_SYS_140_PREFIX").ok(),
        std::env::var("LLVM_SYS_18_PREFIX").ok(),
    ];

    for prefix in prefixes.iter().flatten() {
        let path = Path::new(prefix);
        if path.exists() {
            return Some(LlvConfig {
                version: "14.0".to_string(),
                prefix: Some(prefix.clone()),
                inkwell_available: true,
            });
        }
    }

    if let Ok(llvm_sys) = std::env::var("LLVM_SYS_PREFIX") {
        if !llvm_sys.is_empty() {
            return Some(LlvConfig {
                version: "unknown".to_string(),
                prefix: Some(llvm_sys),
                inkwell_available: true,
            });
        }
    }

    fn check_system_llvm() -> Option<LlvConfig> {
        let clang_paths = [
            "C:/Program Files/LLVM/bin",
            "C:/Program Files (x86)/LLVM/bin",
            "/usr/bin",
            "/usr/local/bin",
        ];

        for base in &clang_paths {
            let clang = Path::new(base).join("clang.exe");
            if clang.exists() {
                return Some(LlvConfig {
                    version: "system".to_string(),
                    prefix: Some(base.to_string()),
                    inkwell_available: true,
                });
            }
        }
        None
    }

    check_system_llvm()
}

pub fn is_llvm_available() -> bool {
    detect_llvm().is_some()
}

pub fn get_llvm_version() -> String {
    detect_llvm()
        .map(|c| c.version)
        .unwrap_or_else(|| "not found".to_string())
}
