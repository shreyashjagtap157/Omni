use crate::ast::{Expr, Stmt};
use crate::lexer::TokenKind;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum MacroArg {
    Expr(Expr),
    Pattern(String),
    Repetition(MacroRepetition),
}

#[derive(Debug, Clone)]
pub struct MacroRepetition {
    pub arg: Box<MacroArg>,
    pub separator: Option<TokenKind>,
    pub kind: RepetitionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepetitionKind {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
}

#[derive(Debug, Clone)]
pub struct MacroRule {
    pub pattern: Vec<MacroPattern>,
    pub template: Vec<MacroToken>,
}

#[derive(Debug, Clone)]
pub enum MacroPattern {
    Literal(TokenKind),
    Ident(String),
    Fragment(String, FragmentSpecifier),
    Repetition {
        name: String,
        kind: RepetitionKind,
        separator: Option<TokenKind>,
    },
}

#[derive(Debug, Clone)]
pub enum FragmentSpecifier {
    Expr,
    Block,
    Stmt,
    Pat,
    Ty,
    Ident,
    Path,
    Meta,
    Item,
    Lifetime,
}

#[derive(Debug, Clone)]
pub enum MacroToken {
    Literal(String),
    Fragment(String, FragmentReplacement),
    Repetition { name: String, kind: RepetitionKind },
}

#[derive(Debug, Clone)]
pub enum FragmentReplacement {
    Expr,
    Block,
    Stmt,
    Ty,
    Ident,
}

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub name: String,
    pub rules: Vec<MacroRule>,
    pub is_macro_rules: bool,
}

#[derive(Debug, Default)]
pub struct MacroExpansionContext {
    pub macros: HashMap<String, MacroDefinition>,
    pub hygiene: HashMap<String, usize>,
    pub depth: usize,
}

impl MacroExpansionContext {
    pub fn new() -> Self {
        MacroExpansionContext {
            macros: HashMap::new(),
            hygiene: HashMap::new(),
            depth: 0,
        }
    }

    pub fn add_macro(&mut self, macro_def: MacroDefinition) {
        self.macros.insert(macro_def.name.clone(), macro_def);
    }

    pub fn get_macro(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
    }

    pub fn generate_unique_ident(&mut self, base: &str) -> String {
        let counter = self.hygiene.entry(base.to_string()).or_insert(0);
        *counter += 1;
        format!("{}_{}", base, counter)
    }

    pub fn expand_macro(&self, name: &str, args: &[MacroArg]) -> Result<Vec<Stmt>, String> {
        let macro_def = self
            .get_macro(name)
            .ok_or_else(|| format!("Macro '{}' not found", name))?;

        for rule in &macro_def.rules {
            if let Some(template) = self.try_match_rule(rule, args)? {
                return Ok(template);
            }
        }

        Err(format!("No matching rule found for macro '{}'", name))
    }

    fn try_match_rule(
        &self,
        rule: &MacroRule,
        args: &[MacroArg],
    ) -> Result<Option<Vec<Stmt>>, String> {
        if rule.pattern.len() != args.len() {
            return Ok(None);
        }

        let mut bindings: HashMap<String, MacroArg> = HashMap::new();

        for (pattern, arg) in rule.pattern.iter().zip(args.iter()) {
            match (pattern, arg) {
                (MacroPattern::Ident(expected), MacroArg::Pattern(actual))
                    if expected == actual => {}
                (MacroPattern::Ident(expected), MacroArg::Expr(Expr::Var(actual)))
                    if expected == actual => {}
                (MacroPattern::Literal(_tok), MacroArg::Expr(_)) => {}
                (MacroPattern::Fragment(name, spec), arg_value) => {
                    let matches = match (spec, arg_value) {
                        (FragmentSpecifier::Expr, MacroArg::Expr(_)) => true,
                        (FragmentSpecifier::Stmt, MacroArg::Expr(_) | MacroArg::Pattern(_)) => true,
                        (FragmentSpecifier::Pat, MacroArg::Pattern(_)) => true,
                        (FragmentSpecifier::Ty, MacroArg::Pattern(_)) => true,
                        (
                            FragmentSpecifier::Ident,
                            MacroArg::Pattern(_) | MacroArg::Expr(Expr::Var(_)),
                        ) => true,
                        (
                            FragmentSpecifier::Path,
                            MacroArg::Pattern(_) | MacroArg::Expr(Expr::Var(_)),
                        ) => true,
                        (FragmentSpecifier::Meta, _) => true,
                        (FragmentSpecifier::Item, _) => true,
                        (FragmentSpecifier::Lifetime, MacroArg::Pattern(_)) => true,
                        _ => false,
                    };
                    if !matches {
                        return Ok(None);
                    }
                    bindings.insert(name.clone(), arg_value.clone());
                }
                (MacroPattern::Repetition { .. }, _) => {
                    return Err(
                        "macro repetition matching is not yet implemented in the expansion engine"
                            .to_string(),
                    );
                }
                _ => return Ok(None),
            }
        }

        let mut result = Vec::new();
        for token in &rule.template {
            match token {
                MacroToken::Literal(s) => {
                    result.push(Stmt::ExprStmt(Expr::StringLit(s.clone())));
                }
                MacroToken::Fragment(name, replacement) => {
                    if let Some(binding) = bindings.get(name) {
                        match (replacement, binding) {
                            (FragmentReplacement::Expr, MacroArg::Expr(e)) => {
                                result.push(Stmt::ExprStmt(e.clone()));
                            }
                            (FragmentReplacement::Block, MacroArg::Expr(e)) => {
                                result.push(Stmt::ExprStmt(e.clone()));
                            }
                            (FragmentReplacement::Stmt, MacroArg::Expr(e)) => {
                                result.push(Stmt::ExprStmt(e.clone()));
                            }
                            (FragmentReplacement::Ty, MacroArg::Pattern(p)) => {
                                result.push(Stmt::ExprStmt(Expr::Var(p.clone())));
                            }
                            (FragmentReplacement::Ident, MacroArg::Pattern(p)) => {
                                result.push(Stmt::ExprStmt(Expr::Var(p.clone())));
                            }
                            _ => {}
                        }
                    }
                }
                MacroToken::Repetition { .. } => {}
            }
        }

        Ok(Some(result))
    }
}

pub fn parse_macro_rules(tokens: &[crate::lexer::Token]) -> Result<Vec<MacroRule>, String> {
    let mut rules = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let mut pattern = Vec::new();

        // Parse pattern until =>
        while i < tokens.len() {
            if tokens[i].text == "=>" {
                i += 1;
                break;
            }

            match tokens[i].kind {
                TokenKind::Ident => {
                    let name = tokens[i].text.clone();
                    if i + 1 < tokens.len() && tokens[i + 1].text == ":" {
                        // Fragment specifier
                        i += 2;
                        if i < tokens.len() {
                            let fragment = match tokens[i].text.as_str() {
                                "expr" => FragmentSpecifier::Expr,
                                "block" => FragmentSpecifier::Block,
                                "stmt" => FragmentSpecifier::Stmt,
                                "pat" => FragmentSpecifier::Pat,
                                "ty" => FragmentSpecifier::Ty,
                                "ident" => FragmentSpecifier::Ident,
                                "path" => FragmentSpecifier::Path,
                                "meta" => FragmentSpecifier::Meta,
                                "item" => FragmentSpecifier::Item,
                                "lifetime" => FragmentSpecifier::Lifetime,
                                _ => FragmentSpecifier::Ident,
                            };
                            pattern.push(MacroPattern::Fragment(name, fragment));
                        }
                    } else {
                        pattern.push(MacroPattern::Ident(name));
                    }
                }
                TokenKind::LineComment => {}
                _ => {
                    pattern.push(MacroPattern::Literal(tokens[i].kind.clone()));
                }
            }
            i += 1;
        }

        // Parse template
        let mut template = Vec::new();
        while i < tokens.len() && tokens[i].kind != TokenKind::Newline {
            match tokens[i].kind {
                TokenKind::Ident => {
                    if tokens[i].text.starts_with('$') {
                        template.push(MacroToken::Fragment(
                            tokens[i].text[1..].to_string(),
                            FragmentReplacement::Expr,
                        ));
                    } else {
                        template.push(MacroToken::Literal(tokens[i].text.clone()));
                    }
                }
                TokenKind::LineComment => {}
                _ => {
                    template.push(MacroToken::Literal(tokens[i].text.clone()));
                }
            }
            i += 1;
        }

        if !pattern.is_empty() && !template.is_empty() {
            rules.push(MacroRule { pattern, template });
        }

        i += 1;
    }

    Ok(rules)
}

pub fn define_macro(name: &str, rules: Vec<MacroRule>) -> MacroDefinition {
    MacroDefinition {
        name: name.to_string(),
        rules,
        is_macro_rules: name.starts_with("macro_rules"),
    }
}
