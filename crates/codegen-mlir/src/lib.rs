//! Experimental MLIR text fixtures.
//!
//! Omni v0.1.4 does not qualify an MLIR backend.  This crate intentionally
//! preserves a small amount of future-facing MLIR text infrastructure, but it
//! does not claim to lower arbitrary LIR correctly and it never substitutes a
//! different execution backend.  Production/native execution belongs to
//! `codegen-native`.

use lir::Module;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlirDialect {
    Func,
    Arith,
    Cf,
    MemRef,
    Linalg,
}

impl MlirDialect {
    pub fn name(self) -> &'static str {
        match self {
            Self::Func => "func",
            Self::Arith => "arith",
            Self::Cf => "cf",
            Self::MemRef => "memref",
            Self::Linalg => "linalg",
        }
    }
}

pub const MLIR_QUALIFIED: bool = false;

fn unavailable() -> String {
    "MLIR lowering/execution is not qualified in Omni v0.1.4; the crate is retained as future backend infrastructure and must not substitute Cranelift or another runtime"
        .to_string()
}

/// LIR-to-MLIR lowering is deliberately unavailable until the backend has a
/// complete stack/SSA, control-flow, memory, ABI, and semantic-validation path.
pub fn emit_mlir_text(_module: &Module) -> Result<String, String> {
    Err(unavailable())
}

/// Omni never aliases the MLIR backend to another backend.  Execution remains
/// unavailable until actual MLIR/LLVM lowering is implemented and qualified.
pub fn compile_and_run_with_mlir(_module: &Module) -> Result<Vec<i64>, String> {
    Err(unavailable())
}

/// Historical API name kept so callers get an explicit error instead of a
/// missing symbol.  It no longer delegates to Cranelift.
pub fn compile_and_run_with_mlir_jit(_module: &Module) -> Result<Vec<i64>, String> {
    Err(unavailable())
}

/// Small tensor-workload fixture for future toolchain-backed MLIR tests.  This
/// emits MLIR text directly and is not presented as an Omni LIR lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TensorAddWorkload {
    pub length: usize,
}

impl TensorAddWorkload {
    pub fn new(length: usize) -> Self {
        Self {
            length: length.max(1),
        }
    }

    pub fn emit_mlir_text(&self) -> String {
        emit_tensor_add_mlir_text(self.length)
    }
}

pub fn emit_tensor_add_mlir_text(length: usize) -> String {
    let tensor_type = format!("tensor<{}xi64>", length.max(1));
    let template = r#"#map_1d_identity = affine_map<(d0) -> (d0)>
module {
    func.func @tensor_add(%lhs: __TY__, %rhs: __TY__) -> __TY__ {
        %result = tensor.empty() : __TY__
        %0 = linalg.generic {
            indexing_maps = [#map_1d_identity, #map_1d_identity, #map_1d_identity],
            iterator_types = ["parallel"]
        } ins(%lhs, %rhs : __TY__, __TY__) outs(%result : __TY__) {
        ^bb0(%a: i64, %b: i64, %acc: i64):
            %sum = arith.addi %a, %b : i64
            linalg.yield %sum : i64
        } -> __TY__
        func.return %0 : __TY__
    }
}
"#;
    template.replace("__TY__", &tensor_type)
}

pub fn emit_control_flow_demo_mlir_text() -> String {
    r#"module {
  func.func @control_flow_demo(%cond: i1, %lhs: i64, %rhs: i64) -> i64 {
    cf.cond_br %cond, ^bb1, ^bb2
  ^bb1:
    cf.br ^bb3(%lhs : i64)
  ^bb2:
    cf.br ^bb3(%rhs : i64)
  ^bb3(%value: i64):
    func.return %value : i64
  }
}
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linalg_fixture_is_explicit_text_fixture() {
        let text = TensorAddWorkload::new(4).emit_mlir_text();
        assert!(text.contains("linalg.generic"));
        assert!(text.contains("tensor<4xi64>"));
    }

    #[test]
    fn lir_backend_fails_closed() {
        let module = lir::example_module();
        let err = emit_mlir_text(&module).expect_err("backend must be unavailable");
        assert!(err.contains("not qualified"));
        assert!(compile_and_run_with_mlir(&module).is_err());
        assert!(compile_and_run_with_mlir_jit(&module).is_err());
    }
}
