use crate::async_effects::{CancellationToken, Channel, SpawnScope};
use crate::interpreter::Value;
use crate::macros::ComptimeContext;
use crate::security::{Capability, CapabilitySystem, CapabilityToken, FfiSandbox};
use std::collections::HashMap;

pub struct OmniInterpreter {
    pub env: HashMap<String, Value>,
    pub ffi_sandbox: FfiSandbox,
    pub capability_system: CapabilitySystem,
    pub comptime: ComptimeContext,
    pub channels: HashMap<String, Channel<Value>>,
    pub cancellation_tokens: HashMap<String, CancellationToken>,
}

impl OmniInterpreter {
    pub fn new() -> Self {
        OmniInterpreter {
            env: HashMap::new(),
            ffi_sandbox: FfiSandbox::new(64 * 1024, 1024 * 1024),
            capability_system: CapabilitySystem::new(),
            comptime: ComptimeContext::new(1000),
            channels: HashMap::new(),
            cancellation_tokens: HashMap::new(),
        }
    }

    pub fn register_capability(&mut self, name: &str, cap: Capability) {
        self.capability_system.register_capability(name, cap);
    }

    pub fn create_token(&mut self, name: &str, caps: Vec<Capability>) -> CapabilityToken {
        self.capability_system.create_token(name, caps)
    }

    pub fn check_capability(&self, token: &CapabilityToken, required: &Capability) -> bool {
        self.capability_system.check_token(token, required)
    }

    pub fn enable_ffi(&mut self) {
        self.ffi_sandbox.enable();
    }

    pub fn disable_ffi(&mut self) {
        self.ffi_sandbox.disable();
    }

    pub fn is_ffi_active(&self) -> bool {
        self.ffi_sandbox.is_active()
    }

    pub fn add_channel(&mut self, name: &str, capacity: usize) {
        let ch: Channel<Value> = Channel::new(capacity);
        self.channels.insert(name.to_string(), ch);
    }

    pub fn send_to_channel(&mut self, name: &str, value: Value) -> Result<(), String> {
        match self.channels.get_mut(name) {
            Some(ch) => ch.send(value),
            None => Err(format!("Channel {} not found", name)),
        }
    }

    pub fn receive_from_channel(&mut self, name: &str) -> Option<Value> {
        self.channels.get_mut(name).and_then(|ch| ch.receive())
    }

    pub fn add_cancellation_token(&mut self, name: &str) {
        let token = CancellationToken::new();
        self.cancellation_tokens.insert(name.to_string(), token);
    }

    pub fn cancel_token(&mut self, name: &str, reason: Option<String>) {
        if let Some(token) = self.cancellation_tokens.get_mut(name) {
            token.cancel(reason);
        }
    }

    pub fn is_token_cancelled(&self, name: &str) -> bool {
        self.cancellation_tokens
            .get(name)
            .map(|t| t.is_cancelled())
            .unwrap_or(false)
    }

    pub fn spawn_scope(&mut self, parent_id: u64) -> SpawnScope {
        SpawnScope::new(parent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Capability;

    #[test]
    fn test_interpreter_ffi() {
        let mut interp = OmniInterpreter::new();
        interp.enable_ffi();
        assert!(interp.is_ffi_active());

        interp.disable_ffi();
        assert!(!interp.is_ffi_active());
    }

    #[test]
    fn test_capability_system() {
        let mut interp = OmniInterpreter::new();
        interp.register_capability("io", Capability::Io);

        let token = interp.create_token("test", vec![Capability::Io]);
        assert!(interp.check_capability(&token, &Capability::Io));
    }

    #[test]
    fn test_channel() {
        let mut interp = OmniInterpreter::new();
        interp.add_channel("test", 10);

        interp.send_to_channel("test", Value::Int(42)).unwrap();
        let val = interp.receive_from_channel("test");
        assert_eq!(val, Some(Value::Int(42)));
    }

    #[test]
    fn test_cancellation_token() {
        let mut interp = OmniInterpreter::new();
        interp.add_cancellation_token("cancel1");

        assert!(!interp.is_token_cancelled("cancel1"));
        interp.cancel_token("cancel1", Some("test cancel".to_string()));
        assert!(interp.is_token_cancelled("cancel1"));
    }

    #[test]
    fn test_spawn_scope() {
        let mut interp = OmniInterpreter::new();
        let mut scope = interp.spawn_scope(1);
        scope.spawn(2).unwrap();
        assert_eq!(scope.child_ids.len(), 1);
    }
}
