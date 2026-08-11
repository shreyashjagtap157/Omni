use std::collections::HashMap;
use std::sync::atomic::{AtomicPtr, Ordering};

pub struct CapabilitySystem {
    capabilities: HashMap<String, Capability>,
    tokens: HashMap<String, CapabilityToken>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    Io,
    Network { allowed_hosts: Vec<String> },
    Filesystem { allowed_paths: Vec<String> },
    Environment { allowed_vars: Vec<String> },
    Random,
    Time,
    Process { spawn: bool, exit: bool },
    Thread,
    Ffi,
}

#[derive(Debug, Clone)]
pub struct CapabilityToken {
    pub name: String,
    pub caps: Vec<Capability>,
    pub expired: bool,
}

impl CapabilitySystem {
    pub fn new() -> Self {
        CapabilitySystem {
            capabilities: HashMap::new(),
            tokens: HashMap::new(),
        }
    }

    pub fn register_capability(&mut self, name: &str, cap: Capability) {
        self.capabilities.insert(name.to_string(), cap);
    }

    pub fn create_token(&mut self, name: &str, caps: Vec<Capability>) -> CapabilityToken {
        let token = CapabilityToken {
            name: name.to_string(),
            caps,
            expired: false,
        };
        self.tokens.insert(name.to_string(), token.clone());
        token
    }

    pub fn check_token(&self, token: &CapabilityToken, required: &Capability) -> bool {
        if token.expired {
            return false;
        }
        token
            .caps
            .iter()
            .any(|c| self.capability_enables(c, required))
    }

    fn capability_enables(&self, granted: &Capability, required: &Capability) -> bool {
        match (granted, required) {
            (Capability::Io, Capability::Io)
            | (Capability::Random, Capability::Random)
            | (Capability::Time, Capability::Time)
            | (Capability::Thread, Capability::Thread)
            | (Capability::Ffi, Capability::Ffi) => true,
            (
                Capability::Network {
                    allowed_hosts: granted,
                },
                Capability::Network {
                    allowed_hosts: required,
                },
            ) => required.iter().all(|host| granted.contains(host)),
            (
                Capability::Filesystem {
                    allowed_paths: granted,
                },
                Capability::Filesystem {
                    allowed_paths: required,
                },
            ) => required.iter().all(|path| granted.contains(path)),
            (
                Capability::Environment {
                    allowed_vars: granted,
                },
                Capability::Environment {
                    allowed_vars: required,
                },
            ) => required.iter().all(|var| granted.contains(var)),
            (
                Capability::Process {
                    spawn: gs,
                    exit: ge,
                },
                Capability::Process {
                    spawn: rs,
                    exit: re,
                },
            ) => (!*rs || *gs) && (!*re || *ge),
            _ => false,
        }
    }

    pub fn revoke_token(&mut self, name: &str) {
        if let Some(token) = self.tokens.get_mut(name) {
            token.expired = true;
        }
    }
}

impl Default for CapabilitySystem {
    fn default() -> Self {
        CapabilitySystem::new()
    }
}

pub struct FfiSandbox {
    stack_ptr: AtomicPtr<u8>,
    stack_size: usize,
    pub memory_limit: usize,
    pub allowed_functions: HashMap<String, FfiSig>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct FfiSig {
    pub name: String,
    pub args: Vec<FfiType>,
    pub ret: FfiType,
    pub safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FfiType {
    Void,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Ptr,
    String,
}

impl FfiSandbox {
    pub fn new(stack_size: usize, memory_limit: usize) -> Self {
        FfiSandbox {
            stack_ptr: AtomicPtr::new(std::ptr::null_mut()),
            stack_size,
            memory_limit,
            allowed_functions: HashMap::new(),
            active: false,
        }
    }

    pub fn enable(&mut self) -> Result<(), FfiError> {
        if self.active {
            return Ok(());
        }
        let ptr = allocate_stack(self.stack_size)?;
        self.stack_ptr.store(ptr, Ordering::Relaxed);
        self.active = true;
        Ok(())
    }

    pub fn disable(&mut self) {
        self.active = false;
        let ptr = self.stack_ptr.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if !ptr.is_null() {
            deallocate_stack(ptr, self.stack_size);
        }
    }

    pub fn allow_function(&mut self, sig: FfiSig) {
        self.allowed_functions.insert(sig.name.clone(), sig);
    }

    pub fn call(&self, name: &str, args: &[FfiValue]) -> Result<FfiValue, FfiError> {
        if !self.active {
            return Err(FfiError::SandboxInactive);
        }

        let sig = self
            .allowed_functions
            .get(name)
            .ok_or_else(|| FfiError::FunctionNotAllowed(name.to_string()))?;
        if !sig.safe {
            return Err(FfiError::UnsafeCall(name.to_string()));
        }
        if sig.args.len() != args.len() {
            return Err(FfiError::InvalidSignature(format!(
                "{} expects {} arguments, got {}",
                name,
                sig.args.len(),
                args.len()
            )));
        }
        for (expected, actual) in sig.args.iter().zip(args.iter()) {
            if !ffi_value_matches(expected, actual) {
                return Err(FfiError::InvalidSignature(format!(
                    "argument type mismatch calling {}",
                    name
                )));
            }
        }

        // The current milestone deliberately has no foreign-call executor.
        // Returning a fabricated value would make an unperformed call appear
        // successful, so fail closed until the stable FFI ABI lands.
        Err(FfiError::BackendUnavailable)
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

fn allocate_stack(size: usize) -> Result<*mut u8, FfiError> {
    let layout = std::alloc::Layout::from_size_align(size.max(1), 16)
        .map_err(|_| FfiError::AllocationFailed)?;
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        Err(FfiError::AllocationFailed)
    } else {
        Ok(ptr)
    }
}

fn deallocate_stack(ptr: *mut u8, size: usize) {
    if ptr.is_null() {
        return;
    }
    if let Ok(layout) = std::alloc::Layout::from_size_align(size.max(1), 16) {
        unsafe { std::alloc::dealloc(ptr, layout) }
    }
}

#[derive(Debug, Clone)]
pub enum FfiValue {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Ptr(*const u8),
    String(String),
}

#[derive(Debug, Clone)]
pub enum FfiError {
    SandboxInactive,
    FunctionNotAllowed(String),
    UnsafeCall(String),
    MemoryLimitExceeded,
    StackOverflow,
    InvalidPointer,
    InvalidSignature(String),
    AllocationFailed,
    BackendUnavailable,
}

fn ffi_value_matches(expected: &FfiType, value: &FfiValue) -> bool {
    matches!(
        (expected, value),
        (FfiType::I8, FfiValue::I8(_))
            | (FfiType::I16, FfiValue::I16(_))
            | (FfiType::I32, FfiValue::I32(_))
            | (FfiType::I64, FfiValue::I64(_))
            | (FfiType::U8, FfiValue::U8(_))
            | (FfiType::U16, FfiValue::U16(_))
            | (FfiType::U32, FfiValue::U32(_))
            | (FfiType::U64, FfiValue::U64(_))
            | (FfiType::F32, FfiValue::F32(_))
            | (FfiType::F64, FfiValue::F64(_))
            | (FfiType::Ptr, FfiValue::Ptr(_))
            | (FfiType::String, FfiValue::String(_))
    )
}

impl Drop for FfiSandbox {
    fn drop(&mut self) {
        self.disable();
    }
}

pub struct BindingsGenerator {
    c_headers: HashMap<String, String>,
}

impl BindingsGenerator {
    pub fn new() -> Self {
        BindingsGenerator {
            c_headers: HashMap::new(),
        }
    }

    pub fn parse_header(&mut self, name: &str, content: &str) {
        self.c_headers.insert(name.to_string(), content.to_string());
    }

    pub fn generate_omni(&self, header: &str) -> String {
        let mut output = String::new();
        output.push_str(&format!("-- Bindings generated from {}\n\n", header));

        for line in self.c_headers.get(header).unwrap_or(&String::new()).lines() {
            if let Some(omni_fn) = self.parse_c_fn(line) {
                output.push_str(&omni_fn);
                output.push('\n');
            }
        }

        output
    }

    fn parse_c_fn(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.starts_with("extern \"C\"")
            || trimmed.starts_with("int ")
            || trimmed.starts_with("void ")
        {
            if let Some(name_end) = trimmed.find('(') {
                let ret_and_name = &trimmed[..name_end];
                let parts: Vec<&str> = ret_and_name.split_whitespace().collect();
                if parts.len() >= 2 {
                    let _ret = parts[0];
                    let name = parts[1];
                    return Some(format!("extern fn {}(...)\n", name));
                }
            }
        }
        None
    }
}

impl Default for BindingsGenerator {
    fn default() -> Self {
        BindingsGenerator::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_system() {
        let mut sys = CapabilitySystem::new();
        sys.register_capability("io", Capability::Io);

        let token = sys.create_token("test", vec![Capability::Io]);
        assert!(sys.check_token(&token, &Capability::Io));
    }

    #[test]
    fn test_ffi_sandbox() {
        let mut sandbox = FfiSandbox::new(64 * 1024, 1024 * 1024);
        sandbox.enable().expect("sandbox allocation");

        assert!(sandbox.is_active());

        sandbox.allow_function(FfiSig {
            name: "test_fn".to_string(),
            args: vec![],
            ret: FfiType::I32,
            safe: true,
        });

        let result = sandbox.call("test_fn", &[]);
        assert!(matches!(result, Err(FfiError::BackendUnavailable)));

        sandbox.disable();
        assert!(!sandbox.is_active());
    }
}
