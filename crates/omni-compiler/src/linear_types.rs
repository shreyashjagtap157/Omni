use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearKind {
    None,
    Linear,
    Affine,
    Owned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    Static,
    Dynamic,
    Polonius,
}

pub struct LinearTypeChecker {
    consumed: HashMap<String, LinearState>,
    linear_kinds: HashMap<String, LinearKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinearState {
    Available,
    Consumed,
    Moved,
}

impl LinearTypeChecker {
    pub fn new() -> Self {
        LinearTypeChecker {
            consumed: HashMap::new(),
            linear_kinds: HashMap::new(),
        }
    }
    
    pub fn register_linear(&mut self, name: &str, kind: LinearKind) {
        self.linear_kinds.insert(name.to_string(), kind);
    }
    
    pub fn is_linear(&self, name: &str) -> bool {
        self.linear_kinds
            .get(name)
            .map(|k| matches!(k, LinearKind::Linear))
            .unwrap_or(false)
    }
    
    pub fn mark_consumed(&mut self, name: &str) -> Result<(), String> {
        if self.is_linear(name) {
            if let Some(state) = self.consumed.get(name) {
                if matches!(state, LinearState::Consumed) {
                    return Err(format!("Linear value `{}` already used", name));
                }
                if matches!(state, LinearState::Moved) {
                    return Err(format!("Linear value `{}` already moved", name));
                }
            }
            self.consumed.insert(name.to_string(), LinearState::Consumed);
        }
        Ok(())
    }
    
    pub fn check_available(&self, name: &str) -> Result<(), String> {
        if self.is_linear(name) {
            if let Some(state) = self.consumed.get(name) {
                return Err(format!("Linear value `{}` already consumed", name));
            }
        }
        Ok(())
    }
    
    pub fn check_scope_exit(&self, stmts: &[String]) -> Vec<String> {
        let mut errors = Vec::new();
        for name in stmts {
            if self.is_linear(name) {
                if let Some(state) = self.consumed.get(name) {
                    if matches!(state, LinearState::Available) {
                        errors.push(format!(
                            "Linear value `{}` not used before scope exit",
                            name
                        ));
                    }
                }
            }
        }
        errors
    }
}

pub struct InOutDesugar {
    inout_params: Vec<(String, Type)>,
}

impl InOutDesugar {
    pub fn new() -> Self {
        InOutDesugar {
            inout_params: Vec::new(),
        }
    }
    
    pub fn detect_inout(&mut self, params: &[String]) -> Vec<(String, bool, Type)> {
        let mut result = Vec::new();
        for p in params {
            if p.starts_with("inout_") {
                let name = p.strip_prefix("inout_").unwrap_or(p);
                result.push((name.to_string(), true, Type::Unknown));
            } else {
                result.push((p.clone(), false, Type::Unknown));
            }
        }
        result
    }
    
    pub fn desugar_signature(&self, sig: &str) -> String {
        let mut result = String::new();
        let params: Vec<&str> = sig.split(',').collect();
        
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                result.push_str(", ");
            }
            
            if p.contains("inout ") {
                result.push_str("__in_");
                result.push_str(p.replace("inout ", "").as_str());
                result.push_str(": ");
                result.push_str(p.split(':').nth(1).unwrap_or(""));
            } else {
                result.push_str(p);
            }
        }
        
        result
    }
    
    pub fn generate_move_in(&self, param: &str) -> Vec<String> {
        let real_name = param.strip_prefix("inout_").unwrap_or(param);
        vec![
            format!("let __in_{} = {};", real_name, param),
        ]
    }
    
    pub fn generate_move_out(&self, param: &str) -> Vec<String> {
        let real_name = param.strip_prefix("inout_").unwrap_or(param);
        vec![
            format!("{} = __out_{};", real_name, real_name),
        ]
    }
}

pub struct FfiSandbox {
    allowed_calls: HashMap<String, bool>,
    io_caps: Vec<String>,
    fs_caps: Vec<String>,
    memory_limit: usize,
    stack_size: usize,
}

impl FfiSandbox {
    pub fn new() -> Self {
        FfiSandbox {
            allowed_calls: HashMap::new(),
            io_caps: Vec::new(),
            fs_caps: Vec::new(),
            1024 * 1024,
            64 * 1024,
        }
    }
    
    pub fn grant_capability(&mut self, capability: &str) {
        match capability {
            "io" => self.io_caps.push("io".to_string()),
            "fs" | "filesystem" => self.fs_caps.push("filesystem".to_string()),
            _ => {}
        }
    }
    
    pub fn has_capability(&self, cap: &str) -> bool {
        self.io_caps.contains(&cap.to_string()) ||
        self.fs_caps.contains(&cap.to_string())
    }
    
    pub fn can_call(&self, fn_name: &str) -> bool {
        self.allowed_calls.get(fn_name).copied().unwrap_or(false)
    }
    
    pub fn allow_call(&mut self, fn_name: &str) {
        self.allowed_calls.insert(fn_name.to_string(), true);
    }
    
    pub fn check(&self, call: &str) -> Result<(), String> {
        if self.can_call(call) {
            Ok(())
        } else {
            Err(format!("FFI call `{}` not allowed in sandbox", call))
        }
    }
}

#[derive(Debug, Clone)]
pub enum Type {
    Unknown,
    Int,
    String,
    Bool,
    Ptr(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_linear_check() {
        let mut checker = LinearTypeChecker::new();
        checker.register_linear("tx", LinearKind::Linear);
        
        assert!(checker.is_linear("tx"));
        
        checker.mark_consumed("tx").unwrap();
        let result = checker.mark_consumed("tx");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_inout_simple() {
        let mut desugar = InOutDesugar::new();
        let params = vec!["inout_x".to_string(), "y".to_string()];
        let result = desugar.detect_inout(&params);
        
        assert_eq!(result.len(), 2);
        assert!(result[0].1);
        assert!(!result[1].1);
    }
    
    #[test]
    fn test_ffi_sandbox() {
        let mut sandbox = FfiSandbox::new();
        sandbox.grant_capability("io");
        
        assert!(sandbox.has_capability("io"));
        assert!(!sandbox.has_capability("fs"));
    }
}