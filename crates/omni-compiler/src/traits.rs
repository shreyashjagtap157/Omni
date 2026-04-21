use crate::ast::Stmt;
use crate::type_checker::Type;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct TraitDefinition {
    pub name: String,
    pub type_params: Vec<String>,
    pub bounds: Vec<TraitBound>,
    pub supertraits: Vec<String>,
    pub methods: Vec<MethodSignature>,
    pub required_methods: Vec<String>,
    pub is_sealed: bool,
}

#[derive(Debug, Clone)]
pub struct TraitBound {
    pub trait_name: String,
    pub for_type: Type,
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub effect: u8,
}

#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_name: String,
    pub impl_type: Type,
    pub methods: Vec<ImplMethod>,
}

impl PartialEq for TraitImpl {
    fn eq(&self, other: &Self) -> bool {
        self.trait_name == other.trait_name && self.impl_type == other.impl_type
    }
}

#[derive(Debug, Clone)]
pub struct ImplMethod {
    pub name: String,
    pub body: Vec<Stmt>,
}

impl PartialEq for ImplMethod {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Debug, Clone)]
pub struct TraitSystem {
    pub traits: HashMap<String, TraitDefinition>,
    pub impls: Vec<TraitImpl>,
    pub resolved_bounds: HashMap<String, Vec<TraitBound>>,
}

impl TraitSystem {
    pub fn new() -> Self {
        let mut system = TraitSystem {
            traits: HashMap::new(),
            impls: Vec::new(),
            resolved_bounds: HashMap::new(),
        };

        // Add built-in traits
        system.add_builtin_traits();

        system
    }

    fn add_builtin_traits(&mut self) {
        // Clone trait
        self.traits.insert(
            "Clone".to_string(),
            TraitDefinition {
                name: "Clone".to_string(),
                type_params: vec!["Self".to_string()],
                bounds: vec![],
                supertraits: vec![],
                methods: vec![MethodSignature {
                    name: "clone".to_string(),
                    params: vec![],
                    return_type: Type::Generic("Self".to_string()),
                    effect: 0,
                }],
                required_methods: vec!["clone".to_string()],
                is_sealed: false,
            },
        );

        // Drop trait
        self.traits.insert(
            "Drop".to_string(),
            TraitDefinition {
                name: "Drop".to_string(),
                type_params: vec!["Self".to_string()],
                bounds: vec![],
                supertraits: vec![],
                methods: vec![MethodSignature {
                    name: "drop".to_string(),
                    params: vec![],
                    return_type: Type::Unit,
                    effect: 0,
                }],
                required_methods: vec!["drop".to_string()],
                is_sealed: false,
            },
        );

        // Debug trait
        self.traits.insert(
            "Debug".to_string(),
            TraitDefinition {
                name: "Debug".to_string(),
                type_params: vec!["Self".to_string()],
                bounds: vec![],
                supertraits: vec![],
                methods: vec![MethodSignature {
                    name: "fmt".to_string(),
                    params: vec![],
                    return_type: Type::String,
                    effect: 0,
                }],
                required_methods: vec!["fmt".to_string()],
                is_sealed: false,
            },
        );

        // Eq trait
        self.traits.insert(
            "Eq".to_string(),
            TraitDefinition {
                name: "Eq".to_string(),
                type_params: vec!["Self".to_string()],
                bounds: vec![],
                supertraits: vec![],
                methods: vec![],
                required_methods: vec![],
                is_sealed: false,
            },
        );

        // PartialEq trait
        self.traits.insert(
            "PartialEq".to_string(),
            TraitDefinition {
                name: "PartialEq".to_string(),
                type_params: vec!["Self".to_string()],
                bounds: vec![],
                supertraits: vec!["Eq".to_string()],
                methods: vec![MethodSignature {
                    name: "eq".to_string(),
                    params: vec![("other".to_string(), Type::Generic("Self".to_string()))],
                    return_type: Type::Bool,
                    effect: 0,
                }],
                required_methods: vec!["eq".to_string()],
                is_sealed: false,
            },
        );

        // Iterator trait
        self.traits.insert(
            "Iterator".to_string(),
            TraitDefinition {
                name: "Iterator".to_string(),
                type_params: vec!["Self".to_string(), "Item".to_string()],
                bounds: vec![],
                supertraits: vec![],
                methods: vec![MethodSignature {
                    name: "next".to_string(),
                    params: vec![],
                    return_type: Type::Generic("Item".to_string()),
                    effect: 0,
                }],
                required_methods: vec!["next".to_string()],
                is_sealed: false,
            },
        );

        // Default trait
        self.traits.insert(
            "Default".to_string(),
            TraitDefinition {
                name: "Default".to_string(),
                type_params: vec!["Self".to_string()],
                bounds: vec![],
                supertraits: vec![],
                methods: vec![MethodSignature {
                    name: "default".to_string(),
                    params: vec![],
                    return_type: Type::Generic("Self".to_string()),
                    effect: 0,
                }],
                required_methods: vec!["default".to_string()],
                is_sealed: false,
            },
        );
    }

    pub fn add_trait(&mut self, trait_def: TraitDefinition) -> Result<(), String> {
        if self.traits.contains_key(&trait_def.name) {
            return Err(format!("Trait '{}' already defined", trait_def.name));
        }
        self.traits.insert(trait_def.name.clone(), trait_def);
        Ok(())
    }

    pub fn add_impl(&mut self, impl_def: TraitImpl) -> Result<(), String> {
        // Check trait exists
        if !self.traits.contains_key(&impl_def.trait_name) {
            return Err(format!("Trait '{}' not found", impl_def.trait_name));
        }

        // Check all required methods are implemented
        let trait_def = &self.traits[&impl_def.trait_name];
        let impl_methods: HashSet<&String> = impl_def.methods.iter().map(|m| &m.name).collect();

        for required in &trait_def.required_methods {
            if !impl_methods.contains(required) {
                return Err(format!(
                    "Missing implementation for required method '{}' in trait '{}'",
                    required, impl_def.trait_name
                ));
            }
        }

        self.impls.push(impl_def);
        Ok(())
    }

    pub fn get_impls_for_type(&self, ty: &Type) -> Vec<&TraitImpl> {
        self.impls.iter().filter(|i| &i.impl_type == ty).collect()
    }

    pub fn check_trait_bound(&self, ty: &Type, trait_name: &str) -> bool {
        // Check if type implements the trait
        self.impls
            .iter()
            .any(|i| &i.impl_type == ty && i.trait_name == trait_name)
    }

    pub fn satisfies_negative_bound(&self, ty: &Type, trait_name: &str) -> bool {
        !self.check_trait_bound(ty, trait_name)
    }

    fn supertraits_of(&self, trait_name: &str) -> Vec<String> {
        self.traits
            .get(trait_name)
            .map(|t| t.supertraits.clone())
            .unwrap_or_default()
    }

    pub fn can_upcast_trait(&self, subtrait: &str, supertrait: &str) -> bool {
        if subtrait == supertrait {
            return true;
        }

        let mut visited = HashSet::new();
        let mut stack = vec![subtrait.to_string()];

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            for parent in self.supertraits_of(&current) {
                if parent == supertrait {
                    return true;
                }
                stack.push(parent);
            }
        }

        false
    }

    pub fn implied_bounds_for_type(&self, ty: &Type) -> Vec<TraitBound> {
        let mut implied = Vec::new();
        let mut seen = HashSet::new();

        for impl_def in self.impls.iter().filter(|i| &i.impl_type == ty) {
            let mut stack = vec![impl_def.trait_name.clone()];
            while let Some(trait_name) = stack.pop() {
                if !seen.insert(trait_name.clone()) {
                    continue;
                }
                implied.push(TraitBound {
                    trait_name: trait_name.clone(),
                    for_type: ty.clone(),
                });
                for parent in self.supertraits_of(&trait_name) {
                    stack.push(parent);
                }
            }
        }

        implied
    }

    pub fn resolve_implied_bounds(&mut self, type_name: &str, type_bounds: &[String]) {
        let implied: Vec<TraitBound> = type_bounds
            .iter()
            .flat_map(|bound| {
                let ty = Type::Generic(bound.clone());
                self.implied_bounds_for_type(&ty)
            })
            .collect();

        self.resolved_bounds.insert(type_name.to_string(), implied);
    }
}

pub fn check_trait_satisfaction(
    trait_system: &TraitSystem,
    impl_type: &Type,
    trait_name: &str,
) -> Result<(), String> {
    if trait_system.check_trait_bound(impl_type, trait_name) {
        Ok(())
    } else {
        Err(format!(
            "Type {:?} does not implement trait '{}'",
            impl_type, trait_name
        ))
    }
}
