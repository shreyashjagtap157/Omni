#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinearState {
    Available,
    Moved,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    Pure,
    Io,
    Async,
    Throw(Box<Type>),
    Panic,
    Alloc,
    Rand,
    Time,
    Log,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectSet {
    pub effects: Vec<Effect>,
}

impl Default for EffectSet {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn with_pure() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Pure);
        es
    }

    pub fn with_panic() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Panic);
        es
    }

    pub fn with_alloc() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Alloc);
        es
    }

    pub fn with_rand() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Rand);
        es
    }

    pub fn with_time() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Time);
        es
    }

    pub fn with_log() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Log);
        es
    }

    pub fn with_throw(ty: Type) -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Throw(Box::new(ty)));
        es
    }

    pub fn with_custom(name: String) -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Custom(name));
        es
    }

    pub fn from_effect(effect: Effect) -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(effect);
        es
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
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

    pub fn union_with(&mut self, other: &EffectSet) {
        for e in &other.effects {
            self.add(e.clone());
        }
    }

    pub fn difference(&self, other: &EffectSet) -> EffectSet {
        let mut result = EffectSet::new();
        for e in &self.effects {
            if !other.contains(e) {
                result.add(e.clone());
            }
        }
        result
    }

    pub fn to_string_list(&self) -> String {
        self.effects_to_strings().join(", ")
    }

    pub fn non_pure_effect_strings(&self) -> Vec<String> {
        self.effects
            .iter()
            .filter(|e| !matches!(e, Effect::Pure))
            .map(|e| self.effect_to_string(e))
            .collect()
    }

    fn effects_to_strings(&self) -> Vec<String> {
        self.effects
            .iter()
            .map(|e| self.effect_to_string(e))
            .collect()
    }

    fn effect_to_string(&self, e: &Effect) -> String {
        match e {
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
        }
    }
}

impl std::ops::BitOr for EffectSet {
    type Output = EffectSet;
    fn bitor(self, rhs: EffectSet) -> Self::Output {
        self.union(&rhs)
    }
}

impl std::ops::BitOrAssign for EffectSet {
    fn bitor_assign(&mut self, rhs: EffectSet) {
        self.union_with(&rhs);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Float,
    Char,
    Byte,
    String,
    Bytes,
    Bool,
    Var(u32),
    Generic(String),
    Ref {
        mutable: bool,
        inner: Box<Type>,
    },
    Fn {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: EffectSet,
    },
    Struct {
        name: String,
        fields: Vec<Type>,
        is_linear: bool,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
        is_sealed: bool,
    },
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    ErrorSet(String),
    Unit,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Type>,
}
