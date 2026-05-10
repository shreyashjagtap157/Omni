use crate::type_checker::Type;

pub const EF_ASYNC: u32 = 0x02;
pub const EF_IO: u32 = 0x01;

#[derive(Debug, Clone)]
pub struct FutureType {
    pub inner_type: Type,
    pub state: FutureState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FutureState {
    Pending,
    Ready,
    PollError,
}

#[derive(Debug, Clone)]
pub struct AsyncContext {
    pub tasks: std::collections::HashMap<String, FutureType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    Pure,
    Io,
    Async,
    Throw(Type),
    Panic,
    Alloc,
    Rand,
    Time,
    Log,
    Custom(String),
}

impl Effect {
    pub fn is_pure(&self) -> bool {
        matches!(self, Effect::Pure)
    }

    pub fn composed_with(&self, other: &Effect) -> bool {
        match (self, other) {
            (Effect::Pure, Effect::Pure) => true,
            (Effect::Pure, _) => true,
            (_, Effect::Pure) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EffectSet {
    effects: Vec<Effect>,
}

impl EffectSet {
    pub fn new() -> EffectSet {
        EffectSet {
            effects: Vec::new(),
        }
    }

    pub fn empty() -> EffectSet {
        EffectSet {
            effects: vec![Effect::Pure],
        }
    }

    pub fn with_io() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Io);
        es
    }

    pub fn with_async() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Async);
        es
    }

    pub fn add(&mut self, effect: Effect) {
        if !self.effects.contains(&effect) {
            self.effects.push(effect);
        }
    }

    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    pub fn union(&self, other: &EffectSet) -> EffectSet {
        let mut result = self.clone();
        for e in &other.effects {
            result.add(e.clone());
        }
        result
    }

    pub fn to_string_list(&self) -> String {
        self.effects
            .iter()
            .map(|e| match e {
                Effect::Pure => "pure".to_string(),
                Effect::Io => "io".to_string(),
                Effect::Async => "async".to_string(),
                Effect::Throw(t) => format!("throw<{:?}>", t),
                Effect::Panic => "panic".to_string(),
                Effect::Alloc => "alloc".to_string(),
                Effect::Rand => "rand".to_string(),
                Effect::Time => "time".to_string(),
                Effect::Log => "log".to_string(),
                Effect::Custom(s) => s.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone)]
pub struct EffectHandler {
    pub effect_type: String,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub name: String,
    pub params: Vec<Type>,
    pub ret_type: Type,
}

impl EffectHandler {
    pub fn new(effect_type: &str) -> Self {
        EffectHandler {
            effect_type: effect_type.to_string(),
            operations: Vec::new(),
        }
    }

    pub fn add_operation(&mut self, op: Operation) {
        self.operations.push(op);
    }

    pub fn find_operation(&self, name: &str) -> Option<&Operation> {
        self.operations.iter().find(|op| op.name == name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CancellationToken {
    cancelled: bool,
    cancel_reason: Option<String>,
}

impl CancellationToken {
    pub fn new() -> Self {
        CancellationToken {
            cancelled: false,
            cancel_reason: None,
        }
    }

    pub fn check(&self) -> Result<(), String> {
        if self.cancelled {
            Err(self
                .cancel_reason
                .clone()
                .unwrap_or_else(|| "Cancelled".to_string()))
        } else {
            Ok(())
        }
    }

    pub fn cancel(&mut self, reason: Option<String>) {
        self.cancelled = true;
        self.cancel_reason = reason;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

#[derive(Debug, Clone)]
pub struct SpawnScope {
    pub policy: ScopePolicy,
    pub parent_id: u64,
    pub child_ids: Vec<u64>,
    pub max_children: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopePolicy {
    Detached,
    JoinAll,
    CancelOthers,
}

impl SpawnScope {
    pub fn new(parent_id: u64) -> Self {
        SpawnScope {
            parent_id,
            child_ids: Vec::new(),
            max_children: 1000,
            policy: ScopePolicy::JoinAll,
        }
    }

    pub fn spawn(&mut self, child_id: u64) -> Result<(), String> {
        if self.child_ids.len() >= self.max_children {
            return Err("Max children reached".to_string());
        }

        if self.policy == ScopePolicy::Detached {
            return Err("Cannot spawn in detached scope without capability".to_string());
        }

        self.child_ids.push(child_id);
        Ok(())
    }

    pub fn verify_all_done(&self) -> Result<(), String> {
        if !self.child_ids.is_empty() && self.policy == ScopePolicy::JoinAll {
            return Err(format!(
                "{} child tasks not completed before scope exit",
                self.child_ids.len()
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Channel<T> {
    buffer: Vec<T>,
    capacity: usize,
    closed: bool,
}

impl<T> Channel<T> {
    pub fn new(capacity: usize) -> Self {
        Channel {
            buffer: Vec::new(),
            capacity,
            closed: false,
        }
    }

    pub fn send(&mut self, value: T) -> Result<(), String> {
        if self.closed {
            return Err("Channel closed".to_string());
        }
        if self.buffer.len() >= self.capacity {
            return Err("Channel full".to_string());
        }
        self.buffer.push(value);
        Ok(())
    }

    pub fn receive(&mut self) -> Option<T> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(self.buffer.remove(0))
        }
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_set() {
        let mut es = EffectSet::new();
        es.add(Effect::Io);
        es.add(Effect::Async);

        assert!(es.contains(&Effect::Io));
        assert!(es.contains(&Effect::Async));
    }

    #[test]
    fn test_cancellation() {
        let mut token = CancellationToken::new();

        assert!(!token.is_cancelled());
        token.check().unwrap();

        token.cancel(Some("Test cancel".to_string()));

        assert!(token.is_cancelled());
        assert!(token.check().is_err());
    }

    #[test]
    fn test_spawn_scope() {
        let mut scope = SpawnScope::new(1);
        scope.spawn(2).unwrap();

        assert_eq!(scope.child_ids.len(), 1);
    }

    #[test]
    fn test_channel() {
        let mut ch: Channel<i32> = Channel::new(10);

        ch.send(42).unwrap();
        ch.send(43).unwrap();

        assert_eq!(ch.receive(), Some(42));
        assert_eq!(ch.receive(), Some(43));
    }
}
