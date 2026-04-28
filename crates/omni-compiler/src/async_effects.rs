use crate::ast::Stmt;
use crate::type_checker::Type;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AsyncFunction {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub body: Vec<Stmt>,
    pub effect: u8,
}

#[derive(Debug, Clone)]
pub struct FutureType {
    pub inner_type: Type,
    pub state: FutureState,
}

#[derive(Debug, Clone)]
pub enum FutureState {
    Pending,
    Ready(Type),
    Polling,
}

#[derive(Debug, Clone)]
pub struct AsyncContext {
    pub tasks: HashMap<String, AsyncTask>,
    pub spawned_count: usize,
}

#[derive(Debug)]
pub struct AsyncScope<'a> {
    context: &'a mut AsyncContext,
    spawned: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AsyncTask {
    pub name: String,
    pub future: FutureType,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Created,
    Running,
    Completed,
    Failed,
}

impl AsyncContext {
    pub fn new() -> Self {
        AsyncContext {
            tasks: HashMap::new(),
            spawned_count: 0,
        }
    }

    pub fn spawn_scope(&mut self) -> AsyncScope<'_> {
        AsyncScope {
            context: self,
            spawned: Vec::new(),
        }
    }

    pub fn spawn(&mut self, name: String, future: FutureType) -> String {
        let task_name = format!("_task_{}", self.spawned_count);
        self.spawned_count += 1;

        self.tasks.insert(
            task_name.clone(),
            AsyncTask {
                name: name.clone(),
                future,
                status: TaskStatus::Created,
            },
        );

        task_name
    }

    pub fn poll(&mut self, task_name: &str) -> Option<Type> {
        if let Some(task) = self.tasks.get_mut(task_name) {
            task.status = TaskStatus::Running;
            task.future.state = FutureState::Ready(task.future.inner_type.clone());
            task.status = TaskStatus::Completed;
            Some(task.future.inner_type.clone())
        } else {
            None
        }
    }

    pub fn join(&mut self, task_names: &[String]) -> Result<Type, String> {
        for name in task_names {
            let Some(task) = self.tasks.get_mut(name) else {
                return Err(format!("unknown task '{}'", name));
            };
            if task.status == TaskStatus::Failed {
                return Err(format!("task '{}' failed", name));
            }
            task.future.state = FutureState::Ready(task.future.inner_type.clone());
            task.status = TaskStatus::Completed;
        }

        Ok(Type::Unit)
    }
}

impl Default for AsyncContext {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> AsyncScope<'a> {
    pub fn spawn(&mut self, name: String, future: FutureType) -> String {
        let task_name = self.context.spawn(name, future);
        self.spawned.push(task_name.clone());
        task_name
    }

    pub fn finish(mut self) -> Result<Type, String> {
        let result = self.context.join(&self.spawned);
        self.cleanup_spawned_tasks();
        result
    }

    fn cleanup_spawned_tasks(&mut self) {
        let spawned = std::mem::take(&mut self.spawned);
        for task_name in spawned {
            self.context.tasks.remove(&task_name);
        }
    }
}

impl Drop for AsyncScope<'_> {
    fn drop(&mut self) {
        self.cleanup_spawned_tasks();
    }
}

pub const EF_ASYNC: u8 = 0b0100;

pub fn is_async_function(effect: u8) -> bool {
    effect & EF_ASYNC != 0
}

pub fn make_async(effect: u8) -> u8 {
    effect | EF_ASYNC
}

pub fn remove_async(effect: u8) -> u8 {
    effect & !EF_ASYNC
}

#[derive(Debug, Clone)]
pub struct AsyncTransform {
    pub generated_futures: HashMap<String, GeneratedFuture>,
}

#[derive(Debug, Clone)]
pub struct GeneratedFuture {
    pub name: String,
    pub state_enum: String,
    pub poll_method: String,
}

#[derive(Debug, Clone)]
pub struct Generator<T> {
    values: Vec<T>,
    index: usize,
}

impl AsyncTransform {
    pub fn new() -> Self {
        AsyncTransform {
            generated_futures: HashMap::new(),
        }
    }

    pub fn transform_function(&self, func: &AsyncFunction) -> Stmt {
        // Transform async function into state machine
        // This is a simplified version - real implementation would generate
        // a state enum and poll method

        let state_enum_name = format!("{}State", func.name);

        Stmt::Struct {
            name: state_enum_name,
            fields: vec![("state".to_string(), "int".to_string())],
            is_linear: false,
        }
    }

    pub fn generate_future_impl(&self, func: &AsyncFunction) -> Vec<Stmt> {
        let mut stmts = Vec::new();

        // Generate Future trait implementation
        stmts.push(Stmt::Struct {
            name: format!("{}Future", func.name),
            fields: vec![("__state".to_string(), "int".to_string())],
            is_linear: false,
        });

        // Generate poll method
        stmts.push(Stmt::Fn {
            name: format!("{}Future_poll", func.name),
            is_public: false,
            is_async: false,
            type_params: vec![],
            params: vec!["self".to_string()],
            ret_type: Some("Option".to_string()),
            effects: vec![],
            body: vec![],
        });

        stmts
    }
}

impl Default for AsyncTransform {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Generator<T> {
    pub fn new(values: Vec<T>) -> Self {
        Generator { values, index: 0 }
    }
}

impl<T: Clone> Iterator for Generator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.values.len() {
            return None;
        }

        let value = self.values[self.index].clone();
        self.index += 1;
        Some(value)
    }
}

pub fn make_generator<T: Clone>(values: Vec<T>) -> Generator<T> {
    Generator::new(values)
}

pub fn check_async_compatibility(caller_effect: u8, callee_effect: u8) -> Result<(), String> {
    // If caller is not async but callee is async, error
    if !is_async_function(caller_effect) && is_async_function(callee_effect) {
        return Err("Cannot call async function from non-async context".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct EffectPolymorphism {
    pub type_vars: HashMap<String, u8>,
}

impl EffectPolymorphism {
    pub fn new() -> Self {
        EffectPolymorphism {
            type_vars: HashMap::new(),
        }
    }

    pub fn add_effect_var(&mut self, name: &str, effect: u8) {
        self.type_vars.insert(name.to_string(), effect);
    }

    pub fn unify_effects(&self, e1: u8, e2: u8) -> Result<u8, String> {
        // If either is a variable/empty effect, return the concrete one.
        if e1 == 0 && e2 != 0 {
            return Ok(e2);
        }
        if e2 == 0 && e1 != 0 {
            return Ok(e1);
        }
        if e1 == e2 {
            return Ok(e1);
        }

        // For the staged bootstrap compiler, effect unification is modeled
        // as effect union so mixed-effect higher-order functions can compose.
        Ok(e1 | e2)
    }
}

impl Default for EffectPolymorphism {
    fn default() -> Self {
        Self::new()
    }
}

pub fn compose_effects(effects: &[u8]) -> u8 {
    effects.iter().fold(0, |acc, &e| acc | e)
}

pub fn restrict_effects(effect: u8, allowed: u8) -> u8 {
    effect & allowed
}

#[derive(Debug, Clone)]
pub struct VariadicGeneric {
    pub params: Vec<Type>,
}

impl VariadicGeneric {
    pub fn new() -> Self {
        VariadicGeneric { params: Vec::new() }
    }

    pub fn from_types(types: &[Type]) -> Self {
        VariadicGeneric {
            params: types.to_vec(),
        }
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Type> {
        self.params.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Type> {
        self.params.iter()
    }
}

impl Default for VariadicGeneric {
    fn default() -> Self {
        Self::new()
    }
}

pub fn make_variadic_fn(
    name: &str,
    variadic_param: &str,
    types: &[Type],
) -> (String, Vec<(String, Type)>) {
    let mut params = Vec::new();

    for (i, ty) in types.iter().enumerate() {
        params.push((format!("{}_{}", variadic_param, i), ty.clone()));
    }

    let full_name = format!("{}_variadic_{}", name, types.len());

    (full_name, params)
}
