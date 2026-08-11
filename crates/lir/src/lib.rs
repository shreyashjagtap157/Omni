// Bootstrap LIR (Low-level IR) infrastructure shared by native emission and development oracles.
// This crate provides a tiny, well-typed IR suitable for lowering from MIR
// and for feeding into a Cranelift-backed codegen backend.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    I64,
    Void,
    /// Pointer to a compiler-validated contiguous sequence of eight-byte cells.
    /// The cell count is part of the type so the native backend can reject
    /// out-of-bounds indirect ABI accesses before emission.
    Ptr(u32),
}

#[derive(Debug, Clone, Default)]
pub struct Module {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Type>,
    pub rets: Vec<Type>,
    pub body: Vec<Instr>,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Instr {
    Const(i64),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    Not,
    Load(u32),
    Store(u32),
    Call(String),
    Ret,
    Jump(usize),
    CondJump {
        if_true: usize,
        if_false: usize,
    },
    Drop(u32),
    PrintStr(String),
    /// Push a pointer to immutable UTF-8 bytes in the native module's literal pool.
    StringRef(String),
    /// Pointer to immutable binary data in the module literal pool. Unlike
    /// StringRef this payload is not UTF-8 constrained.
    BytesRef(Vec<u8>),
    /// Consume `(data_ptr, byte_len)` and print the bytes followed by a newline.
    PrintBytes,
    /// Consume `(data_ptr, byte_len, index)`, bounds-check the index, and push
    /// the selected byte zero-extended to an i64 scalar cell.
    LoadByteIndex,
    LoadOffset(u32, i64),
    StoreOffset(u32, i64),
    BoundsCheck(u64),
    /// Bounds-checked load from a contiguous local scalar-cell aggregate.
    /// Consumes a signed index, checks 0 <= index < len, and pushes the cell.
    LoadIndex {
        base: u32,
        len: u64,
    },
    /// Address of a frame-local cell. Used only to pass compiler-owned value
    /// storage through the v0.1.4 indirect ABI.
    GetAddr(u32),
    /// Load/store through a pointer parameter with a statically validated cell
    /// offset. These operations intentionally avoid general source-level raw
    /// pointer arithmetic.
    LoadPtrOffset(u32, i64),
    StorePtrOffset(u32, i64),
    AddOffset,
    LoadInd,
    StoreInd,
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
    pub fn new(
        name: impl Into<String>,
        params: Vec<Type>,
        ret: Type,
        body: Vec<Instr>,
        effects: Vec<String>,
    ) -> Self {
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
            effects,
        }
    }

    // Constructor for multiple returns
    pub fn new_multi(
        name: impl Into<String>,
        params: Vec<Type>,
        rets: Vec<Type>,
        body: Vec<Instr>,
        effects: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            rets,
            body,
            effects,
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
        vec![],
    );
    m.add_function(main);
    m
}
