//! Omni's owned native AOT backend.
//!
//! This crate deliberately does not use a VM, JIT, C compiler, assembler, or
//! external linker for its currently supported target. It translates Omni LIR
//! directly into x86-64 machine instructions and writes a Linux ELF64 image.
//! Additional object/executable formats are added target-by-target.

use lir::{Instr, Module, Type};
const OMNI_PROJECT_VERSION: &str = "0.2.0.2";

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

pub const ARITHMETIC_FAULT_EXIT: i32 = 101;
pub const BOUNDS_FAULT_EXIT: i32 = 102;

static NATIVE_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTarget {
    X86_64Linux,
}

impl NativeTarget {
    pub fn host() -> Result<Self, String> {
        match (std::env::consts::ARCH, std::env::consts::OS) {
            ("x86_64", "linux") => Ok(Self::X86_64Linux),
            (arch, os) => Err(format!(
                "owned native AOT backend does not yet support host target {arch}-{os}; \
                 currently supported: x86_64-linux"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NativeArtifact {
    pub path: PathBuf,
    pub target: NativeTarget,
    pub bytes_written: usize,
}

#[derive(Debug, Clone)]
pub struct NativeRunResult {
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn compile_to_native(module: &Module, output_path: &Path) -> Result<PathBuf, String> {
    compile_to_target(module, output_path, NativeTarget::host()?).map(|a| a.path)
}

pub fn compile_to_target(
    module: &Module,
    output_path: &Path,
    target: NativeTarget,
) -> Result<NativeArtifact, String> {
    if module.functions.is_empty() {
        return Err("cannot emit a native artifact for an empty LIR module".to_string());
    }

    let bytes = match target {
        NativeTarget::X86_64Linux => x86_64_linux::emit_elf(module)?,
    };

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create output directory '{}': {e}",
                    parent.display()
                )
            })?;
        }
    }
    fs::write(output_path, &bytes)
        .map_err(|e| format!("failed to write '{}': {e}", output_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(output_path)
            .map_err(|e| format!("failed to stat '{}': {e}", output_path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(output_path, permissions)
            .map_err(|e| format!("failed to chmod '{}': {e}", output_path.display()))?;
    }

    Ok(NativeArtifact {
        path: output_path.to_path_buf(),
        target,
        bytes_written: bytes.len(),
    })
}

fn execute_native_artifact(path: &Path) -> std::io::Result<Output> {
    const MAX_ATTEMPTS: u32 = 8;
    for attempt in 0..MAX_ATTEMPTS {
        match Command::new(path).output() {
            Err(error) if error.raw_os_error() == Some(26) && attempt + 1 < MAX_ATTEMPTS => {
                thread::sleep(Duration::from_millis(u64::from(attempt + 1)));
            }
            result => return result,
        }
    }
    unreachable!("bounded native execution retry loop must return")
}

pub fn compile_and_run_native(module: &Module) -> Result<NativeRunResult, String> {
    let target = NativeTarget::host()?;
    let mut path = std::env::temp_dir();
    let sequence = NATIVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let unique = format!(
        "omni-native-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos(),
        sequence,
    );
    path.push(unique);
    let artifact = compile_to_target(module, &path, target)?;
    let output = match execute_native_artifact(&artifact.path) {
        Ok(output) => output,
        Err(e) => {
            let _ = fs::remove_file(&artifact.path);
            return Err(format!(
                "failed to execute '{}': {e}",
                artifact.path.display()
            ));
        }
    };
    fs::remove_file(&artifact.path).map_err(|e| {
        format!(
            "native program executed but temporary artifact '{}' could not be removed: {e}",
            artifact.path.display()
        )
    })?;
    Ok(NativeRunResult {
        status: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

mod x86_64_linux {
    use super::*;

    const ELF_BASE: u64 = 0x0040_0000;
    const CODE_OFFSET: usize = 0x1000;
    const ELF_HEADER_SIZE: usize = 64;
    const PROGRAM_HEADER_SIZE: usize = 56;

    #[derive(Debug, Clone)]
    enum PatchKind {
        Rel32,
        RipRel32,
    }

    #[derive(Debug, Clone)]
    struct Patch {
        at: usize,
        label: String,
        kind: PatchKind,
    }

    #[derive(Default)]
    struct Asm {
        bytes: Vec<u8>,
        labels: HashMap<String, usize>,
        patches: Vec<Patch>,
    }

    impl Asm {
        fn pos(&self) -> usize {
            self.bytes.len()
        }

        fn label(&mut self, name: impl Into<String>) -> Result<(), String> {
            let name = name.into();
            if self.labels.insert(name.clone(), self.pos()).is_some() {
                return Err(format!("duplicate native label '{name}'"));
            }
            Ok(())
        }

        fn b(&mut self, value: u8) {
            self.bytes.push(value);
        }

        fn bs(&mut self, values: &[u8]) {
            self.bytes.extend_from_slice(values);
        }

        fn i32(&mut self, value: i32) {
            self.bs(&value.to_le_bytes());
        }

        fn u32(&mut self, value: u32) {
            self.bs(&value.to_le_bytes());
        }

        fn i64(&mut self, value: i64) {
            self.bs(&value.to_le_bytes());
        }

        fn call(&mut self, label: impl Into<String>) {
            self.b(0xE8);
            let at = self.pos();
            self.u32(0);
            self.patches.push(Patch {
                at,
                label: label.into(),
                kind: PatchKind::Rel32,
            });
        }

        fn jmp(&mut self, label: impl Into<String>) {
            self.b(0xE9);
            let at = self.pos();
            self.u32(0);
            self.patches.push(Patch {
                at,
                label: label.into(),
                kind: PatchKind::Rel32,
            });
        }

        fn jcc(&mut self, opcode2: u8, label: impl Into<String>) {
            self.bs(&[0x0F, opcode2]);
            let at = self.pos();
            self.u32(0);
            self.patches.push(Patch {
                at,
                label: label.into(),
                kind: PatchKind::Rel32,
            });
        }

        fn lea_rsi_rip(&mut self, label: impl Into<String>) {
            self.bs(&[0x48, 0x8D, 0x35]);
            let at = self.pos();
            self.u32(0);
            self.patches.push(Patch {
                at,
                label: label.into(),
                kind: PatchKind::RipRel32,
            });
        }

        fn lea_rax_rip(&mut self, label: impl Into<String>) {
            self.bs(&[0x48, 0x8D, 0x05]);
            let at = self.pos();
            self.u32(0);
            self.patches.push(Patch {
                at,
                label: label.into(),
                kind: PatchKind::RipRel32,
            });
        }

        fn patch(mut self) -> Result<Vec<u8>, String> {
            for patch in &self.patches {
                let target = *self
                    .labels
                    .get(&patch.label)
                    .ok_or_else(|| format!("undefined native label '{}'", patch.label))?;
                let next = patch.at + 4;
                let disp = target as i64 - next as i64;
                let disp = i32::try_from(disp).map_err(|_| {
                    format!(
                        "native relative displacement to '{}' is out of range",
                        patch.label
                    )
                })?;
                let encoded = disp.to_le_bytes();
                self.bytes[patch.at..patch.at + 4].copy_from_slice(&encoded);
                match patch.kind {
                    PatchKind::Rel32 | PatchKind::RipRel32 => {}
                }
            }
            Ok(self.bytes)
        }
    }

    fn fn_label(name: &str) -> String {
        format!("fn::{name}")
    }

    fn ip_label(name: &str, ip: usize) -> String {
        format!("fn::{name}::ip::{ip}")
    }

    fn string_label(id: usize) -> String {
        format!("str::{id}")
    }

    fn bytes_label(id: usize) -> String {
        format!("bytes::{id}")
    }

    fn max_slot(function: &lir::Function) -> Option<u32> {
        function
            .body
            .iter()
            .filter_map(|i| match i {
                Instr::Load(s)
                | Instr::Store(s)
                | Instr::Drop(s)
                | Instr::LoadOffset(s, _)
                | Instr::StoreOffset(s, _)
                | Instr::LoadIndex { base: s, .. }
                | Instr::GetAddr(s)
                | Instr::LoadPtrOffset(s, _)
                | Instr::StorePtrOffset(s, _) => Some(*s),
                _ => None,
            })
            .max()
    }

    fn aligned_frame_bytes(function: &lir::Function) -> usize {
        let slots = max_slot(function).map(|s| s as usize + 1).unwrap_or(0);
        let raw = slots.saturating_mul(8);
        (raw + 15) & !15
    }

    fn local_disp(slot: u32) -> Result<i32, String> {
        let bytes = (slot as i64 + 1) * 8;
        let bytes = i32::try_from(bytes)
            .map_err(|_| format!("local slot {slot} exceeds native frame encoding limit"))?;
        Ok(-bytes)
    }

    fn emit_mov_rax_imm64(a: &mut Asm, value: i64) {
        a.bs(&[0x48, 0xB8]);
        a.i64(value);
    }

    fn emit_push_param_regs(a: &mut Asm, count: usize) -> Result<(), String> {
        if count > 6 {
            return Err(format!(
                "owned x86-64 Linux backend currently supports at most 6 integer parameters, got {count}"
            ));
        }
        // SysV order: rdi, rsi, rdx, rcx, r8, r9. Push in source order so
        // the final parameter is at the top of the evaluation stack.
        let encodings: [&[u8]; 6] = [
            &[0x57],       // push rdi
            &[0x56],       // push rsi
            &[0x52],       // push rdx
            &[0x51],       // push rcx
            &[0x41, 0x50], // push r8
            &[0x41, 0x51], // push r9
        ];
        for bytes in encodings.iter().take(count) {
            a.bs(bytes);
        }
        Ok(())
    }

    fn emit_pop_call_args(a: &mut Asm, count: usize) -> Result<(), String> {
        if count > 6 {
            return Err(format!(
                "owned x86-64 Linux backend currently supports at most 6 integer arguments, got {count}"
            ));
        }
        // Arguments were pushed in source order. Pop from last argument to
        // first into the corresponding registers.
        let pops: [&[u8]; 6] = [
            &[0x5F],       // pop rdi
            &[0x5E],       // pop rsi
            &[0x5A],       // pop rdx
            &[0x59],       // pop rcx
            &[0x41, 0x58], // pop r8
            &[0x41, 0x59], // pop r9
        ];
        for idx in (0..count).rev() {
            a.bs(pops[idx]);
        }
        Ok(())
    }

    fn emit_store_local(a: &mut Asm, slot: u32) -> Result<(), String> {
        a.b(0x58); // pop rax
        let disp = local_disp(slot)?;
        if (-128..=127).contains(&disp) {
            a.bs(&[0x48, 0x89, 0x45, disp as i8 as u8]);
        } else {
            a.bs(&[0x48, 0x89, 0x85]);
            a.i32(disp);
        }
        Ok(())
    }

    fn emit_load_local(a: &mut Asm, slot: u32) -> Result<(), String> {
        let disp = local_disp(slot)?;
        if (-128..=127).contains(&disp) {
            a.bs(&[0x48, 0x8B, 0x45, disp as i8 as u8]);
        } else {
            a.bs(&[0x48, 0x8B, 0x85]);
            a.i32(disp);
        }
        a.b(0x50); // push rax
        Ok(())
    }

    fn local_offset_disp(base_slot: u32, byte_offset: i64) -> Result<i32, String> {
        if byte_offset < 0 || byte_offset % 8 != 0 {
            return Err(format!(
                "aggregate byte offset {byte_offset} must be a non-negative multiple of 8"
            ));
        }
        let base = i64::from(local_disp(base_slot)?);
        let effective = base
            .checked_add(byte_offset)
            .ok_or_else(|| "aggregate local displacement overflow".to_string())?;
        if effective > -8 || effective < base {
            return Err(format!(
                "aggregate byte offset {byte_offset} escapes local frame from base slot {base_slot}"
            ));
        }
        i32::try_from(effective)
            .map_err(|_| format!("aggregate local displacement {effective} is out of range"))
    }

    fn validate_ptr_param_offset(
        function: &lir::Function,
        ptr_slot: u32,
        byte_offset: i64,
    ) -> Result<(), String> {
        if byte_offset < 0 || byte_offset % 8 != 0 {
            return Err(format!(
                "indirect ABI byte offset {byte_offset} must be a non-negative multiple of 8"
            ));
        }
        let cells = match function.params.get(ptr_slot as usize) {
            Some(Type::Ptr(cells)) => *cells,
            Some(other) => {
                return Err(format!(
                    "local slot {ptr_slot} is backed by parameter type {other:?}, not an indirect value pointer"
                ))
            }
            None => {
                return Err(format!(
                    "indirect ABI pointer slot {ptr_slot} is not an incoming parameter"
                ))
            }
        };
        let cell = u64::try_from(byte_offset / 8)
            .map_err(|_| "indirect ABI cell index conversion failed".to_string())?;
        if cell >= u64::from(cells) {
            return Err(format!(
                "indirect ABI offset {byte_offset} escapes {cells}-cell value"
            ));
        }
        i32::try_from(byte_offset).map_err(|_| {
            format!("indirect ABI offset {byte_offset} exceeds x86-64 displacement encoding")
        })?;
        Ok(())
    }

    fn emit_get_local_addr(a: &mut Asm, slot: u32) -> Result<(), String> {
        let disp = local_disp(slot)?;
        if (-128..=127).contains(&disp) {
            a.bs(&[0x48, 0x8D, 0x45, disp as i8 as u8]); // lea rax, [rbp+disp8]
        } else {
            a.bs(&[0x48, 0x8D, 0x85]); // lea rax, [rbp+disp32]
            a.i32(disp);
        }
        a.b(0x50);
        Ok(())
    }

    fn emit_load_ptr_offset(
        a: &mut Asm,
        function: &lir::Function,
        ptr_slot: u32,
        byte_offset: i64,
    ) -> Result<(), String> {
        validate_ptr_param_offset(function, ptr_slot, byte_offset)?;
        let ptr_disp = local_disp(ptr_slot)?;
        if (-128..=127).contains(&ptr_disp) {
            a.bs(&[0x48, 0x8B, 0x55, ptr_disp as i8 as u8]); // mov rdx, [rbp+disp8]
        } else {
            a.bs(&[0x48, 0x8B, 0x95]);
            a.i32(ptr_disp);
        }
        let offset =
            i32::try_from(byte_offset).map_err(|_| "pointer offset out of range".to_string())?;
        if offset == 0 {
            a.bs(&[0x48, 0x8B, 0x02]); // mov rax, [rdx]
        } else if (-128..=127).contains(&offset) {
            a.bs(&[0x48, 0x8B, 0x42, offset as i8 as u8]);
        } else {
            a.bs(&[0x48, 0x8B, 0x82]);
            a.i32(offset);
        }
        a.b(0x50);
        Ok(())
    }

    fn emit_store_ptr_offset(
        a: &mut Asm,
        function: &lir::Function,
        ptr_slot: u32,
        byte_offset: i64,
    ) -> Result<(), String> {
        validate_ptr_param_offset(function, ptr_slot, byte_offset)?;
        a.b(0x58); // pop value into rax
        let ptr_disp = local_disp(ptr_slot)?;
        if (-128..=127).contains(&ptr_disp) {
            a.bs(&[0x48, 0x8B, 0x55, ptr_disp as i8 as u8]);
        } else {
            a.bs(&[0x48, 0x8B, 0x95]);
            a.i32(ptr_disp);
        }
        let offset =
            i32::try_from(byte_offset).map_err(|_| "pointer offset out of range".to_string())?;
        if offset == 0 {
            a.bs(&[0x48, 0x89, 0x02]); // mov [rdx], rax
        } else if (-128..=127).contains(&offset) {
            a.bs(&[0x48, 0x89, 0x42, offset as i8 as u8]);
        } else {
            a.bs(&[0x48, 0x89, 0x82]);
            a.i32(offset);
        }
        Ok(())
    }

    fn emit_store_local_offset(
        a: &mut Asm,
        base_slot: u32,
        byte_offset: i64,
    ) -> Result<(), String> {
        a.b(0x58); // pop rax
        let disp = local_offset_disp(base_slot, byte_offset)?;
        if (-128..=127).contains(&disp) {
            a.bs(&[0x48, 0x89, 0x45, disp as i8 as u8]);
        } else {
            a.bs(&[0x48, 0x89, 0x85]);
            a.i32(disp);
        }
        Ok(())
    }

    fn emit_load_local_offset(a: &mut Asm, base_slot: u32, byte_offset: i64) -> Result<(), String> {
        let disp = local_offset_disp(base_slot, byte_offset)?;
        if (-128..=127).contains(&disp) {
            a.bs(&[0x48, 0x8B, 0x45, disp as i8 as u8]);
        } else {
            a.bs(&[0x48, 0x8B, 0x85]);
            a.i32(disp);
        }
        a.b(0x50); // push rax
        Ok(())
    }

    fn validate_local_index(base_slot: u32, len: u64) -> Result<(), String> {
        if len == 0 {
            return Err(
                "bounds-checked local index cannot target a zero-length aggregate".to_string(),
            );
        }
        let last = len
            .checked_sub(1)
            .and_then(|value| value.checked_mul(8))
            .ok_or_else(|| "aggregate indexed span overflow".to_string())?;
        let last = i64::try_from(last)
            .map_err(|_| "aggregate indexed span exceeds native displacement range".to_string())?;
        local_offset_disp(base_slot, last)?;
        i64::try_from(len).map_err(|_| "bounds length exceeds i64".to_string())?;
        Ok(())
    }

    fn emit_load_local_index(a: &mut Asm, base_slot: u32, len: u64) -> Result<(), String> {
        validate_local_index(base_slot, len)?;
        a.b(0x58); // pop rax = signed index
        a.bs(&[0x48, 0x85, 0xC0]); // test rax, rax
        a.jcc(0x88, "runtime::bounds_fault"); // js
        a.bs(&[0x48, 0xB9]); // mov rcx, imm64
        a.i64(i64::try_from(len).map_err(|_| "bounds length exceeds i64".to_string())?);
        a.bs(&[0x48, 0x39, 0xC8]); // cmp rax, rcx
        a.jcc(0x83, "runtime::bounds_fault"); // jae
        a.bs(&[0x48, 0xC1, 0xE0, 0x03]); // shl rax, 3
        a.bs(&[0x48, 0x89, 0xEA]); // mov rdx, rbp
        a.bs(&[0x48, 0x01, 0xC2]); // add rdx, rax
        let disp = local_disp(base_slot)?;
        if (-128..=127).contains(&disp) {
            a.bs(&[0x48, 0x8B, 0x42, disp as i8 as u8]); // mov rax, [rdx+disp8]
        } else {
            a.bs(&[0x48, 0x8B, 0x82]); // mov rax, [rdx+disp32]
            a.i32(disp);
        }
        a.b(0x50); // push loaded cell
        Ok(())
    }

    fn emit_zero_local(a: &mut Asm, slot: u32) -> Result<(), String> {
        let disp = local_disp(slot)?;
        if (-128..=127).contains(&disp) {
            a.bs(&[0x48, 0xC7, 0x45, disp as i8 as u8, 0, 0, 0, 0]);
        } else {
            a.bs(&[0x48, 0xC7, 0x85]);
            a.i32(disp);
            a.u32(0);
        }
        Ok(())
    }

    fn emit_cmp_result(a: &mut Asm, setcc: u8) {
        a.b(0x5B); // pop rbx = rhs
        a.b(0x58); // pop rax = lhs
        a.bs(&[0x48, 0x39, 0xD8]); // cmp rax, rbx
        a.bs(&[0x0F, setcc, 0xC0]); // setcc al
        a.bs(&[0x48, 0x0F, 0xB6, 0xC0]); // movzx rax, al
        a.b(0x50); // push rax
    }

    fn emit_print_str(a: &mut Asm, label: &str, len: usize) -> Result<(), String> {
        let len = i64::try_from(len).map_err(|_| "string literal too large".to_string())?;
        // write(1, ptr, len)
        emit_mov_rax_imm64(a, 1);
        emit_mov_rax_imm64_to_reg_rdi(a, 1);
        a.lea_rsi_rip(label.to_string());
        emit_mov_rax_imm64_to_reg_rdx(a, len);
        a.bs(&[0x0F, 0x05]); // syscall
                             // write(1, "\n", 1)
        emit_mov_rax_imm64(a, 1);
        emit_mov_rax_imm64_to_reg_rdi(a, 1);
        a.lea_rsi_rip("runtime::newline".to_string());
        emit_mov_rax_imm64_to_reg_rdx(a, 1);
        a.bs(&[0x0F, 0x05]);
        Ok(())
    }

    fn emit_print_bytes_from_stack(a: &mut Asm) {
        // Stack order is data pointer, then byte length.
        a.b(0x5A); // pop rdx = len
        a.b(0x5E); // pop rsi = data
        emit_mov_rax_imm64(a, 1); // write
        emit_mov_rax_imm64_to_reg_rdi(a, 1); // stdout
        a.bs(&[0x0F, 0x05]);
        emit_mov_rax_imm64(a, 1);
        emit_mov_rax_imm64_to_reg_rdi(a, 1);
        a.lea_rsi_rip("runtime::newline".to_string());
        emit_mov_rax_imm64_to_reg_rdx(a, 1);
        a.bs(&[0x0F, 0x05]);
    }

    fn emit_load_byte_index(a: &mut Asm) {
        // Stack order is data pointer, byte length, signed index.
        a.b(0x59); // pop rcx = index
        a.b(0x5A); // pop rdx = len
        a.b(0x58); // pop rax = data
        a.bs(&[0x48, 0x85, 0xC9]); // test rcx, rcx
        a.jcc(0x88, "runtime::bounds_fault"); // js
        a.bs(&[0x48, 0x39, 0xD1]); // cmp rcx, rdx
        a.jcc(0x83, "runtime::bounds_fault"); // jae
        a.bs(&[0x0F, 0xB6, 0x04, 0x08]); // movzx eax, byte [rax+rcx]
        a.b(0x50); // push selected byte as zero-extended i64
    }

    fn emit_mov_rax_imm64_to_reg_rdi(a: &mut Asm, value: i64) {
        a.bs(&[0x48, 0xBF]);
        a.i64(value);
    }

    fn emit_mov_rax_imm64_to_reg_rdx(a: &mut Asm, value: i64) {
        a.bs(&[0x48, 0xBA]);
        a.i64(value);
    }

    pub(super) fn emit_elf(module: &Module) -> Result<Vec<u8>, String> {
        let function_map: HashMap<&str, &lir::Function> = module
            .functions
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect();
        let entry = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .ok_or_else(|| "native module has no 'main' entry function".to_string())?;
        if !entry.params.is_empty() {
            return Err("native entry function must not require parameters".to_string());
        }
        if entry.rets.len() > 1 {
            return Err("native entry function may return at most one i64 value".to_string());
        }

        let mut strings: BTreeMap<String, usize> = BTreeMap::new();
        let mut bytes_pool: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
        for f in &module.functions {
            for instr in &f.body {
                match instr {
                    Instr::PrintStr(s) | Instr::StringRef(s) if !strings.contains_key(s) => {
                        let id = strings.len();
                        strings.insert(s.clone(), id);
                    }
                    Instr::BytesRef(bytes) if !bytes_pool.contains_key(bytes) => {
                        let id = bytes_pool.len();
                        bytes_pool.insert(bytes.clone(), id);
                    }
                    _ => {}
                }
            }
        }

        validate_module(module, &function_map)?;

        let mut a = Asm::default();
        a.label("_start")?;
        a.call(fn_label(&entry.name));
        if entry.rets.is_empty() {
            a.bs(&[0x31, 0xFF]); // xor edi, edi
        } else {
            a.bs(&[0x89, 0xC7]); // mov edi, eax
        }
        a.bs(&[0xB8, 0x3C, 0, 0, 0]); // mov eax, 60
        a.bs(&[0x0F, 0x05]); // syscall
        a.bs(&[0x0F, 0x0B]); // ud2

        a.label("runtime::arithmetic_fault")?;
        a.bs(&[0xBF]);
        a.u32(ARITHMETIC_FAULT_EXIT as u32);
        a.bs(&[0xB8, 0x3C, 0, 0, 0]);
        a.bs(&[0x0F, 0x05, 0x0F, 0x0B]);

        a.label("runtime::bounds_fault")?;
        a.bs(&[0xBF]);
        a.u32(BOUNDS_FAULT_EXIT as u32);
        a.bs(&[0xB8, 0x3C, 0, 0, 0]);
        a.bs(&[0x0F, 0x05, 0x0F, 0x0B]);

        emit_print_i64_runtime(&mut a)?;

        for f in &module.functions {
            emit_function(&mut a, f, &function_map, &strings, &bytes_pool)?;
        }

        a.label("runtime::newline")?;
        a.b(b'\n');
        for (s, id) in &strings {
            a.label(string_label(*id))?;
            a.bs(s.as_bytes());
        }
        for (bytes, id) in &bytes_pool {
            a.label(bytes_label(*id))?;
            a.bs(bytes);
        }

        let code = a.patch()?;
        let entry_offset = 0usize;
        write_elf_image(&code, entry_offset)
    }

    fn validate_module(
        module: &Module,
        function_map: &HashMap<&str, &lir::Function>,
    ) -> Result<(), String> {
        let mut names = HashSet::new();
        for f in &module.functions {
            if !names.insert(f.name.as_str()) {
                return Err(format!("duplicate LIR function name '{}'", f.name));
            }
            if f.params.len() > 6 {
                return Err(format!(
                    "function '{}' accepts at most 6 parameters in the v0.1.4.1 SysV scalar ABI",
                    f.name
                ));
            }
            if f.params
                .iter()
                .any(|ty| !matches!(ty, Type::I64 | Type::Ptr(_)))
            {
                return Err(format!(
                    "function '{}' uses a parameter outside the qualified i64/indirect-value ABI",
                    f.name
                ));
            }
            if f.rets.len() > 1 {
                return Err(format!(
                    "function '{}' has {} returns; owned native {} supports at most one",
                    f.name,
                    f.rets.len(),
                    OMNI_PROJECT_VERSION
                ));
            }
            if f.rets.iter().any(|ty| *ty != Type::I64) {
                return Err(format!(
                    "function '{}' uses a non-i64 return; owned native {} currently supports scalar i64 returns only",
                    f.name, OMNI_PROJECT_VERSION
                ));
            }
            if f.body.is_empty() && !f.rets.is_empty() {
                return Err(format!(
                    "function '{}' returns a value but has an empty LIR body",
                    f.name
                ));
            }
            for (ip, instr) in f.body.iter().enumerate() {
                match instr {
                    Instr::Jump(t) => {
                        if *t >= f.body.len() {
                            return Err(format!(
                                "{}: jump at {ip} targets invalid instruction {t}",
                                f.name
                            ));
                        }
                    }
                    Instr::CondJump { if_true, if_false } => {
                        if *if_true >= f.body.len() || *if_false >= f.body.len() {
                            return Err(format!(
                                "{}: conditional jump at {ip} has invalid target",
                                f.name
                            ));
                        }
                    }
                    Instr::Call(name) if name != "print" => {
                        if let Some(feature) = name.strip_prefix("__omni_unsupported_") {
                            return Err(format!(
                                "{}: feature '{}' is accepted by the front end but is not yet implemented by the {} owned native pipeline",
                                f.name, feature, OMNI_PROJECT_VERSION
                            ));
                        }
                        if !function_map.contains_key(name.as_str()) {
                            return Err(format!(
                                "{}: native backend cannot resolve call to '{name}'",
                                f.name
                            ));
                        }
                    }
                    Instr::StringRef(_)
                    | Instr::BytesRef(_)
                    | Instr::PrintBytes
                    | Instr::LoadByteIndex => {}
                    Instr::LoadOffset(base, offset) | Instr::StoreOffset(base, offset) => {
                        local_offset_disp(*base, *offset).map_err(|error| {
                            format!(
                                "{}: invalid aggregate access at instruction {ip}: {error}",
                                f.name
                            )
                        })?;
                    }
                    Instr::BoundsCheck(len) => {
                        i64::try_from(*len).map_err(|_| {
                            format!("{}: bounds length at instruction {ip} exceeds i64", f.name)
                        })?;
                    }
                    Instr::LoadIndex { base, len } => {
                        validate_local_index(*base, *len).map_err(|error| {
                            format!(
                                "{}: invalid bounds-checked aggregate access at instruction {ip}: {error}",
                                f.name
                            )
                        })?;
                    }
                    Instr::GetAddr(slot) => {
                        local_disp(*slot).map_err(|error| {
                            format!(
                                "{}: invalid local address at instruction {ip}: {error}",
                                f.name
                            )
                        })?;
                    }
                    Instr::LoadPtrOffset(slot, offset) | Instr::StorePtrOffset(slot, offset) => {
                        validate_ptr_param_offset(f, *slot, *offset).map_err(|error| {
                            format!(
                                "{}: invalid indirect value access at instruction {ip}: {error}",
                                f.name
                            )
                        })?;
                    }
                    Instr::AddOffset | Instr::LoadInd | Instr::StoreInd => {
                        return Err(format!(
                            "{}: instruction {ip} ({instr:?}) requires general raw-pointer semantics that remain unqualified",
                            f.name
                        ));
                    }
                    _ => {}
                }
            }
            analyze_eval_stack(f, function_map)?;
        }
        Ok(())
    }

    /// Verify the implicit LIR evaluation stack before emitting machine code.
    ///
    /// The emitter materializes incoming SysV parameters on this stack before
    /// the first LIR instruction. Every reachable control-flow merge must agree
    /// on stack depth. This turns malformed lowering into a compile-time error
    /// instead of a native stack-corruption bug.
    fn analyze_eval_stack(
        f: &lir::Function,
        function_map: &HashMap<&str, &lir::Function>,
    ) -> Result<Vec<Option<usize>>, String> {
        if f.body.is_empty() {
            return Ok(Vec::new());
        }

        let mut depth_at: Vec<Option<usize>> = vec![None; f.body.len()];
        let mut queue = VecDeque::new();
        depth_at[0] = Some(f.params.len());
        queue.push_back(0usize);

        while let Some(ip) = queue.pop_front() {
            let before = depth_at.get(ip).and_then(|depth| *depth).ok_or_else(|| {
                format!(
                    "{}: internal verifier queue lost stack depth at instruction {ip}",
                    f.name
                )
            })?;
            let instr = &f.body[ip];
            let (pops, pushes) = match instr {
                Instr::Const(_) | Instr::Load(_) | Instr::StringRef(_) | Instr::BytesRef(_) => {
                    (0usize, 1usize)
                }
                Instr::Add
                | Instr::Sub
                | Instr::Mul
                | Instr::Div
                | Instr::Mod
                | Instr::Lt
                | Instr::Gt
                | Instr::Le
                | Instr::Ge
                | Instr::Eq
                | Instr::Ne => (2, 1),
                Instr::Not => (1, 1),
                Instr::Store(_) => (1, 0),
                Instr::Call(name) if name == "print" => (1, 0),
                Instr::Call(name) => {
                    let callee = function_map
                        .get(name.as_str())
                        .ok_or_else(|| format!("{}: unknown native callee '{name}'", f.name))?;
                    (callee.params.len(), callee.rets.len())
                }
                Instr::Ret => (f.rets.len(), 0),
                Instr::CondJump { .. } => (1, 0),
                Instr::Jump(_) | Instr::Drop(_) | Instr::PrintStr(_) | Instr::Nop => (0, 0),
                Instr::PrintBytes => (2, 0),
                Instr::LoadByteIndex => (3, 1),
                Instr::LoadOffset(_, _) => (0, 1),
                Instr::StoreOffset(_, _) => (1, 0),
                Instr::BoundsCheck(_) | Instr::LoadIndex { .. } => (1, 1),
                Instr::GetAddr(_) | Instr::LoadPtrOffset(_, _) => (0, 1),
                Instr::StorePtrOffset(_, _) => (1, 0),
                Instr::AddOffset | Instr::LoadInd | Instr::StoreInd => {
                    unreachable!(
                        "unsupported raw-pointer memory operations rejected before stack validation"
                    )
                }
            };

            if before < pops {
                return Err(format!(
                    "{}: LIR evaluation-stack underflow at instruction {ip} ({instr:?}); depth {before}, needs {pops}",
                    f.name
                ));
            }
            let after = before - pops + pushes;

            if matches!(instr, Instr::Ret) {
                if after != 0 {
                    return Err(format!(
                        "{}: return at instruction {ip} leaves {after} stale value(s) on the LIR evaluation stack",
                        f.name
                    ));
                }
                continue;
            }

            let mut successors = [None, None];
            match instr {
                Instr::Jump(target) => successors[0] = Some(*target),
                Instr::CondJump { if_true, if_false } => {
                    successors[0] = Some(*if_true);
                    successors[1] = Some(*if_false);
                }
                _ if ip + 1 < f.body.len() => successors[0] = Some(ip + 1),
                _ => {
                    if !f.rets.is_empty() {
                        return Err(format!(
                            "{}: reachable control flow falls off the end without returning a value",
                            f.name
                        ));
                    }
                    if after != 0 {
                        return Err(format!(
                            "{}: reachable fallthrough leaves {after} value(s) on the LIR evaluation stack",
                            f.name
                        ));
                    }
                }
            }

            for succ in successors.into_iter().flatten() {
                match depth_at[succ] {
                    None => {
                        depth_at[succ] = Some(after);
                        queue.push_back(succ);
                    }
                    Some(existing) if existing == after => {}
                    Some(existing) => {
                        return Err(format!(
                            "{}: control-flow merge at instruction {succ} has inconsistent LIR stack depths {existing} and {after}",
                            f.name
                        ));
                    }
                }
            }
        }

        Ok(depth_at)
    }

    fn emit_stack_align_for_call(a: &mut Asm, live_eval_values: usize) -> bool {
        // The prologue keeps the frame base 16-byte aligned before the LIR
        // evaluation stack is used. Each live evaluation value occupies one
        // eight-byte push. SysV requires 16-byte alignment at a call site.
        if live_eval_values % 2 == 1 {
            a.bs(&[0x48, 0x83, 0xEC, 0x08]); // sub rsp, 8
            true
        } else {
            false
        }
    }

    fn emit_stack_unalign_after_call(a: &mut Asm, adjusted: bool) {
        if adjusted {
            a.bs(&[0x48, 0x83, 0xC4, 0x08]); // add rsp, 8
        }
    }

    fn emit_function(
        a: &mut Asm,
        f: &lir::Function,
        function_map: &HashMap<&str, &lir::Function>,
        strings: &BTreeMap<String, usize>,
        bytes_pool: &BTreeMap<Vec<u8>, usize>,
    ) -> Result<(), String> {
        let stack_depths = analyze_eval_stack(f, function_map)?;
        a.label(fn_label(&f.name))?;
        a.b(0x55); // push rbp
        a.bs(&[0x48, 0x89, 0xE5]); // mov rbp, rsp
        let frame = aligned_frame_bytes(f);
        if frame > 0 {
            if frame <= i32::MAX as usize {
                a.bs(&[0x48, 0x81, 0xEC]); // sub rsp, imm32
                a.u32(frame as u32);
            } else {
                return Err(format!(
                    "function '{}' requires an excessively large frame",
                    f.name
                ));
            }
        }
        emit_push_param_regs(a, f.params.len())?;

        for (ip, instr) in f.body.iter().enumerate() {
            a.label(ip_label(&f.name, ip))?;
            match instr {
                Instr::Const(v) => {
                    emit_mov_rax_imm64(a, *v);
                    a.b(0x50);
                }
                Instr::Add => {
                    a.b(0x5B);
                    a.b(0x58);
                    a.bs(&[0x48, 0x01, 0xD8]);
                    a.jcc(0x80, "runtime::arithmetic_fault"); // jo
                    a.b(0x50);
                }
                Instr::Sub => {
                    a.b(0x5B);
                    a.b(0x58);
                    a.bs(&[0x48, 0x29, 0xD8]);
                    a.jcc(0x80, "runtime::arithmetic_fault");
                    a.b(0x50);
                }
                Instr::Mul => {
                    a.b(0x5B);
                    a.b(0x58);
                    a.bs(&[0x48, 0x0F, 0xAF, 0xC3]); // imul rax, rbx
                    a.jcc(0x80, "runtime::arithmetic_fault");
                    a.b(0x50);
                }
                Instr::Div | Instr::Mod => {
                    a.b(0x5B); // rhs
                    a.bs(&[0x48, 0x85, 0xDB]); // test rbx, rbx
                    a.jcc(0x84, "runtime::arithmetic_fault"); // je
                    a.b(0x58); // lhs
                               // Detect i64::MIN / -1 before idiv.
                    a.bs(&[0x48, 0xB9]);
                    a.i64(i64::MIN);
                    a.bs(&[0x48, 0x39, 0xC8]); // cmp rax, rcx
                    let safe_label = format!("{}::divsafe::{ip}", fn_label(&f.name));
                    a.jcc(0x85, safe_label.clone()); // jne
                    a.bs(&[0x48, 0x83, 0xFB, 0xFF]); // cmp rbx, -1
                    a.jcc(0x84, "runtime::arithmetic_fault"); // je
                    a.label(safe_label)?;
                    a.bs(&[0x48, 0x99]); // cqo
                    a.bs(&[0x48, 0xF7, 0xFB]); // idiv rbx
                    if matches!(instr, Instr::Div) {
                        a.b(0x50); // push rax
                    } else {
                        a.b(0x52); // push rdx
                    }
                }
                Instr::Lt => emit_cmp_result(a, 0x9C), // setl
                Instr::Gt => emit_cmp_result(a, 0x9F), // setg
                Instr::Le => emit_cmp_result(a, 0x9E), // setle
                Instr::Ge => emit_cmp_result(a, 0x9D), // setge
                Instr::Eq => emit_cmp_result(a, 0x94), // sete
                Instr::Ne => emit_cmp_result(a, 0x95), // setne
                Instr::Not => {
                    a.b(0x58);
                    a.bs(&[0x48, 0x85, 0xC0]);
                    a.bs(&[0x0F, 0x94, 0xC0]);
                    a.bs(&[0x48, 0x0F, 0xB6, 0xC0]);
                    a.b(0x50);
                }
                Instr::Load(slot) => emit_load_local(a, *slot)?,
                Instr::Store(slot) => emit_store_local(a, *slot)?,
                Instr::Call(name) if name == "print" => {
                    let before = stack_depths[ip].ok_or_else(|| {
                        format!("internal error: missing stack depth for {}:{ip}", f.name)
                    })?;
                    a.b(0x5F); // pop rdi
                    let adjusted = emit_stack_align_for_call(a, before - 1);
                    a.call("runtime::print_i64");
                    emit_stack_unalign_after_call(a, adjusted);
                }
                Instr::Call(name) => {
                    let callee = *function_map
                        .get(name.as_str())
                        .ok_or_else(|| format!("unknown native callee '{name}'"))?;
                    let before = stack_depths[ip].ok_or_else(|| {
                        format!("internal error: missing stack depth for {}:{ip}", f.name)
                    })?;
                    emit_pop_call_args(a, callee.params.len())?;
                    let live_values = before - callee.params.len();
                    let adjusted = emit_stack_align_for_call(a, live_values);
                    a.call(fn_label(name));
                    emit_stack_unalign_after_call(a, adjusted);
                    if !callee.rets.is_empty() {
                        a.b(0x50);
                    }
                }
                Instr::Ret => {
                    if f.rets.is_empty() {
                        a.bs(&[0x31, 0xC0]);
                    } else {
                        a.b(0x58);
                    }
                    a.bs(&[0xC9, 0xC3]); // leave; ret
                }
                Instr::Jump(target) => a.jmp(ip_label(&f.name, *target)),
                Instr::CondJump { if_true, if_false } => {
                    a.b(0x58);
                    a.bs(&[0x48, 0x85, 0xC0]);
                    a.jcc(0x85, ip_label(&f.name, *if_true)); // jne
                    a.jmp(ip_label(&f.name, *if_false));
                }
                Instr::Drop(slot) => emit_zero_local(a, *slot)?,
                Instr::PrintStr(s) => {
                    let id = strings
                        .get(s)
                        .ok_or_else(|| "internal string-pool mismatch".to_string())?;
                    emit_print_str(a, &string_label(*id), s.len())?;
                }
                Instr::StringRef(s) => {
                    let id = strings
                        .get(s)
                        .ok_or_else(|| "internal string-pool mismatch".to_string())?;
                    a.lea_rax_rip(string_label(*id));
                    a.b(0x50);
                }
                Instr::BytesRef(bytes) => {
                    let id = bytes_pool
                        .get(bytes)
                        .ok_or_else(|| "internal bytes-pool mismatch".to_string())?;
                    a.lea_rax_rip(bytes_label(*id));
                    a.b(0x50);
                }
                Instr::PrintBytes => emit_print_bytes_from_stack(a),
                Instr::LoadByteIndex => emit_load_byte_index(a),
                Instr::Nop => {}
                Instr::LoadOffset(base, offset) => emit_load_local_offset(a, *base, *offset)?,
                Instr::StoreOffset(base, offset) => emit_store_local_offset(a, *base, *offset)?,
                Instr::BoundsCheck(len) => {
                    a.b(0x58); // pop rax (signed index)
                    a.bs(&[0x48, 0x85, 0xC0]); // test rax, rax
                    a.jcc(0x88, "runtime::bounds_fault"); // js: negative index
                    a.bs(&[0x48, 0xB9]); // mov rcx, imm64
                    a.i64(
                        i64::try_from(*len).map_err(|_| "bounds length exceeds i64".to_string())?,
                    );
                    a.bs(&[0x48, 0x39, 0xC8]); // cmp rax, rcx
                    a.jcc(0x83, "runtime::bounds_fault"); // jae
                    a.b(0x50); // preserve validated index
                }
                Instr::LoadIndex { base, len } => emit_load_local_index(a, *base, *len)?,
                Instr::GetAddr(slot) => emit_get_local_addr(a, *slot)?,
                Instr::LoadPtrOffset(slot, offset) => emit_load_ptr_offset(a, f, *slot, *offset)?,
                Instr::StorePtrOffset(slot, offset) => emit_store_ptr_offset(a, f, *slot, *offset)?,
                Instr::AddOffset | Instr::LoadInd | Instr::StoreInd => {
                    unreachable!("validated before emission")
                }
            }
        }

        if !matches!(f.body.last(), Some(Instr::Ret)) {
            a.bs(&[0x31, 0xC0, 0xC9, 0xC3]);
        }
        Ok(())
    }

    fn emit_print_i64_runtime(a: &mut Asm) -> Result<(), String> {
        a.label("runtime::print_i64")?;
        // Hand-written leaf routine. Input: rdi = signed i64. It converts to
        // decimal into a 32-byte stack buffer and performs write(1, ..., n),
        // then writes a newline. No libc and no external runtime are involved.
        a.b(0x55); // push rbp
        a.bs(&[0x48, 0x89, 0xE5]);
        a.bs(&[0x48, 0x83, 0xEC, 0x40]); // sub rsp, 64
        a.bs(&[0x48, 0x89, 0xF8]); // mov rax, rdi
        a.bs(&[0x48, 0x8D, 0x75, 0xFF]); // lea rsi, [rbp-1]
        a.bs(&[0xC6, 0x06, 0x0A]); // mov byte [rsi], '\n'
        a.bs(&[0x48, 0xC7, 0xC1, 0x01, 0, 0, 0]); // mov rcx, 1 (length)
        a.bs(&[0x45, 0x31, 0xC0]); // xor r8d, r8d (negative flag)
        a.bs(&[0x48, 0x85, 0xC0]); // test rax,rax
        let nonneg = "runtime::print_i64::nonneg";
        a.jcc(0x89, nonneg); // jns
        a.bs(&[0x41, 0xB0, 0x01]); // mov r8b,1
                                   // Negate as unsigned two's complement; works for i64::MIN.
        a.bs(&[0x48, 0xF7, 0xD8]); // neg rax
        a.label(nonneg)?;
        let loop_label = "runtime::print_i64::digits";
        a.label(loop_label)?;
        a.bs(&[0x31, 0xD2]); // xor edx,edx
        a.bs(&[0x41, 0xB9, 0x0A, 0, 0, 0]); // mov r9d,10
        a.bs(&[0x49, 0xF7, 0xF1]); // div r9
        a.bs(&[0x80, 0xC2, 0x30]); // add dl,'0'
        a.bs(&[0x48, 0xFF, 0xCE]); // dec rsi
        a.bs(&[0x88, 0x16]); // mov [rsi],dl
        a.bs(&[0x48, 0xFF, 0xC1]); // inc rcx
        a.bs(&[0x48, 0x85, 0xC0]); // test rax,rax
        a.jcc(0x85, loop_label); // jne
        a.bs(&[0x45, 0x84, 0xC0]); // test r8b,r8b
        let no_sign = "runtime::print_i64::nosign";
        a.jcc(0x84, no_sign); // je
        a.bs(&[0x48, 0xFF, 0xCE]); // dec rsi
        a.bs(&[0xC6, 0x06, 0x2D]); // '-'
        a.bs(&[0x48, 0xFF, 0xC1]); // inc rcx
        a.label(no_sign)?;
        // write(1, rsi, rcx)
        a.bs(&[0x48, 0x89, 0xCA]); // mov rdx,rcx
        a.bs(&[0xBF, 0x01, 0, 0, 0]); // mov edi,1
        a.bs(&[0xB8, 0x01, 0, 0, 0]); // mov eax,1
        a.bs(&[0x0F, 0x05]);
        a.bs(&[0xC9, 0xC3]);
        Ok(())
    }

    fn write_elf_image(code: &[u8], entry_code_offset: usize) -> Result<Vec<u8>, String> {
        let entry_file_offset = CODE_OFFSET
            .checked_add(entry_code_offset)
            .ok_or_else(|| "ELF entry offset overflow".to_string())?;
        let entry_vaddr = ELF_BASE
            .checked_add(entry_file_offset as u64)
            .ok_or_else(|| "ELF entry address overflow".to_string())?;
        let file_size = CODE_OFFSET
            .checked_add(code.len())
            .ok_or_else(|| "ELF file size overflow".to_string())?;

        let mut out = vec![0u8; CODE_OFFSET];
        // e_ident
        out[0..4].copy_from_slice(b"\x7FELF");
        out[4] = 2; // ELFCLASS64
        out[5] = 1; // little endian
        out[6] = 1; // version
        out[7] = 0; // SYSV ABI

        put_u16(&mut out, 16, 2); // ET_EXEC
        put_u16(&mut out, 18, 62); // EM_X86_64
        put_u32(&mut out, 20, 1);
        put_u64(&mut out, 24, entry_vaddr);
        put_u64(&mut out, 32, ELF_HEADER_SIZE as u64);
        put_u64(&mut out, 40, 0); // no section table
        put_u32(&mut out, 48, 0);
        put_u16(&mut out, 52, ELF_HEADER_SIZE as u16);
        put_u16(&mut out, 54, PROGRAM_HEADER_SIZE as u16);
        put_u16(&mut out, 56, 1);
        put_u16(&mut out, 58, 0);
        put_u16(&mut out, 60, 0);
        put_u16(&mut out, 62, 0);

        // Single read+execute PT_LOAD segment containing headers, code and literals.
        let ph = ELF_HEADER_SIZE;
        put_u32(&mut out, ph, 1); // PT_LOAD
        put_u32(&mut out, ph + 4, 5); // PF_R | PF_X
        put_u64(&mut out, ph + 8, 0);
        put_u64(&mut out, ph + 16, ELF_BASE);
        put_u64(&mut out, ph + 24, ELF_BASE);
        put_u64(&mut out, ph + 32, file_size as u64);
        put_u64(&mut out, ph + 40, file_size as u64);
        put_u64(&mut out, ph + 48, 0x1000);

        out.extend_from_slice(code);
        Ok(out)
    }

    fn put_u16(out: &mut [u8], off: usize, v: u16) {
        out[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }

    fn put_u32(out: &mut [u8], off: usize, v: u32) {
        out[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn put_u64(out: &mut [u8], off: usize, v: u64) {
        out[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }
}
