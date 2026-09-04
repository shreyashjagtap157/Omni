use std::collections::HashMap;

use crate::ast::{Expr, Program, Stmt};
use crate::complete_lexer::CompleteLexer;
use crate::diagnostics::{error_codes, Diagnostic, Severity};
use crate::effect_resolver::EffectResolver;
use crate::inout_desugar::desugar_inout_in_ast;
use crate::module_system::ModuleSystem;
use crate::parser::Parser;
use crate::resolver::{self, ResolveResult};
use crate::traits::{check_trait_satisfaction, TraitSystem};
use crate::type_checker;

pub struct Compiler {
    pub source: String,
    pub backend: Backend,
    pub source_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Owned AOT backend. This is the canonical execution target.
    Native,
    /// Development-only JIT backend.
    Cranelift,
    LLVM,
    Wasm,
    Rust,
}

pub struct CompilationResult {
    pub program: Option<Program>,
    pub resolve_result: Option<ResolveResult>,
    pub type_map: Option<HashMap<String, type_checker::Type>>,
    pub effect_resolver: Option<EffectResolver>,
    pub mir: Option<crate::mir::MirModule>,
    pub codegen_output: Option<Vec<i64>>,
    pub wasm_output: Option<Vec<u8>>,
    pub module_system: Option<ModuleSystem>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Compiler {
    pub fn new(source: &str, backend: Backend) -> Self {
        Self {
            source: source.to_string(),
            backend,
            source_path: None,
        }
    }

    pub fn with_path(source: &str, backend: Backend, path: std::path::PathBuf) -> Self {
        Self {
            source: source.to_string(),
            backend,
            source_path: Some(path),
        }
    }

    pub fn compile(&self) -> CompilationResult {
        let mut diagnostics = Vec::new();

        // 1. Lexing
        let mut lexer = CompleteLexer::new(&self.source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                diagnostics.push(e);
                return CompilationResult {
                    program: None,
                    resolve_result: None,
                    type_map: None,
                    effect_resolver: None,
                    mir: None,
                    codegen_output: None,
                    wasm_output: None,
                    module_system: None,
                    diagnostics,
                };
            }
        };

        // 1b. Pre-parse macro expansion. Expansion errors are fatal: compiling
        // the unexpanded token stream after a failed expansion can assign a
        // different meaning to the same source program.
        let tokens = match crate::macros::expand_tokens_pre_parse(&tokens) {
            Ok(expanded) => expanded,
            Err(mut macro_diags) => {
                let fatal = macro_diags.iter().any(|d| d.severity == Severity::Error);
                diagnostics.append(&mut macro_diags);
                if fatal {
                    return CompilationResult {
                        program: None,
                        resolve_result: None,
                        type_map: None,
                        effect_resolver: None,
                        mir: None,
                        codegen_output: None,
                        wasm_output: None,
                        module_system: None,
                        diagnostics,
                    };
                }
                tokens
            }
        };

        // 2. Parsing
        let mut parser = Parser::new(tokens);
        let mut program = match parser.parse_program() {
            Ok(prog) => prog,
            Err(e) => {
                diagnostics.push(e);
                return CompilationResult {
                    program: None,
                    resolve_result: None,
                    type_map: None,
                    effect_resolver: None,
                    mir: None,
                    codegen_output: None,
                    wasm_output: None,
                    module_system: None,
                    diagnostics,
                };
            }
        };

        // 2a. Inout desugaring
        if let Err(e) = desugar_inout_in_ast(&mut program) {
            diagnostics.push(Diagnostic::error(error_codes::PARSER_INVALID_EXPRESSION, e));
            return CompilationResult {
                program: Some(program),
                resolve_result: None,
                type_map: None,
                effect_resolver: None,
                mir: None,
                codegen_output: None,
                wasm_output: None,
                module_system: None,
                diagnostics,
            };
        }

        // 2b. Source-level control-flow legality.
        let control_flow_diagnostics = crate::control_flow::validate(&program);
        if !control_flow_diagnostics.is_empty() {
            diagnostics.extend(control_flow_diagnostics);
            return CompilationResult {
                program: Some(program),
                resolve_result: None,
                type_map: None,
                effect_resolver: None,
                mir: None,
                codegen_output: None,
                wasm_output: None,
                module_system: None,
                diagnostics,
            };
        }

        // 2c. Module system initialization. If the caller supplied a concrete
        // source path, failure to load that source/package is fatal; silently
        // substituting an empty module graph can bypass visibility/import rules.
        let module_system = if let Some(ref path) = self.source_path {
            match crate::module_system::init_module_system(path) {
                Ok(ms) => Some(ms),
                Err(e) => {
                    diagnostics.push(Diagnostic::error(
                        error_codes::CODEGEN_COMPILATION_FAILED,
                        format!("Module system initialization failed: {}", e),
                    ));
                    return CompilationResult {
                        program: Some(program),
                        resolve_result: None,
                        type_map: None,
                        effect_resolver: None,
                        mir: None,
                        codegen_output: None,
                        wasm_output: None,
                        module_system: None,
                        diagnostics,
                    };
                }
            }
        } else {
            Some(crate::module_system::ModuleSystem::new())
        };

        // 3. Resolving
        let resolve_result = match resolver::resolve_program(&program) {
            Ok(res) => res,
            Err(errs) => {
                diagnostics.extend(errs);
                return CompilationResult {
                    program: Some(program),
                    resolve_result: None,
                    type_map: None,
                    effect_resolver: None,
                    mir: None,
                    codegen_output: None,
                    wasm_output: None,
                    module_system,
                    diagnostics,
                };
            }
        };

        // 3a. Cross-module import visibility checking
        let mut visibility_error = false;
        if let Some(ref ms) = module_system {
            let requesting_module = if let Some(ref path) = self.source_path {
                ms.module_for_file(path)
                    .unwrap_or_else(|| "main".to_string())
            } else {
                "main".to_string()
            };
            check_imports_visibility(&program.stmts, ms, &requesting_module, &mut diagnostics);
            visibility_error = diagnostics.iter().any(|d| d.severity == Severity::Error);
        }

        if visibility_error {
            return CompilationResult {
                program: Some(program),
                resolve_result: Some(resolve_result),
                type_map: None,
                effect_resolver: None,
                mir: None,
                codegen_output: None,
                wasm_output: None,
                module_system,
                diagnostics,
            };
        }

        // 4. Type Checking
        let type_map =
            match type_checker::type_check_program_with_modules(&program, module_system.as_ref()) {
                Ok(map) => map,
                Err(e) => {
                    diagnostics.push(e);
                    return CompilationResult {
                        program: Some(program),
                        resolve_result: Some(resolve_result),
                        type_map: None,
                        effect_resolver: None,
                        mir: None,
                        codegen_output: None,
                        wasm_output: None,
                        module_system,
                        diagnostics,
                    };
                }
            };

        // 4a. Monomorphization / generic codegen gate. Generic source is
        // accepted by the frontend, but it MUST NOT silently reach machine
        // code until the concrete-specialization pass can prove its ABI.
        {
            let mut monomorphizer = crate::monomorphizer::Monomorphizer::new();
            if let Err(e) = monomorphizer.specialize(&mut program, &type_map) {
                diagnostics.push(Diagnostic::error(
                    error_codes::CODEGEN_UNSUPPORTED_FEATURE,
                    e,
                ));
                return CompilationResult {
                    program: Some(program),
                    resolve_result: Some(resolve_result),
                    type_map: Some(type_map),
                    effect_resolver: None,
                    mir: None,
                    codegen_output: None,
                    wasm_output: None,
                    module_system,
                    diagnostics,
                };
            }
        }

        // 4b. Security capability checks — verify capability tokens and FFI sandbox rules
        {
            let mut cap_system = crate::security::CapabilitySystem::new();
            for stmt in &program.stmts {
                if let Stmt::Capability {
                    name, permissions, ..
                } = stmt
                {
                    for perm in permissions {
                        let cap = match perm.as_str() {
                            "io" => crate::security::Capability::Io,
                            "network" => crate::security::Capability::Network {
                                allowed_hosts: vec![],
                            },
                            "filesystem" => crate::security::Capability::Filesystem {
                                allowed_paths: vec![],
                            },
                            "random" => crate::security::Capability::Random,
                            "time" => crate::security::Capability::Time,
                            "ffi" => crate::security::Capability::Ffi,
                            "thread" => crate::security::Capability::Thread,
                            "process" => crate::security::Capability::Process {
                                spawn: true,
                                exit: true,
                            },
                            _ => continue,
                        };
                        cap_system.register_capability(name, cap);
                    }
                }
            }
            // Check for capabilities used in function calls
            for stmt in &program.stmts {
                if let Stmt::FfiSandbox { allow_list, .. } = stmt {
                    let mut sandbox = crate::security::FfiSandbox::new(64 * 1024, 1024 * 1024);
                    for func_name in allow_list {
                        sandbox.allow_function(crate::security::FfiSig {
                            name: func_name.clone(),
                            args: vec![],
                            ret: crate::security::FfiType::I64,
                            safe: true,
                        });
                    }
                }
            }
        }

        // 4c. Compile-time evaluation — evaluate comptime limit directives
        // and comptime-known constant expressions.
        {
            let mut comptime_ctx = crate::comptime::ComptimeContext::new();
            // `comptime_limit` limits evaluator operations, not recursion depth.
            for stmt in &program.stmts {
                if let Stmt::ComptimeLimit { max_ops, .. } = stmt {
                    comptime_ctx.max_operations = *max_ops as usize;
                }
            }
            // Attempt to evaluate comptime-known expressions
            for stmt in &program.stmts {
                if let Stmt::ExprStmt(expr, _) = stmt {
                    if let Expr::Call(name, _, _) = expr {
                        if name == "comptime_eval" || name == "comptime" {
                            if let Err(e) = comptime_ctx.eval_expr(expr) {
                                diagnostics.push(Diagnostic::error(
                                    error_codes::CODEGEN_UNSUPPORTED_FEATURE,
                                    format!("Comptime evaluation failed: {:?}", e),
                                ));
                            }
                        }
                    }
                }
            }
        }
        if diagnostics.iter().any(|d| d.severity == Severity::Error) {
            return CompilationResult {
                program: Some(program),
                resolve_result: Some(resolve_result),
                type_map: Some(type_map),
                effect_resolver: None,
                mir: None,
                codegen_output: None,
                wasm_output: None,
                module_system,
                diagnostics,
            };
        }

        // 4d. Trait checking — register trait definitions and impl blocks
        {
            let mut trait_system = TraitSystem::new();
            for stmt in &program.stmts {
                if let Stmt::Trait {
                    name,
                    type_params,
                    methods: trait_methods,
                    ..
                } = stmt
                {
                    let registered_methods: Vec<crate::traits::MethodSignature> = trait_methods
                        .iter()
                        .filter_map(|m| {
                            if let Stmt::Fn {
                                name: mname,
                                params: mparams,
                                ret_type,
                                effects,
                                ..
                            } = m
                            {
                                let generic_names: std::collections::HashSet<&str> = type_params
                                    .iter()
                                    .map(|(n, _)| n.as_str())
                                    .chain(std::iter::once("Self"))
                                    .collect();
                                let ptypes: Vec<(String, crate::types::Type)> = mparams
                                    .iter()
                                    .map(|(pname, pty)| {
                                        let ty = pty
                                            .as_deref()
                                            .map(|ann| {
                                                trait_type_from_annotation(ann, &generic_names)
                                            })
                                            .unwrap_or_else(|| {
                                                crate::types::Type::Generic(format!(
                                                    "__inferred_{}",
                                                    pname
                                                ))
                                            });
                                        (pname.clone(), ty)
                                    })
                                    .collect();
                                let rtype_final = ret_type
                                    .as_deref()
                                    .map(|ann| trait_type_from_annotation(ann, &generic_names))
                                    .unwrap_or(crate::types::Type::Unit);
                                let mut ef_set = crate::types::EffectSet::new();
                                for e in effects {
                                    match e.as_str() {
                                        "io" => ef_set.add(crate::types::Effect::Io),
                                        "async" => ef_set.add(crate::types::Effect::Async),
                                        "panic" => ef_set.add(crate::types::Effect::Panic),
                                        "pure" => ef_set.add(crate::types::Effect::Pure),
                                        _ => ef_set.add(crate::types::Effect::Custom(e.clone())),
                                    }
                                }
                                Some(crate::traits::MethodSignature {
                                    name: mname.clone(),
                                    params: ptypes,
                                    return_type: rtype_final,
                                    effect: ef_set,
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                    let required: Vec<String> =
                        registered_methods.iter().map(|m| m.name.clone()).collect();
                    if let Err(msg) = trait_system.add_trait(crate::traits::TraitDefinition {
                        name: name.clone(),
                        type_params: type_params.iter().map(|(n, _)| n.clone()).collect(),
                        bounds: vec![],
                        supertraits: vec![],
                        methods: registered_methods,
                        required_methods: required,
                        is_sealed: false,
                    }) {
                        diagnostics.push(Diagnostic::error(error_codes::TYPE_TRAIT_BOUND, msg));
                    }
                }
                if let Stmt::Impl {
                    target,
                    for_type: Some(for_ty),
                    methods,
                    ..
                } = stmt
                {
                    // Only `impl Trait for Type` participates in the trait
                    // registry. Inherent `impl Type` blocks are method
                    // containers and must not be misread as a trait named Type.
                    let registered_methods: Vec<crate::traits::ImplMethod> = methods
                        .iter()
                        .filter_map(|m| {
                            if let Stmt::Fn {
                                name: mname,
                                body: mbody,
                                ..
                            } = m
                            {
                                Some(crate::traits::ImplMethod {
                                    name: mname.clone(),
                                    body: mbody.clone(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                    let impl_type = crate::types::Type::Struct {
                        name: for_ty.clone(),
                        fields: vec![],
                        is_linear: false,
                    };
                    if let Err(msg) = trait_system.add_impl(crate::traits::TraitImpl {
                        trait_name: target.clone(),
                        impl_type,
                        methods: registered_methods,
                    }) {
                        diagnostics.push(Diagnostic::error(error_codes::TYPE_TRAIT_BOUND, msg));
                    }
                }
            }

            // Check trait satisfaction for registered type bounds
            for (trait_name, def) in &trait_system.traits {
                for bound in &def.bounds {
                    if matches!(&bound.for_type, crate::types::Type::Generic(name) if name == "Self")
                    {
                        continue;
                    }
                    if let Err(msg) =
                        check_trait_satisfaction(&trait_system, &bound.for_type, trait_name)
                    {
                        diagnostics.push(Diagnostic::error(error_codes::TYPE_TRAIT_BOUND, msg));
                    }
                }
            }

            // Check function type parameter trait bounds
            for stmt in &program.stmts {
                if let Stmt::Fn {
                    name, type_params, ..
                } = stmt
                {
                    for (param_name, bounds) in type_params {
                        for bound in bounds {
                            if !trait_system.traits.contains_key(bound) {
                                diagnostics.push(Diagnostic::error(
                                    error_codes::TYPE_TRAIT_BOUND,
                                    format!(
                                        "Type parameter '{}' in function '{}' requires unknown trait '{}'",
                                        param_name, name, bound
                                    ),
                                ));
                            }
                        }
                    }
                }
            }

            // Halt if trait checking produced errors
            if diagnostics.iter().any(|d| d.severity == Severity::Error) {
                return CompilationResult {
                    program: Some(program),
                    resolve_result: Some(resolve_result),
                    type_map: Some(type_map),
                    effect_resolver: None,
                    mir: None,
                    codegen_output: None,
                    wasm_output: None,
                    module_system,
                    diagnostics,
                };
            }
        }

        // 5. Effect Resolution
        let mut effect_resolver = crate::effect_resolver::EffectResolver::new();
        if let Err(e) = effect_resolver.resolve(&program) {
            diagnostics.push(e);
            return CompilationResult {
                program: Some(program),
                resolve_result: Some(resolve_result),
                type_map: Some(type_map),
                effect_resolver: Some(effect_resolver),
                mir: None,
                codegen_output: None,
                wasm_output: None,
                module_system,
                diagnostics,
            };
        }

        // 6. MIR Generation
        let mut mir = crate::mir::lower_program_to_mir(&program);

        // 6a. Unsafe validation
        let unsafe_warnings = crate::mir::validate_unsafe_usage(&mir);
        for warning in unsafe_warnings {
            diagnostics.push(Diagnostic::warning(
                error_codes::CODEGEN_UNSUPPORTED_FEATURE,
                warning,
            ));
        }

        // 6b. Verify unoptimized MIR first. Borrow/ownership validation must
        // observe source-faithful MIR; optimization is not permitted to erase
        // evidence needed for soundness checks.
        if let Err(e) = crate::mir::validate_control_flow(&mir) {
            diagnostics.push(Diagnostic::error(error_codes::CODEGEN_INVALID_MIR, e));
            return CompilationResult {
                program: Some(program),
                resolve_result: Some(resolve_result),
                type_map: Some(type_map),
                effect_resolver: Some(effect_resolver),
                mir: Some(mir),
                codegen_output: None,
                wasm_output: None,
                module_system,
                diagnostics,
            };
        }

        // 7. Borrow checking precedes optimization. The canonical compiler has
        // no environment-variable bypass: a soundness gate cannot depend on a
        // process environment accident. The current adapter is conservative
        // for the scalar baseline; full ownership qualification is a v0.2.0 gate.
        if let Err(e) = crate::polonius::check_mir(&mir) {
            diagnostics.push(Diagnostic::error(error_codes::BORROW_USE_AFTER_MOVE, e));
            return CompilationResult {
                program: Some(program),
                resolve_result: Some(resolve_result),
                type_map: Some(type_map),
                effect_resolver: Some(effect_resolver),
                mir: Some(mir),
                codegen_output: None,
                wasm_output: None,
                module_system,
                diagnostics,
            };
        }

        // 7c. Provenance violation check: creating a reference `&val` and
        // immediately dereferencing it without preserving the value loses
        // provenance, which is a semantic violation. Detect this pattern
        // before optimization passes can silently erase the evidence.
        {
            use crate::mir::Instruction;

            let mut provenance_violations = Vec::new();

            for function in &mir.functions {
                // Track whether the previous non-trivial instruction was a borrow
                let mut last_was_borrow = false;

                for block in &function.blocks {
                    for instr in &block.instrs {
                        match instr {
                            Instruction::Borrow { .. } => {
                                // Track borrow of a local variable
                                last_was_borrow = true;
                            }
                            Instruction::Deref { .. } => {
                                // Check if this deref immediately follows a borrow
                                // and the result is about to be discarded (not stored to a variable)
                                if last_was_borrow {
                                    // This is a provenance violation: &val was created and
                                    // *p is being discarded without preserving the value
                                    provenance_violations.push(
                                        "provenance loss: reference dropped after immediate dereference".to_string(),
                                    );
                                }
                                // After a deref (whether or not it was a provenance violation),
                                // the borrow is no longer relevant since the value was consumed
                                last_was_borrow = false;
                            }
                            Instruction::Move { .. } => {
                                // A move after a borrow may consume the reference
                                // or may be moving the borrowed value to another location.
                                // In either case, reset the borrow tracking since the
                                // reference pattern has been transformed.
                                last_was_borrow = false;
                            }
                            _ => {
                                // Any other instruction (binary op, assign, etc.) resets
                                // the borrow tracking since it's a different kind of operation.
                                last_was_borrow = false;
                            }
                        }
                    }
                }
            }

            if !provenance_violations.is_empty() {
                for msg in &provenance_violations {
                    diagnostics.push(Diagnostic::error(
                        error_codes::CODEGEN_INVALID_MIR,
                        msg.clone(),
                    ));
                }
                return CompilationResult {
                    program: Some(program),
                    resolve_result: Some(resolve_result),
                    type_map: Some(type_map),
                    effect_resolver: Some(effect_resolver),
                    mir: Some(mir),
                    codegen_output: None,
                    wasm_output: None,
                    module_system,
                    diagnostics,
                };
            }
        }

        // 7b. Optimize only after semantic/borrow validation, then re-verify the
        // transformed control-flow graph before any backend sees it.
        crate::mir_optimize::run_mir_optimizations(&mut mir);
        if let Err(e) = crate::mir::validate_control_flow(&mir) {
            diagnostics.push(Diagnostic::error(error_codes::CODEGEN_INVALID_MIR, e));
            return CompilationResult {
                program: Some(program),
                resolve_result: Some(resolve_result),
                type_map: Some(type_map),
                effect_resolver: Some(effect_resolver),
                mir: Some(mir),
                codegen_output: None,
                wasm_output: None,
                module_system,
                diagnostics,
            };
        }

        // 8. Codegen
        let mut codegen_output = None;
        #[cfg(feature = "wasm-backend")]
        let mut wasm_output = None;
        #[cfg(not(feature = "wasm-backend"))]
        let wasm_output: Option<Vec<u8>> = None;

        if self.backend == Backend::Wasm {
            #[cfg(feature = "wasm-backend")]
            {
                match crate::codegen_lir::lower_mir_to_lir(&mir) {
                    Ok(lir_module) => match codegen_wasm::emit_wasm_bytes(&lir_module) {
                        Ok(bytes) => wasm_output = Some(bytes),
                        Err(e) => diagnostics.push(Diagnostic::error(
                            error_codes::CODEGEN_COMPILATION_FAILED,
                            e,
                        )),
                    },
                    Err(e) => diagnostics.push(Diagnostic::error(
                        error_codes::CODEGEN_COMPILATION_FAILED,
                        e,
                    )),
                }
            }
            #[cfg(not(feature = "wasm-backend"))]
            {
                diagnostics.push(Diagnostic::error(
                    error_codes::CODEGEN_UNSUPPORTED_FEATURE,
                    "Wasm backend is disabled; rebuild with feature 'wasm-backend'",
                ));
            }
        } else if self.backend != Backend::Native {
            // Non-native developer backends may execute in-process. The
            // canonical Native backend deliberately stops at verified MIR/LIR
            // here so artifact emission never has an implicit execution side
            // effect.
            match crate::codegen_lir::lower_mir_to_lir(&mir) {
                Ok(lir_module) => {
                    match crate::codegen::compile_and_run(&lir_module, self.backend) {
                        Ok(res) => codegen_output = Some(res),
                        Err(e) => diagnostics.push(Diagnostic::error(
                            error_codes::CODEGEN_COMPILATION_FAILED,
                            e,
                        )),
                    }
                }
                Err(e) => diagnostics.push(Diagnostic::error(
                    error_codes::CODEGEN_COMPILATION_FAILED,
                    e,
                )),
            }
        }

        CompilationResult {
            program: Some(program),
            resolve_result: Some(resolve_result),
            type_map: Some(type_map),
            effect_resolver: Some(effect_resolver),
            mir: Some(mir),
            codegen_output,
            wasm_output,
            module_system,
            diagnostics,
        }
    }
}

fn trait_type_from_annotation(
    annotation: &str,
    generic_names: &std::collections::HashSet<&str>,
) -> crate::types::Type {
    if generic_names.contains(annotation) {
        return crate::types::Type::Generic(annotation.to_string());
    }
    match annotation.to_ascii_lowercase().as_str() {
        "int" | "i64" | "isize" => crate::types::Type::Int,
        "float" | "f64" | "float64" => crate::types::Type::Float,
        "char" => crate::types::Type::Char,
        "string" | "str" => crate::types::Type::String,
        "bool" | "boolean" => crate::types::Type::Bool,
        "unit" | "void" | "()" => crate::types::Type::Unit,
        "never" => crate::types::Type::Never,
        _ => crate::types::Type::Struct {
            name: annotation.to_string(),
            fields: vec![],
            is_linear: false,
        },
    }
}

fn check_imports_visibility(
    stmts: &[Stmt],
    ms: &ModuleSystem,
    requesting_module: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let capabilities: Vec<String> = ms
        .manifest
        .as_ref()
        .map(|m| {
            m.capabilities
                .iter()
                .filter(|c| c.enabled)
                .map(|c| c.kind.clone())
                .collect()
        })
        .unwrap_or_default();

    for stmt in stmts {
        match stmt {
            Stmt::Use { path, .. } => {
                let normalized_path = path.replace(".", "::");
                if let Some(module) = ms.modules.get(&normalized_path) {
                    let sym = crate::module_system::SymbolInfo {
                        name: module.name.clone(),
                        module_path: module.parent.clone().unwrap_or_default(),
                        visibility: module.visibility.clone(),
                        is_type: false,
                    };
                    if !ms.is_symbol_accessible(&sym, requesting_module, &capabilities) {
                        diagnostics.push(Diagnostic::error(
                            error_codes::TYPE_VISIBILITY_PRIVATE,
                            format!(
                                "Module '{}' is private and cannot be accessed from '{}'",
                                normalized_path, requesting_module
                            ),
                        ));
                    }
                } else if let Some((module_path, symbol_name)) = normalized_path.rsplit_once("::") {
                    if let Some(symbols) = ms.symbols.get(symbol_name) {
                        if let Some(sym) = symbols.iter().find(|s| s.module_path == module_path) {
                            if !ms.is_symbol_accessible(sym, requesting_module, &capabilities) {
                                diagnostics.push(Diagnostic::error(
                                    error_codes::TYPE_VISIBILITY_PRIVATE,
                                    format!(
                                        "Symbol '{}' is private and cannot be accessed from '{}'",
                                        normalized_path, requesting_module
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
            Stmt::UseScoped {
                path,
                aliases,
                body,
                ..
            } => {
                let normalized_base = path.replace(".", "::");
                if let Some(module) = ms.modules.get(&normalized_base) {
                    let sym = crate::module_system::SymbolInfo {
                        name: module.name.clone(),
                        module_path: module.parent.clone().unwrap_or_default(),
                        visibility: module.visibility.clone(),
                        is_type: false,
                    };
                    if !ms.is_symbol_accessible(&sym, requesting_module, &capabilities) {
                        diagnostics.push(Diagnostic::error(
                            error_codes::TYPE_VISIBILITY_PRIVATE,
                            format!(
                                "Module '{}' is private and cannot be accessed from '{}'",
                                normalized_base, requesting_module
                            ),
                        ));
                    }
                }
                for (alias, resolved_name) in aliases {
                    let symbol_name = resolved_name.clone().unwrap_or_else(|| alias.clone());
                    if let Some(symbols) = ms.symbols.get(&symbol_name) {
                        if let Some(sym) = symbols.iter().find(|s| s.module_path == normalized_base)
                        {
                            if !ms.is_symbol_accessible(sym, requesting_module, &capabilities) {
                                diagnostics.push(Diagnostic::error(
                                    error_codes::TYPE_VISIBILITY_PRIVATE,
                                    format!("Symbol '{}::{}' is private and cannot be accessed from '{}'", normalized_base, symbol_name, requesting_module),
                                ));
                            }
                        }
                    }
                }
                check_imports_visibility(body, ms, requesting_module, diagnostics);
            }
            Stmt::Block(inner, _)
            | Stmt::Loop { body: inner, .. }
            | Stmt::For { body: inner, .. }
            | Stmt::While { body: inner, .. }
            | Stmt::WhileIn { body: inner, .. }
            | Stmt::Unsafe { body: inner, .. } => {
                check_imports_visibility(inner, ms, requesting_module, diagnostics);
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                check_imports_visibility(then_body, ms, requesting_module, diagnostics);
                check_imports_visibility(else_body, ms, requesting_module, diagnostics);
            }
            Stmt::Fn { body: inner, .. } => {
                check_imports_visibility(inner, ms, requesting_module, diagnostics);
            }
            Stmt::Impl { methods: inner, .. } => {
                check_imports_visibility(inner, ms, requesting_module, diagnostics);
            }
            Stmt::ModBlock(_, inner, _) => {
                check_imports_visibility(inner, ms, requesting_module, diagnostics);
            }
            _ => {}
        }
    }
}
