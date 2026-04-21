use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::ast::{Expr, Program, Stmt};
use crate::diagnostics::{Diagnostic, Span};
use crate::lexer::{Lexer, Token, TokenKind};
use crate::mir;
use crate::parser::Parser;

fn char_pos_to_byte_index(text: &str, target_line: usize, target_col: usize) -> Option<usize> {
    let mut line: usize = 1;
    let mut col: usize = 0;
    for (byte_idx, ch) in text.char_indices() {
        if line == target_line && col == target_col {
            return Some(byte_idx);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    if line == target_line && col == target_col {
        return Some(text.len());
    }
    None
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub span: Span,
    pub text: String,
    pub contents: Vec<HoverContent>,
}

#[derive(Debug, Clone)]
pub enum HoverContent {
    Type(String),
    Effect(u8),
    Definition(String),
    Documentation(String),
}

#[derive(Debug)]
pub struct CompilationDatabase {
    sources: HashMap<String, Arc<SourceFile>>,
    analysis: HashMap<String, Arc<FileAnalysis>>,
    // Workspace-wide index: symbol name -> list of (path, SymbolInfo)
    workspace_index: HashMap<String, Vec<(String, SymbolInfo)>>,
}

#[derive(Debug)]
pub struct SourceFile {
    pub path: String,
    pub text: String,
    pub version: usize,
}

#[derive(Debug)]
pub struct FileAnalysis {
    pub path: String,
    pub ast: Program,
    pub symbols: HashMap<String, SymbolInfo>,
    pub types: HashMap<SymbolId, TypeInfo>,
    pub diagnostics: Vec<Diagnostic>,
    pub mir: Option<mir::MirModule>,
    pub version: usize,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SymbolId {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub definition_span: Span,
    pub resolved_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Parameter,
    Struct,
    Field,
    Enum,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub span: Span,
    pub type_string: String,
    pub effect: u8,
}

impl CompilationDatabase {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            analysis: HashMap::new(),
            workspace_index: HashMap::new(),
        }
    }

    pub fn add_source(&mut self, path: String, text: String, version: usize) {
        let source = Arc::new(SourceFile {
            path: path.clone(),
            text: text.clone(),
            version,
        });
        self.sources.insert(path.clone(), source);

        // If we already had analysis for this path, clear its entries first
        self.clear_workspace_entries_for_path(&path);

        let analysis = self.analyze_file(&path, &text, version);
        self.analysis.insert(path.clone(), Arc::new(analysis));

        // update workspace index for quick cross-file lookups
        if let Some(analysis) = self.analysis.get(&path) {
            for (name, sym) in &analysis.symbols {
                self.workspace_index
                    .entry(name.clone())
                    .or_default()
                    .push((path.clone(), sym.clone()));
            }
        }
    }

    fn clear_workspace_entries_for_path(&mut self, path: &str) {
        let mut to_remove: Vec<String> = Vec::new();
        for (name, vec) in self.workspace_index.iter_mut() {
            vec.retain(|(p, _)| p != path);
            if vec.is_empty() {
                to_remove.push(name.clone());
            }
        }
        for k in to_remove {
            self.workspace_index.remove(&k);
        }
    }

    pub fn remove_source(&mut self, path: &str) {
        self.sources.remove(path);
        self.analysis.remove(path);
        self.clear_workspace_entries_for_path(path);
    }

    pub fn rename_source_file(&mut self, old: &str, new: String) {
        if let Some(src) = self.sources.remove(old) {
            let s_text = src.text.clone();
            let s_version = src.version;
            let new_src = SourceFile {
                path: new.clone(),
                text: s_text.clone(),
                version: s_version,
            };
            self.sources.insert(new.clone(), Arc::new(new_src));

            // Re-run analysis for the file under the new path
            let analysis = self.analyze_file(&new, &s_text, s_version);
            self.analysis.insert(new.clone(), Arc::new(analysis));
        } else {
            // If there was no source record, just move any existing analysis entry
            if let Some(analysis_arc) = self.analysis.remove(old) {
                self.analysis.insert(new.clone(), analysis_arc);
            }
        }
        // Update workspace index entries referencing the old path
        for (_name, entries) in self.workspace_index.iter_mut() {
            for (p, _s) in entries.iter_mut() {
                if p == old {
                    *p = new.clone();
                }
            }
        }
    }

    pub fn get_source_text(&self, path: &str) -> Option<String> {
        self.sources.get(path).map(|s| s.text.clone())
    }

    /// Rename all occurrences of `old_name` to `new_name` across all
    /// indexed workspace files. This updates the in-memory sources,
    /// re-runs analysis for modified files, and updates the workspace
    /// index accordingly.
    pub fn rename_symbol_across_workspace(
        &mut self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), String> {
        let paths: Vec<String> = self.sources.keys().cloned().collect();
        for path in paths {
            let src_arc = match self.sources.get(&path) {
                Some(s) => Arc::clone(s),
                None => continue,
            };
            let src_text = src_arc.text.clone();
            let mut lexer = Lexer::new(&src_text);
            let tokens = match lexer.tokenize() {
                Ok(t) => t,
                Err(_) => continue,
            };

            let mut replacements: Vec<(usize, usize)> = Vec::new();
            for token in tokens.iter() {
                if token.kind == TokenKind::Ident && token.text == old_name {
                    let start_col = token.col.saturating_sub(1);
                    let start_byte = char_pos_to_byte_index(&src_text, token.line, start_col)
                        .ok_or_else(|| "failed to compute start byte".to_string())?;
                    let end_byte = char_pos_to_byte_index(
                        &src_text,
                        token.line,
                        start_col + token.text.chars().count(),
                    )
                    .unwrap_or(src_text.len());
                    replacements.push((start_byte, end_byte));
                }
            }

            if replacements.is_empty() {
                continue;
            }

            // Apply replacements from end -> start to keep byte indices valid
            replacements.sort_by_key(|replacement| std::cmp::Reverse(replacement.0));
            let mut new_text = src_text.clone();
            for (start, end) in replacements {
                new_text.replace_range(start..end, new_name);
            }

            // update source
            let new_version = src_arc.version + 1;
            let new_src = SourceFile {
                path: path.clone(),
                text: new_text.clone(),
                version: new_version,
            };
            self.sources.insert(path.clone(), Arc::new(new_src));

            // re-analyze and update analysis and workspace index for this file
            self.clear_workspace_entries_for_path(&path);
            let analysis = self.analyze_file(&path, &new_text, new_version);
            self.analysis.insert(path.clone(), Arc::new(analysis));
            if let Some(analysis) = self.analysis.get(&path) {
                for (name, sym) in &analysis.symbols {
                    self.workspace_index
                        .entry(name.clone())
                        .or_default()
                        .push((path.clone(), sym.clone()));
                }
            }
        }

        Ok(())
    }

    /// Recursively index `.omni` files under `root`. This will read each
    /// file and add it to the compilation database so cross-file lookups
    /// and hovers can resolve symbols from workspace files.
    pub fn index_workspace_dir(&mut self, root: &str) {
        let rootp = Path::new(root);
        fn visit(db: &mut CompilationDatabase, path: &Path) {
            if path.is_dir() {
                if let Ok(entries) = fs::read_dir(path) {
                    for e in entries.flatten() {
                        let p = e.path();
                        visit(db, &p);
                    }
                }
            } else if let Some(ext) = path.extension() {
                if ext == "omni" {
                    if let Ok(text) = fs::read_to_string(path) {
                        if let Some(pstr) = path.to_str() {
                            db.add_source(pstr.to_string(), text, 1);
                        }
                    }
                }
            }
        }

        visit(self, rootp);
    }

    fn token_at_position(&self, path: &str, line: usize, col: usize) -> Option<Token> {
        let source = self.sources.get(path)?;
        let mut lexer = Lexer::new(&source.text);
        let tokens = lexer.tokenize().ok()?;

        tokens.into_iter().find(|token| {
            token.kind == TokenKind::Ident && token.line == line && {
                let start_col = token.col.saturating_sub(1);
                let end_col = start_col + token.text.chars().count();
                col >= start_col && col < end_col
            }
        })
    }

    fn find_name_span(text: &str, name: &str) -> Span {
        let mut lexer = Lexer::new(text);
        if let Ok(tokens) = lexer.tokenize() {
            for token in tokens {
                if token.kind == TokenKind::Ident && token.text == name {
                    let start_col = token.col.saturating_sub(1);
                    let end_col = start_col + token.text.chars().count().saturating_sub(1);
                    return Span::new(token.line, start_col, token.line, end_col);
                }
            }
        }

        Span::new(1, 0, 1, name.len())
    }

    fn symbol_by_name(
        &self,
        name: &str,
        prefer_path: Option<&str>,
    ) -> Option<(String, SymbolInfo)> {
        if let Some(path) = prefer_path {
            if let Some(analysis) = self.analysis.get(path) {
                if let Some(symbol) = analysis.symbols.get(name) {
                    return Some((path.to_string(), symbol.clone()));
                }
            }
        }

        // Fast path: workspace index populated by `add_source` / `index_workspace_dir`.
        if let Some(entries) = self.workspace_index.get(name) {
            if let Some((p, s)) = entries.first() {
                return Some((p.clone(), s.clone()));
            }
        }

        // Fallback: scan currently-analyzed files
        for (path, analysis) in &self.analysis {
            if let Some(symbol) = analysis.symbols.get(name) {
                return Some((path.clone(), symbol.clone()));
            }
        }

        None
    }

    fn type_by_name(&self, name: &str, prefer_path: Option<&str>) -> Option<TypeInfo> {
        if let Some(path) = prefer_path {
            if let Some(analysis) = self.analysis.get(path) {
                if let Some(type_info) = analysis
                    .types
                    .iter()
                    .find_map(|(id, type_info)| (id.name == name).then(|| type_info.clone()))
                {
                    return Some(type_info);
                }
            }
        }

        // Try workspace index entries for this symbol name to find a path
        if let Some(entries) = self.workspace_index.get(name) {
            for (p, _s) in entries {
                if let Some(analysis) = self.analysis.get(p) {
                    if let Some(type_info) = analysis
                        .types
                        .iter()
                        .find_map(|(id, type_info)| (id.name == name).then(|| type_info.clone()))
                    {
                        return Some(type_info);
                    }
                }
            }
        }

        for analysis in self.analysis.values() {
            if let Some(type_info) = analysis
                .types
                .iter()
                .find_map(|(id, type_info)| (id.name == name).then(|| type_info.clone()))
            {
                return Some(type_info);
            }
        }

        None
    }

    fn analyze_file(&self, path: &str, text: &str, version: usize) -> FileAnalysis {
        let mut lexer = Lexer::new(text);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                return FileAnalysis {
                    path: path.to_string(),
                    ast: Program::new(),
                    symbols: HashMap::new(),
                    types: HashMap::new(),
                    diagnostics: vec![Diagnostic::error(
                        crate::diagnostics::error_codes::PARSER_UNEXPECTED_TOKEN,
                        e,
                    )],
                    mir: None,
                    version,
                };
            }
        };

        let mut parser = Parser::new(tokens);
        let ast = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                return FileAnalysis {
                    path: path.to_string(),
                    ast: Program::new(),
                    symbols: HashMap::new(),
                    types: HashMap::new(),
                    diagnostics: vec![Diagnostic::error(
                        crate::diagnostics::error_codes::PARSER_UNEXPECTED_TOKEN,
                        e,
                    )],
                    mir: None,
                    version,
                };
            }
        };

        let symbols = self.extract_symbols(&ast, text);
        let types = self.extract_types(&ast, text);
        let mir_module = mir::lower_program_to_mir(&ast);

        FileAnalysis {
            path: path.to_string(),
            ast,
            symbols,
            types,
            diagnostics: vec![],
            mir: Some(mir_module),
            version,
        }
    }

    fn extract_symbols(&self, ast: &Program, text: &str) -> HashMap<String, SymbolInfo> {
        let mut symbols = HashMap::new();

        for stmt in &ast.stmts {
            match stmt {
                Stmt::Fn { name, params, .. } => {
                    let span = Self::find_name_span(text, name);
                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            id: SymbolId {
                                name: name.clone(),
                                span: span.clone(),
                            },
                            kind: SymbolKind::Function,
                            definition_span: span,
                            resolved_name: Some(name.clone()),
                        },
                    );

                    for param in params {
                        let span = Self::find_name_span(text, param);
                        symbols.insert(
                            param.clone(),
                            SymbolInfo {
                                id: SymbolId {
                                    name: param.clone(),
                                    span: span.clone(),
                                },
                                kind: SymbolKind::Parameter,
                                definition_span: span,
                                resolved_name: Some(name.clone()),
                            },
                        );
                    }
                }
                Stmt::Let(name, _) => {
                    let span = Self::find_name_span(text, name);
                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            id: SymbolId {
                                name: name.clone(),
                                span: span.clone(),
                            },
                            kind: SymbolKind::Variable,
                            definition_span: span,
                            resolved_name: None,
                        },
                    );
                }
                Stmt::Struct {
                    name,
                    fields,
                    is_linear: _,
                } => {
                    let span = Self::find_name_span(text, name);
                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            id: SymbolId {
                                name: name.clone(),
                                span: span.clone(),
                            },
                            kind: SymbolKind::Struct,
                            definition_span: span,
                            resolved_name: Some(name.clone()),
                        },
                    );

                    for (field_name, _) in fields {
                        let span = Self::find_name_span(text, field_name);
                        symbols.insert(
                            field_name.clone(),
                            SymbolInfo {
                                id: SymbolId {
                                    name: field_name.clone(),
                                    span: span.clone(),
                                },
                                kind: SymbolKind::Field,
                                definition_span: span,
                                resolved_name: Some(name.clone()),
                            },
                        );
                    }
                }
                Stmt::Enum { name, variants, .. } => {
                    let span = Self::find_name_span(text, name);
                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            id: SymbolId {
                                name: name.clone(),
                                span: span.clone(),
                            },
                            kind: SymbolKind::Enum,
                            definition_span: span,
                            resolved_name: Some(name.clone()),
                        },
                    );

                    for variant in variants {
                        let span = Self::find_name_span(text, &variant.name);
                        symbols.insert(
                            variant.name.clone(),
                            SymbolInfo {
                                id: SymbolId {
                                    name: variant.name.clone(),
                                    span: span.clone(),
                                },
                                kind: SymbolKind::Enum,
                                definition_span: span,
                                resolved_name: Some(name.clone()),
                            },
                        );
                    }
                }
                _ => {}
            }
        }

        symbols
    }

    fn extract_types(&self, ast: &Program, text: &str) -> HashMap<SymbolId, TypeInfo> {
        let mut types = HashMap::new();

        // Simplified type extraction - for a full implementation,
        // this would run through the type checker
        for stmt in &ast.stmts {
            match stmt {
                Stmt::Fn { name, ret_type, .. } => {
                    let type_str = match ret_type {
                        Some(t) => format!("fn({})", t),
                        None => "fn()".to_string(),
                    };
                    let span = Self::find_name_span(text, name);
                    types.insert(
                        SymbolId {
                            name: name.clone(),
                            span: span.clone(),
                        },
                        TypeInfo {
                            span,
                            type_string: type_str,
                            effect: 0,
                        },
                    );
                }
                Stmt::Let(name, expr) => {
                    let type_str = match expr {
                        Expr::Number(_) => "int".to_string(),
                        Expr::StringLit(_) => "string".to_string(),
                        Expr::Bool(_) => "bool".to_string(),
                        Expr::Call(_, _) => "fn()".to_string(),
                        _ => "unknown".to_string(),
                    };
                    let span = Self::find_name_span(text, name);
                    types.insert(
                        SymbolId {
                            name: name.clone(),
                            span: span.clone(),
                        },
                        TypeInfo {
                            span,
                            type_string: type_str,
                            effect: 0,
                        },
                    );
                }
                _ => {}
            }
        }

        types
    }

    pub fn get_symbol_at(&self, path: &str, line: usize, col: usize) -> Option<SymbolInfo> {
        if let Some(analysis) = self.analysis.get(path) {
            for symbol in analysis.symbols.values() {
                let span = &symbol.definition_span;
                if span.start_line == line && col >= span.start_col && col <= span.end_col {
                    return Some(symbol.clone());
                }
            }
        }

        let token = self.token_at_position(path, line, col)?;
        self.symbol_by_name(&token.text, Some(path))
            .map(|(_, symbol)| symbol)
    }

    pub fn get_type_at(&self, path: &str, line: usize, col: usize) -> Option<TypeInfo> {
        if let Some(analysis) = self.analysis.get(path) {
            for type_info in analysis.types.values() {
                let span = &type_info.span;
                if span.start_line == line && col >= span.start_col && col <= span.end_col {
                    return Some(type_info.clone());
                }
            }
        }

        let token = self.token_at_position(path, line, col)?;
        self.type_by_name(&token.text, Some(path))
    }

    pub fn get_definition_location(
        &self,
        path: &str,
        line: usize,
        col: usize,
    ) -> Option<(String, Span)> {
        if let Some(analysis) = self.analysis.get(path) {
            for symbol in analysis.symbols.values() {
                let span = &symbol.definition_span;
                if span.start_line == line && col >= span.start_col && col <= span.end_col {
                    return Some((path.to_string(), span.clone()));
                }
            }
        }

        let token = self.token_at_position(path, line, col)?;
        self.symbol_by_name(&token.text, Some(path))
            .map(|(def_path, symbol)| (def_path, symbol.definition_span))
    }

    pub fn get_diagnostics(&self, path: &str) -> Vec<Diagnostic> {
        self.analysis
            .get(path)
            .map(|a| a.diagnostics.clone())
            .unwrap_or_default()
    }

    pub fn hover_at(&self, path: &str, line: usize, col: usize) -> Option<QueryResult> {
        hover_at(self, path, line, col)
    }

    pub fn goto_definition(&self, path: &str, line: usize, col: usize) -> Option<(String, Span)> {
        goto_definition(self, path, line, col)
    }

    pub fn get_inlay_hints(&self, path: &str) -> Vec<InlayHint> {
        get_inlay_hints(self, path)
    }

    pub fn get_borrow_visualization(&self, path: &str) -> BorrowVisualization {
        get_borrow_visualization(self, path)
    }

    pub fn get_completions(&self, path: &str, line: usize, col: usize) -> Vec<CompletionItem> {
        get_completions(self, path, line, col)
    }
}

impl Default for CompilationDatabase {
    fn default() -> Self {
        Self::new()
    }
}

pub fn hover_at(
    db: &CompilationDatabase,
    path: &str,
    line: usize,
    col: usize,
) -> Option<QueryResult> {
    // Prefer type hover on references, then fall back to symbol metadata.
    if let Some(type_info) = db.get_type_at(path, line, col) {
        return Some(QueryResult {
            span: type_info.span.clone(),
            text: type_info.type_string.clone(),
            contents: vec![
                HoverContent::Type(type_info.type_string.clone()),
                HoverContent::Effect(type_info.effect),
            ],
        });
    }

    if let Some(symbol) = db.get_symbol_at(path, line, col) {
        return Some(QueryResult {
            span: symbol.definition_span.clone(),
            text: format!("{:?}", symbol.kind),
            contents: vec![HoverContent::Definition(symbol.id.name.clone())],
        });
    }

    None
}

pub fn goto_definition(
    db: &CompilationDatabase,
    path: &str,
    line: usize,
    col: usize,
) -> Option<(String, Span)> {
    db.get_definition_location(path, line, col)
}

pub fn get_inlay_hints(db: &CompilationDatabase, path: &str) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    let analysis = match db.analysis.get(path) {
        Some(a) => a,
        None => return hints,
    };

    // Add type hints for let bindings
    for (id, type_info) in &analysis.types {
        if matches!(analysis.symbols.get(&id.name), Some(s) if s.kind == SymbolKind::Variable) {
            hints.push(InlayHint {
                span: Span::new(
                    type_info.span.start_line,
                    type_info.span.end_col,
                    type_info.span.start_line,
                    type_info.span.end_col + 2,
                ),
                text: format!(": {}", type_info.type_string),
                kind: InlayHintKind::Type,
            });
        }
    }

    // Add effect hints for functions that have effects
    for symbol in analysis.symbols.values() {
        if symbol.kind == SymbolKind::Function {
            let type_info = analysis.types.get(&symbol.id);
            if let Some(ti) = type_info {
                if ti.effect != 0 {
                    hints.push(InlayHint {
                        span: Span::new(ti.span.start_line, 0, ti.span.start_line, 1),
                        text: format!("[{}]", effect_to_string(ti.effect)),
                        kind: InlayHintKind::Effect,
                    });
                }
            }
        }
    }

    hints
}

#[derive(Debug, Clone)]
pub struct InlayHint {
    pub span: Span,
    pub text: String,
    pub kind: InlayHintKind,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: CompletionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Function,
    Variable,
    Type,
    Field,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    Type,
    Effect,
    Parameter,
}

fn effect_to_string(effect: u8) -> String {
    let mut parts = Vec::new();
    if effect & 0b0001 != 0 {
        parts.push("io");
    }
    if effect & 0b0010 != 0 {
        parts.push("pure");
    }
    if effect & 0b0100 != 0 {
        parts.push("async");
    }
    if effect & 0b1000 != 0 {
        parts.push("panic");
    }

    if parts.is_empty() {
        return "pure".to_string();
    }
    parts.join("+")
}

fn completion_prefix_at(text: &str, line_1_based: usize, col_0_based: usize) -> String {
    let line = match text.lines().nth(line_1_based.saturating_sub(1)) {
        Some(l) => l,
        None => return String::new(),
    };

    let upto: String = line.chars().take(col_0_based).collect();
    let chars: Vec<char> = upto.chars().collect();
    let mut start = chars.len();
    while start > 0 {
        let c = chars[start - 1];
        if c.is_ascii_alphanumeric() || c == '_' {
            start -= 1;
        } else {
            break;
        }
    }
    chars[start..].iter().collect()
}

fn symbol_kind_to_completion_kind(kind: SymbolKind) -> CompletionKind {
    match kind {
        SymbolKind::Function => CompletionKind::Function,
        SymbolKind::Variable | SymbolKind::Parameter => CompletionKind::Variable,
        SymbolKind::Struct | SymbolKind::Enum => CompletionKind::Type,
        SymbolKind::Field => CompletionKind::Field,
    }
}

pub fn get_completions(
    db: &CompilationDatabase,
    path: &str,
    line: usize,
    col: usize,
) -> Vec<CompletionItem> {
    let source = match db.sources.get(path) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let prefix = completion_prefix_at(&source.text, line, col);

    let keywords = [
        "let", "fn", "if", "else", "match", "while", "for", "loop", "return", "break", "continue",
        "struct", "enum", "trait", "impl", "pub", "use", "mod", "unsafe", "linear", "async",
        "await", "comptime", "true", "false", "print",
    ];

    let mut items: Vec<CompletionItem> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut push_item = |label: String, detail: Option<String>, kind: CompletionKind| {
        if seen.insert(label.clone()) {
            items.push(CompletionItem {
                label,
                detail,
                kind,
            });
        }
    };

    for kw in keywords {
        if prefix.is_empty() || kw.starts_with(&prefix) {
            push_item(
                kw.to_string(),
                Some("keyword".to_string()),
                CompletionKind::Keyword,
            );
        }
    }

    if let Some(analysis) = db.analysis.get(path) {
        for (name, sym) in &analysis.symbols {
            if prefix.is_empty() || name.starts_with(&prefix) {
                push_item(
                    name.clone(),
                    Some(format!("{:?}", sym.kind)),
                    symbol_kind_to_completion_kind(sym.kind),
                );
            }
        }
    }

    for (name, entries) in &db.workspace_index {
        if prefix.is_empty() || name.starts_with(&prefix) {
            if let Some((_p, sym)) = entries.first() {
                push_item(
                    name.clone(),
                    Some(format!("{:?}", sym.kind)),
                    symbol_kind_to_completion_kind(sym.kind),
                );
            } else {
                push_item(name.clone(), None, CompletionKind::Variable);
            }
        }
    }

    items.sort_by(|a, b| a.label.cmp(&b.label));
    items
}

#[derive(Debug, Clone)]
pub struct BorrowInfo {
    pub variable: String,
    pub borrow_span: Span,
    pub kind: BorrowKind,
    pub lifetime_start: usize,
    pub lifetime_end: usize,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    Shared,
    Mutable,
    Move,
}

#[derive(Debug, Clone)]
pub struct MoveInfo {
    pub variable: String,
    pub source_span: Span,
    pub dest_span: Span,
    pub is_valid: bool,
}

#[derive(Debug, Clone)]
pub struct BorrowVisualization {
    pub borrows: Vec<BorrowInfo>,
    pub moves: Vec<MoveInfo>,
    pub issues: Vec<BorrowIssue>,
}

#[derive(Debug, Clone)]
pub struct BorrowIssue {
    pub severity: BorrowIssueSeverity,
    pub message: String,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowIssueSeverity {
    Error,
    Warning,
    Note,
}

pub fn analyze_borrows(db: &CompilationDatabase, path: &str) -> BorrowVisualization {
    let mut borrows = Vec::new();
    let mut moves = Vec::new();
    let mut issues = Vec::new();

    let analysis = match db.analysis.get(path) {
        Some(a) => a,
        None => {
            return BorrowVisualization {
                borrows,
                moves,
                issues,
            }
        }
    };

    let mir = match &analysis.mir {
        Some(m) => m,
        None => {
            return BorrowVisualization {
                borrows,
                moves,
                issues,
            }
        }
    };

    // Analyze MIR instructions for borrow patterns
    let mut var_definitions: HashMap<String, Vec<Span>> = HashMap::new();
    let mut var_uses: HashMap<String, Vec<Span>> = HashMap::new();
    let mut var_moves: HashMap<String, Vec<Span>> = HashMap::new();
    let mut instruction_idx: usize = 0;

    for func in &mir.functions {
        for block in &func.blocks {
            for instr in &block.instrs {
                instruction_idx += 1;

                match instr {
                    crate::mir::Instruction::ConstInt { dest, .. }
                    | crate::mir::Instruction::ConstStr { dest, .. }
                    | crate::mir::Instruction::ConstBool { dest, .. } => {
                        var_definitions
                            .entry(dest.clone())
                            .or_default()
                            .push(Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ));
                    }
                    crate::mir::Instruction::Move { dest, src } => {
                        var_definitions
                            .entry(dest.clone())
                            .or_default()
                            .push(Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ));
                        var_moves.entry(src.clone()).or_default().push(Span::new(
                            block.id,
                            instruction_idx,
                            block.id,
                            instruction_idx,
                        ));
                        moves.push(MoveInfo {
                            variable: src.clone(),
                            source_span: Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ),
                            dest_span: Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ),
                            is_valid: true,
                        });
                    }
                    crate::mir::Instruction::LinearMove { dest, src } => {
                        var_definitions
                            .entry(dest.clone())
                            .or_default()
                            .push(Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ));
                        var_moves.entry(src.clone()).or_default().push(Span::new(
                            block.id,
                            instruction_idx,
                            block.id,
                            instruction_idx,
                        ));
                        moves.push(MoveInfo {
                            variable: src.clone(),
                            source_span: Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ),
                            dest_span: Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ),
                            is_valid: true,
                        });
                    }
                    crate::mir::Instruction::Print { src }
                    | crate::mir::Instruction::Return { value: src } => {
                        var_uses.entry(src.clone()).or_default().push(Span::new(
                            block.id,
                            instruction_idx,
                            block.id,
                            instruction_idx,
                        ));
                    }
                    crate::mir::Instruction::BinaryOp {
                        dest, left, right, ..
                    } => {
                        var_definitions
                            .entry(dest.clone())
                            .or_default()
                            .push(Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ));
                        var_uses.entry(left.clone()).or_default().push(Span::new(
                            block.id,
                            instruction_idx,
                            block.id,
                            instruction_idx,
                        ));
                        var_uses.entry(right.clone()).or_default().push(Span::new(
                            block.id,
                            instruction_idx,
                            block.id,
                            instruction_idx,
                        ));
                    }
                    crate::mir::Instruction::Assign { dest, src } => {
                        var_definitions
                            .entry(dest.clone())
                            .or_default()
                            .push(Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ));
                        var_uses.entry(src.clone()).or_default().push(Span::new(
                            block.id,
                            instruction_idx,
                            block.id,
                            instruction_idx,
                        ));
                    }
                    crate::mir::Instruction::Call { dest, args, .. } => {
                        var_definitions
                            .entry(dest.clone())
                            .or_default()
                            .push(Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ));
                        for arg in args {
                            var_uses.entry(arg.clone()).or_default().push(Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ));
                        }
                    }
                    crate::mir::Instruction::FieldAccess { dest, base, .. }
                    | crate::mir::Instruction::StructAccess { dest, base, .. } => {
                        var_definitions
                            .entry(dest.clone())
                            .or_default()
                            .push(Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ));
                        var_uses.entry(base.clone()).or_default().push(Span::new(
                            block.id,
                            instruction_idx,
                            block.id,
                            instruction_idx,
                        ));
                        // Field access creates a shared borrow
                        borrows.push(BorrowInfo {
                            variable: base.clone(),
                            borrow_span: Span::new(
                                block.id,
                                instruction_idx,
                                block.id,
                                instruction_idx,
                            ),
                            kind: BorrowKind::Shared,
                            lifetime_start: instruction_idx,
                            lifetime_end: instruction_idx + 10, // Conservative estimate
                            is_valid: true,
                        });
                    }
                    crate::mir::Instruction::Drop { var } => {
                        // Check for double drop
                        if let Some(previous_drops) = var_moves.get(var) {
                            if previous_drops.len() > 1 {
                                issues.push(BorrowIssue {
                                    severity: BorrowIssueSeverity::Error,
                                    message: format!("potential double drop of '{}'", var),
                                    spans: vec![Span::new(
                                        block.id,
                                        instruction_idx,
                                        block.id,
                                        instruction_idx,
                                    )],
                                });
                            }
                        }
                        // Check for use-after-move
                        if let Some(uses) = var_uses.get(var) {
                            for use_span in uses {
                                if use_span.start_line == block.id
                                    && use_span.start_col > instruction_idx
                                {
                                    issues.push(BorrowIssue {
                                        severity: BorrowIssueSeverity::Error,
                                        message: format!("use of moved value '{}'", var),
                                        spans: vec![
                                            Span::new(
                                                block.id,
                                                instruction_idx,
                                                block.id,
                                                instruction_idx,
                                            ),
                                            use_span.clone(),
                                        ],
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Check for uninitialized use
    for (var, uses) in &var_uses {
        if !var_definitions.contains_key(var) {
            for use_span in uses {
                issues.push(BorrowIssue {
                    severity: BorrowIssueSeverity::Error,
                    message: format!("use of possibly uninitialized variable '{}'", var),
                    spans: vec![use_span.clone()],
                });
            }
        }
    }

    BorrowVisualization {
        borrows,
        moves,
        issues,
    }
}

pub fn get_borrow_visualization(db: &CompilationDatabase, path: &str) -> BorrowVisualization {
    analyze_borrows(db, path)
}

pub fn get_live_variables_at(
    db: &CompilationDatabase,
    path: &str,
    _line: usize,
    _col: usize,
) -> Vec<String> {
    let analysis = match db.analysis.get(path) {
        Some(a) => a,
        None => return Vec::new(),
    };

    let mir = match &analysis.mir {
        Some(m) => m,
        None => return Vec::new(),
    };

    let mut live_vars: Vec<String> = Vec::new();

    // Simple liveness analysis - check what's used after this point
    for func in &mir.functions {
        for block in &func.blocks {
            for instr in &block.instrs {
                match instr {
                    crate::mir::Instruction::Print { src }
                    | crate::mir::Instruction::Return { value: src }
                        if !live_vars.contains(src) =>
                    {
                        live_vars.push(src.clone());
                    }
                    crate::mir::Instruction::Call { args, .. } => {
                        for arg in args {
                            if !live_vars.contains(arg) {
                                live_vars.push(arg.clone());
                            }
                        }
                    }
                    crate::mir::Instruction::BinaryOp { left, right, .. } => {
                        if !live_vars.contains(left) {
                            live_vars.push(left.clone());
                        }
                        if !live_vars.contains(right) {
                            live_vars.push(right.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    live_vars
}
