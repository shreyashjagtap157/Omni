//! Simple mock Polonius engine for Stage0 integration tests.
//!
//! This crate accepts the textual facts produced by `export_polonius_input`
//! and implements a small in-process solver that detects the same simple
//! borrow errors as the existing local checker. It's intentionally small and
//! designed to be replaced by a real `polonius-engine` integration later.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VarState {
    Init,
    Moved,
}

#[derive(Debug)]
enum Instr {
    ConstInt { dest: String },
    ConstStr { dest: String },
    Move { dest: String, src: String },
    Print { src: String },
    Drop { var: String },
}

#[derive(Debug)]
struct Block {
    id: usize,
    instrs: Vec<Instr>,
}

#[derive(Debug)]
struct Func {
    name: String,
    blocks: Vec<Block>,
}

pub fn solve(facts: &str) -> Result<String, String> {
    // Parse the simple textual facts format produced by `export_polonius_input`.
    let mut funcs: Vec<Func> = Vec::new();
    // Map of live facts: (func, block, instr) -> set of variable/path names
    let mut live_map: std::collections::HashMap<
        (String, usize, usize),
        std::collections::HashSet<String>,
    > = std::collections::HashMap::new();
    let mut cur_func: Option<Func> = None;
    let mut cur_block: Option<Block> = None;

    for raw in facts.lines() {
        let line = raw.trim_start();
        if line.is_empty() {
            continue;
        }
        // collect region/loan facts like `live <func> <block> <instr> <var>`
        if let Some(rest) = line.strip_prefix("live ") {
            let mut parts = rest.split_whitespace();
            if let (Some(f), Some(b), Some(i), Some(var)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            {
                if let (Ok(bid), Ok(ii)) = (b.parse::<usize>(), i.parse::<usize>()) {
                    live_map
                        .entry((f.to_string(), bid, ii))
                        .or_default()
                        .insert(var.to_string());
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("function ") {
            // start new function
            if let Some(f) = cur_func.take() {
                // flush last block
                if let Some(b) = cur_block.take() {
                    let mut f2 = f;
                    f2.blocks.push(b);
                    funcs.push(f2);
                } else {
                    funcs.push(f);
                }
            }
            cur_func = Some(Func {
                name: rest.trim().to_string(),
                blocks: Vec::new(),
            });
            cur_block = None;
            continue;
        }
        if let Some(rest) = line.strip_prefix("block ") {
            // start new block
            // flush previous block into current func
            if let Some(b) = cur_block.take() {
                if let Some(f) = cur_func.as_mut() {
                    f.blocks.push(b);
                }
            }
            let id = rest.trim().parse::<usize>().unwrap_or(0);
            cur_block = Some(Block {
                id,
                instrs: Vec::new(),
            });
            continue;
        }

        // instruction line: "<left>: <rest>"
        if let Some(colon) = line.find(':') {
            let _left = line[..colon].trim();
            let rest = line[colon + 1..].trim();
            if rest.starts_with("const_int ") {
                let dest = _left.to_string();
                if let Some(b) = cur_block.as_mut() {
                    b.instrs.push(Instr::ConstInt { dest });
                }
            } else if rest.starts_with("const_str ") {
                let dest = _left.to_string();
                if let Some(b) = cur_block.as_mut() {
                    b.instrs.push(Instr::ConstStr { dest });
                }
            } else if let Some(src) = rest.strip_prefix("move ") {
                let src = src.trim().to_string();
                let dest = _left.to_string();
                if let Some(b) = cur_block.as_mut() {
                    b.instrs.push(Instr::Move { dest, src });
                }
            } else if let Some(src) = rest.strip_prefix("print ") {
                let src = src.trim().to_string();
                if let Some(b) = cur_block.as_mut() {
                    b.instrs.push(Instr::Print { src });
                }
            } else if let Some(var) = rest.strip_prefix("drop ") {
                let var = var.trim().to_string();
                if let Some(b) = cur_block.as_mut() {
                    b.instrs.push(Instr::Drop { var });
                }
            } else {
                // unknown instruction - ignore
            }
        }
    }

    // flush last blocks and function
    if let Some(b) = cur_block.take() {
        if let Some(f) = cur_func.as_mut() {
            f.blocks.push(b);
        }
    }
    if let Some(f) = cur_func.take() {
        funcs.push(f);
    }

    // Now run the same lightweight checker over the parsed structure.
    for f in &funcs {
        let mut state: std::collections::HashMap<String, VarState> =
            std::collections::HashMap::new();
        for b in &f.blocks {
            for (i, instr) in b.instrs.iter().enumerate() {
                match instr {
                    Instr::ConstInt { dest } | Instr::ConstStr { dest } => {
                        state.insert(dest.clone(), VarState::Init);
                    }
                    Instr::Move { dest, src } => {
                        let (src_base, src_field) = split_var(src);

                        let get_state = |state: &std::collections::HashMap<String, VarState>,
                                         name: &str|
                         -> Option<VarState> {
                            if let Some(s) = state.get(name) {
                                return Some(*s);
                            }
                            if let Some(pos) = name.find('.') {
                                let base = &name[..pos];
                                if let Some(s2) = state.get(base) {
                                    return Some(*s2);
                                }
                            }
                            None
                        };

                        let src_key = src.clone();
                        match get_state(&state, &src_key) {
                            Some(VarState::Init) => {
                                state.insert(dest.clone(), VarState::Init);
                                if src_field.is_none() {
                                    let base = src_base.clone();
                                    state.insert(base.clone(), VarState::Moved);
                                    // If we previously tracked dotted child paths
                                    // (e.g., `x.a`) mark them moved as well so
                                    // tests that initialize fields then move the
                                    // base behave as expected.
                                    let keys: Vec<String> = state.keys().cloned().collect();
                                    for k in keys {
                                        if k.starts_with(&(base.clone() + ".")) {
                                            state.insert(k, VarState::Moved);
                                        }
                                    }
                                } else {
                                    state.insert(src_key.clone(), VarState::Moved);
                                }
                            }
                            Some(VarState::Moved) => {
                                return Err(format!("polonius-mock: use-after-move in func '{}' block {} instr {}: {}", f.name, b.id, i, src));
                            }
                            None => {
                                return Err(format!("polonius-mock: use of uninitialized variable '{}' in func '{}' block {} instr {}", src, f.name, b.id, i));
                            }
                        }
                    }
                    Instr::Print { src } => {
                        let (base, _field) = split_var(src);
                        let state_opt = if let Some(s) = state.get(src) {
                            Some(*s)
                        } else {
                            state.get(&base).cloned()
                        };
                        match state_opt {
                            Some(VarState::Init) => {}
                            Some(VarState::Moved) => {
                                // Only flag use-after-move if the exporter marked this
                                // variable as live at this point.
                                let key = (f.name.clone(), b.id, i);
                                let is_live = live_map
                                    .get(&key)
                                    .map(|s| s.contains(src) || s.contains(&base))
                                    .unwrap_or(false);
                                if is_live {
                                    return Err(format!("polonius-mock: use-after-move (print) '{}' in func '{}' block {} instr {}", src, f.name, b.id, i));
                                }
                            }
                            None => {
                                return Err(format!("polonius-mock: use of uninitialized variable '{}' (print) in func '{}' block {} instr {}", src, f.name, b.id, i));
                            }
                        }
                    }
                    Instr::Drop { var } => {
                        let key = var.clone();
                        let (base, field) = split_var(var);
                        let state_opt = if let Some(s) = state.get(&key) {
                            Some(*s)
                        } else {
                            state.get(&base).cloned()
                        };
                        match state_opt {
                            Some(VarState::Init) => {
                                if field.is_none() {
                                    // Mark only the base as moved.
                                    state.insert(base.clone(), VarState::Moved);
                                } else {
                                    state.insert(key.clone(), VarState::Moved);
                                }
                            }
                            Some(VarState::Moved) => {
                                // Double-drop is an error only when the var is live here.
                                let keyk = (f.name.clone(), b.id, i);
                                let is_live = live_map
                                    .get(&keyk)
                                    .map(|s| s.contains(&key) || s.contains(&base))
                                    .unwrap_or(false);
                                if is_live {
                                    return Err(format!("polonius-mock: double-drop '{}' in func '{}' block {} instr {}", var, f.name, b.id, i));
                                }
                            }
                            None => {
                                return Err(format!("polonius-mock: drop of uninitialized '{}' in func '{}' block {} instr {}", var, f.name, b.id, i));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok("ok".to_string())
}

fn split_var(name: &str) -> (String, Option<String>) {
    if let Some(pos) = name.find('.') {
        let base = name[..pos].to_string();
        let field = name[pos + 1..].to_string();
        (base, Some(field))
    } else {
        (name.to_string(), None)
    }
}
