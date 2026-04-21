// Minimal LIR (Low-level IR) scaffold for Step 6 (LIR + Cranelift).
// This crate provides a tiny, well-typed IR suitable for lowering from MIR
// and for feeding into a Cranelift-backed codegen backend.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    I64,
    Void,
    Ptr,
}

#[derive(Debug, Clone, Default)]
pub struct Module {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Type>,
    // Support multiple return values (possibly zero)
    pub rets: Vec<Type>,
    pub body: Vec<Instr>,
}

#[derive(Debug, Clone)]
pub enum Instr {
    Const(i64),
    Add,
    Sub,
    Mul,
    Div,
    Load(u32),
    Store(u32),
    Call(String),
    Ret,
    Jump(usize),
    CondJump { if_true: usize, if_false: usize },
    Drop(u32),
    Nop,
}

impl Module {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }
    pub fn add_function(&mut self, f: Function) {
        self.functions.push(f);
    }
}

impl Function {
    // Convenience constructor for a single return value (or Void)
    pub fn new(name: impl Into<String>, params: Vec<Type>, ret: Type, body: Vec<Instr>) -> Self {
        let rets = if ret == Type::Void {
            Vec::new()
        } else {
            vec![ret]
        };
        Self {
            name: name.into(),
            params,
            rets,
            body,
        }
    }

    // Constructor for multiple returns
    pub fn new_multi(
        name: impl Into<String>,
        params: Vec<Type>,
        rets: Vec<Type>,
        body: Vec<Instr>,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            rets,
            body,
        }
    }
}

/// Example module used by tests and as a small smoke fixture.
pub fn example_module() -> Module {
    let mut m = Module::new();
    let main = Function::new(
        "main",
        vec![],
        Type::I64,
        vec![Instr::Const(40), Instr::Const(2), Instr::Add, Instr::Ret],
    );
    m.add_function(main);
    m
}
