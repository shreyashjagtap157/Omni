//! Adapter crate that exposes a stable `check_facts` API.
//!
//! It can either delegate to the in-repo `polonius_engine_mock` or invoke an
//! external `polonius` CLI when `OMNI_USE_POLONIUS=1`.

use std::process::Command;

// Try to use the polonius engine library if the crate was compiled with the
// `use_polonius_lib` feature. This function is a no-op (returns `None`) when
// the feature is not enabled so the code can fall back to the CLI path.
#[cfg(not(feature = "use_polonius_lib"))]
fn try_polonius_engine(_facts: &str) -> Option<Result<(), String>> {
    None
}

#[cfg(feature = "use_polonius_lib")]
fn try_polonius_engine(facts: &str) -> Option<Result<(), String>> {
    // When the feature is enabled we attempt to translate our textual
    // exporter format into per-function `AllFacts` and ask the library to
    // compute results. If anything goes wrong we return `Some(Err(_))` so the
    // caller can report a useful error; returning `None` means "library not
    // available / couldn't run" and the caller will fall back to the CLI.
    use polonius_engine::{AllFacts, Output, Algorithm};

    // Minimal, conservative parser: group lines by `function <name>` header
    // and pass each function's facts to the engine separately.
    let mut groups: std::collections::HashMap<String, Vec<&str>> = std::collections::HashMap::new();
    let mut current: Option<String> = None;
    for line in facts.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if let Some(rest) = line.strip_prefix("function ") {
            current = Some(rest.to_string());
            groups.entry(current.as_ref().unwrap().clone()).or_default();
            continue;
        }
        if let Some(name) = &current {
            groups.get_mut(name).unwrap().push(line);
        }
    }

    for (func, lines) in groups.into_iter() {
        // Define a tiny Atom wrapper and FactTypes implementation so we can
        // instantiate `AllFacts<T>` with a simple in-crate type. The
        // polonius_engine crate requires types implementing `FactTypes` and
        // `Atom`; implementing them here lets us avoid depending on rustc
        // internals while still using the engine's API.
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        struct AtomId(usize);

        impl From<usize> for AtomId {
            fn from(u: usize) -> AtomId { AtomId(u) }
        }
        impl Into<usize> for AtomId {
            fn into(self) -> usize { self.0 }
        }

        impl polonius_engine::Atom for AtomId {
            fn index(self) -> usize { self.0 }
        }

        #[derive(Copy, Clone, Debug)]
        struct SimpleFacts;

        impl polonius_engine::FactTypes for SimpleFacts {
            type Origin = AtomId;
            type Loan = AtomId;
            type Point = AtomId;
            type Variable = AtomId;
            type Path = AtomId;
        }

        // Build a conservative AllFacts<SimpleFacts> instance and populate a
        // small set of relations that are sufficient for our parity tests.
        let mut all: AllFacts<SimpleFacts> = AllFacts::default();

        use std::collections::HashMap;
        // Map (block, instr) -> point id
        let mut point_map: HashMap<(String, usize), usize> = HashMap::new();
        let mut next_point: usize = 0;
        // Map variable name -> local id
        let mut local_map: HashMap<String, usize> = HashMap::new();
        let mut next_local: usize = 0;

        for l in &lines {
            if let Some(rest) = l.strip_prefix("point ") {
                let mut parts = rest.split_whitespace();
                let _f = parts.next();
                if let (Some(b), Some(i)) = (parts.next(), parts.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        point_map.insert((b.to_string(), i), next_point);
                        next_point += 1;
                    }
                }
            }
        }

        // Simple CFG: link consecutive points within the same function in
        // the order we discovered them. This gives a usable control-flow
        // graph for tiny functions used in tests. Also build a reverse map
        // from point id -> (block, instr) to translate diagnostics later.
        let mut points_by_block: HashMap<String, Vec<usize>> = HashMap::new();
        let mut point_rev: HashMap<usize, (String, usize)> = HashMap::new();
        for ((block, instr), &pt) in point_map.iter() {
            points_by_block.entry(block.clone()).or_default().push(pt);
            point_rev.insert(pt, (block.clone(), *instr));
        }
        for (_block, mut pts) in points_by_block.into_iter() {
            pts.sort_unstable();
            for w in pts.windows(2) {
                if let [a, b] = w {
                    all.cfg_edge.push((AtomId(*a), AtomId(*b)));
                }
            }
        }

        // Path bookkeeping: map string paths (e.g., "x", "x.a") to Path ids
        let mut path_map: HashMap<String, usize> = HashMap::new();
        let mut next_path: usize = 0;
        let mut child_pairs: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
        let mut path_is_var_set: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();

        // Helper to ensure a path and its parent chain exist and emit child_path/path_is_var
        fn ensure_path_fn(
            p: &str,
            path_map: &mut HashMap<String, usize>,
            next_path: &mut usize,
            child_pairs: &mut std::collections::HashSet<(usize, usize)>,
            path_is_var_set: &mut std::collections::HashSet<(usize, usize)>,
            local_map: &mut HashMap<String, usize>,
            next_local: &mut usize,
            all: &mut AllFacts<SimpleFacts>,
        ) -> usize {
            if let Some(&id) = path_map.get(p) {
                return id;
            }
            let parts: Vec<&str> = p.split('.').collect();
            let mut accum = String::new();
            let mut prev_id: Option<usize> = None;
            for (i, part) in parts.iter().enumerate() {
                if i == 0 {
                    accum = part.to_string();
                } else {
                    accum.push_str(".");
                    accum.push_str(part);
                }
                if let Some(&existing) = path_map.get(&accum) {
                    prev_id = Some(existing);
                    continue;
                }
                let id = *next_path;
                *next_path += 1;
                path_map.insert(accum.clone(), id);

                // root path corresponds to a variable
                if i == 0 {
                    let lid = *local_map.entry(accum.clone()).or_insert_with(|| {
                        let v = *next_local; *next_local += 1; v
                    });
                    if path_is_var_set.insert((id, lid)) {
                        all.path_is_var.push((AtomId(id), AtomId(lid)));
                    }
                }

                if let Some(parent) = prev_id {
                    if child_pairs.insert((id, parent)) {
                        all.child_path.push((AtomId(id), AtomId(parent)));
                    }
                }
                prev_id = Some(id);
            }
            *path_map.get(p).unwrap()
        }

        // Walk lines to populate relations.
        for l in &lines {
            if let Some(rest) = l.strip_prefix("def ") {
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(var)) = (p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&pt) = point_map.get(&(b.to_string(), i)) {
                            if var.contains('.') {
                                let pid = ensure_path_fn(var, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                                all.path_assigned_at_base.push((AtomId(pid), AtomId(pt)));
                            } else {
                                let lid = *local_map.entry(var.to_string()).or_insert_with(|| {
                                    let v = next_local; next_local += 1; v
                                });
                                all.var_defined_at.push((AtomId(lid), AtomId(pt)));
                            }
                        }
                    }
                }
            } else if let Some(rest) = l.strip_prefix("use ") {
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(var)) = (p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&pt) = point_map.get(&(b.to_string(), i)) {
                            if var.contains('.') {
                                let pid = ensure_path_fn(var, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                                all.path_accessed_at_base.push((AtomId(pid), AtomId(pt)));
                                // also mark the root variable as used
                                let root = var.split('.').next().unwrap();
                                let lid = *local_map.entry(root.to_string()).or_insert_with(|| {
                                    let v = next_local; next_local += 1; v
                                });
                                all.var_used_at.push((AtomId(lid), AtomId(pt)));
                            } else {
                                let lid = *local_map.entry(var.to_string()).or_insert_with(|| {
                                    let v = next_local; next_local += 1; v
                                });
                                all.var_used_at.push((AtomId(lid), AtomId(pt)));
                            }
                        }
                    }
                }
            } else if let Some(rest) = l.strip_prefix("drop ") {
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(var)) = (p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&pt) = point_map.get(&(b.to_string(), i)) {
                            // Treat drop of a path as a var_dropped_at for its root
                            let root = var.split('.').next().unwrap();
                            let lid = *local_map.entry(root.to_string()).or_insert_with(|| {
                                let v = next_local; next_local += 1; v
                            });
                            all.var_dropped_at.push((AtomId(lid), AtomId(pt)));
                        }
                    }
                }
            } else if let Some(rest) = l.strip_prefix("move ") {
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(src), Some(dest)) = (p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&pt) = point_map.get(&(b.to_string(), i)) {
                            // source move: mark path_moved_at_base for the source path/root
                            if src.contains('.') {
                                let pid = ensure_path_fn(src, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                                all.path_moved_at_base.push((AtomId(pid), AtomId(pt)));
                            } else {
                                let pid = ensure_path_fn(src, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                                all.path_moved_at_base.push((AtomId(pid), AtomId(pt)));
                            }
                            // destination becomes defined
                            if dest.contains('.') {
                                let pd = ensure_path_fn(dest, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                                all.path_assigned_at_base.push((AtomId(pd), AtomId(pt)));
                            } else {
                                let lid = *local_map.entry(dest.to_string()).or_insert_with(|| {
                                    let v = next_local; next_local += 1; v
                                });
                                all.var_defined_at.push((AtomId(lid), AtomId(pt)));
                            }
                        }
                    }
                }
            } else if let Some(rest) = l.strip_prefix("linear_move ") {
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(src), Some(dest)) = (p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&pt) = point_map.get(&(b.to_string(), i)) {
                            let pid = ensure_path_fn(src, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                            all.path_moved_at_base.push((AtomId(pid), AtomId(pt)));
                            if dest.contains('.') {
                                let pd = ensure_path_fn(dest, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                                all.path_assigned_at_base.push((AtomId(pd), AtomId(pt)));
                            } else {
                                let lid = *local_map.entry(dest.to_string()).or_insert_with(|| {
                                    let v = next_local; next_local += 1; v
                                });
                                all.var_defined_at.push((AtomId(lid), AtomId(pt)));
                            }
                        }
                    }
                }
            } else if let Some(rest) = l.strip_prefix("jump ") {
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(target)) = (p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&src) = point_map.get(&(b.to_string(), i)) {
                            if let Some(&tgt) = point_map.get(&(target.to_string(), 0)) {
                                all.cfg_edge.push((AtomId(src), AtomId(tgt)));
                            }
                        }
                    }
                }
            } else if let Some(rest) = l.strip_prefix("jump_if ") {
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(_cond), Some(target)) = (p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&src) = point_map.get(&(b.to_string(), i)) {
                            if let Some(&tgt) = point_map.get(&(target.to_string(), 0)) {
                                all.cfg_edge.push((AtomId(src), AtomId(tgt)));
                            }
                        }
                    }
                }
            } else if let Some(rest) = l.strip_prefix("field ") {
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(path)) = (p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&pt) = point_map.get(&(b.to_string(), i)) {
                            let pid = ensure_path_fn(path, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                            all.path_accessed_at_base.push((AtomId(pid), AtomId(pt)));
                        }
                    }
                }
            } else if let Some(rest) = l.strip_prefix("struct_field ") {
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(path)) = (p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&pt) = point_map.get(&(b.to_string(), i)) {
                            let pid = ensure_path_fn(path, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                            all.path_accessed_at_base.push((AtomId(pid), AtomId(pt)));
                        }
                    }
                }
            } else if let Some(rest) = l.strip_prefix("index ") {
                // index <func> <block> <instr> <base[index]>
                let mut p = rest.split_whitespace();
                let _f = p.next();
                if let (Some(b), Some(i), Some(expr)) = (p.next(), p.next(), p.next()) {
                    if let Ok(i) = i.parse::<usize>() {
                        if let Some(&pt) = point_map.get(&(b.to_string(), i)) {
                            // strip any indexing to get base path like `x[0]` -> `x`
                            let base = if let Some(pos) = expr.find('[') { &expr[..pos] } else { expr };
                            let pid = ensure_path_fn(base, &mut path_map, &mut next_path, &mut child_pairs, &mut path_is_var_set, &mut local_map, &mut next_local, &mut all);
                            all.path_accessed_at_base.push((AtomId(pid), AtomId(pt)));
                        }
                    }
                }
            }
        }

        // Finally run the engine on this function's AllFacts.
        let output = Output::compute(&all, Algorithm::Hybrid, false);
        if output.errors.is_empty() {
            // ok for this function
        } else {
            // Map engine points back to (block,instr) user-facing locations
            let mut parts = Vec::new();
            for (pt, loans) in output.errors.iter() {
                let idx: usize = (*pt).into();
                if let Some((block, instr)) = point_rev.get(&idx) {
                    parts.push(format!("block {} instr {}: loans {:?}", block, instr, loans));
                } else {
                    parts.push(format!("point {}: loans {:?}", idx, loans));
                }
            }
            return Some(Err(format!("polonius engine errors in {}: {}", func, parts.join("; "))));
        }
    }

    Some(Ok(()))
}

pub fn check_facts(facts: &str) -> Result<(), String> {
    if std::env::var("OMNI_USE_POLONIUS").ok().as_deref() == Some("1") {
        // First, attempt to run the library-backed solver (when compiled with
        // the optional feature). If it returned `Some` that indicates the
        // library either succeeded or produced an error we should forward.
        if let Some(res) = try_polonius_engine(facts) {
            return res;
        }

        // Fall back to invoking the external `polonius` CLI on PATH.
        // Create a temporary directory with per-relation `.facts` files so the
        // Polonius CLI can read tab-delimited tables. The CLI expects a
        // directory containing files like `var_defined_at.facts`, `cfg_edge.facts`,
        // etc. Populate the minimal set of relations derived from our textual
        // exporter and write empty files for the rest.
        use std::collections::{HashMap, HashSet};

        let tmpdir = std::env::temp_dir().join(format!("omni_polonius_{}_{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
        std::fs::create_dir_all(&tmpdir).map_err(|e| format!("adapter: failed to create temp dir {}: {}", tmpdir.display(), e))?;
        let tmp_display = tmpdir.to_string_lossy().into_owned();

        // If requested, print the facts directory path to stderr so callers
        // can inspect the generated `.facts` files for debugging.
        if std::env::var("OMNI_DUMP_FACTS_DIR").ok().as_deref() == Some("1") {
            eprintln!("POLONIUS_FACTS_DIR={}", tmp_display);
        }

        // Prepare containers for relation lines.
        let rel_names = [
            "loan_issued_at",
            "universal_region",
            "cfg_edge",
            "loan_killed_at",
            "subset_base",
            "loan_invalidated_at",
            "var_defined_at",
            "var_used_at",
            "var_dropped_at",
            "use_of_var_derefs_origin",
            "drop_of_var_derefs_origin",
            "child_path",
            "path_is_var",
            "path_assigned_at_base",
            "path_moved_at_base",
            "path_accessed_at_base",
            "known_placeholder_subset",
            "placeholder",
        ];
        let mut rel_lines: HashMap<&str, Vec<String>> = HashMap::new();
        for &r in &rel_names { rel_lines.insert(r, Vec::new()); }

        // First pass: collect points
        let mut point_map: HashMap<(String, String, usize), String> = HashMap::new();
        let mut block_points: HashMap<(String, String), Vec<(usize, String)>> = HashMap::new();
        for raw in facts.lines() {
            let line = raw.trim();
            if line.is_empty() { continue; }
            if let Some(rest) = line.strip_prefix("point ") {
                let mut parts = rest.split_whitespace();
                if let (Some(f), Some(b), Some(i)) = (parts.next(), parts.next(), parts.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        let pkey = format!("P::{}::{}::{}", f, b, ii);
                        point_map.insert((f.to_string(), b.to_string(), ii), pkey.clone());
                        block_points.entry((f.to_string(), b.to_string())).or_default().push((ii, pkey));
                    }
                }
            }
        }

        // Compute first and last point per block (for jump targets and
        // fallthrough CFG edges). Also group blocks per-function so we can
        // synthesize fallthrough edges between consecutive blocks.
        let mut block_first: HashMap<(String, String), String> = HashMap::new();
        let mut block_last: HashMap<(String, String), String> = HashMap::new();
        let mut blocks_by_func: HashMap<String, Vec<String>> = HashMap::new();
        for ((f, b), mut v) in block_points.into_iter() {
            v.sort_unstable_by_key(|(i, _)| *i);
            if let Some((_, p)) = v.first() { block_first.insert((f.clone(), b.clone()), p.clone()); }
            if let Some((_, p)) = v.last() { block_last.insert((f.clone(), b.clone()), p.clone()); }
            blocks_by_func.entry(f).or_default().push(b);
        }

        // Synthesize fallthrough CFG edges between consecutive blocks in a
        // function (e.g., block 0 -> block 1) using the last point of the
        // predecessor and the first point of the successor.
        for (func, mut blocks) in blocks_by_func.into_iter() {
            blocks.sort_unstable_by_key(|s| s.parse::<usize>().unwrap_or(0));
            for w in blocks.windows(2) {
                if let [a, b] = w {
                    if let (Some(src), Some(tgt)) = (block_last.get(&(func.clone(), a.clone())), block_first.get(&(func.clone(), b.clone()))) {
                        rel_lines.get_mut("cfg_edge").unwrap().push(format!("{}\t{}", src, tgt));
                    }
                }
            }
        }

        // Helper to ensure path parent relationships are recorded. Also build
        // a map from parent -> children so we can synthesize child path moves
        // when the base is moved (helps parity with the mock solver).
        let mut path_seen: HashSet<String> = HashSet::new();
        let parent_map = std::cell::RefCell::new(HashMap::<String, Vec<String>>::new());
        let mut ensure_path = |p: &str, rel_lines: &mut HashMap<&str, Vec<String>>| {
            if path_seen.contains(p) { return; }
            let parts: Vec<&str> = p.split('.').collect();
            let mut accum = String::new();
            let mut prev: Option<String> = None;
            for (i, part) in parts.iter().enumerate() {
                if i == 0 { accum = part.to_string(); } else { accum.push('.'); accum.push_str(part); }
                if !path_seen.contains(&accum) {
                    // path_is_var for root
                    if i == 0 {
                        rel_lines.get_mut("path_is_var").unwrap().push(format!("{}\t{}", accum, accum));
                    }
                    if let Some(p2) = prev.as_ref() {
                        rel_lines.get_mut("child_path").unwrap().push(format!("{}\t{}", accum, p2));
                        // Also emit a conservative subset relation for the child
                        // pointing to its parent. This helps the external CLI
                        // understand that `x.a` is a subset of `x`.
                        rel_lines.get_mut("subset_base").unwrap().push(format!("{}\t{}", accum, p2));
                        parent_map.borrow_mut().entry(p2.clone()).or_default().push(accum.clone());
                    }
                    path_seen.insert(accum.clone());
                }
                prev = Some(accum.clone());
            }
        };

        // Second pass: populate relations
        for raw in facts.lines() {
            let line = raw.trim();
            if line.is_empty() { continue; }
            if let Some(rest) = line.strip_prefix("def ") {
                let mut p = rest.split_whitespace();
                if let (Some(_f), Some(b), Some(i), Some(var)) = (p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        if let Some(pk) = point_map.get(&(rest.split_whitespace().next().unwrap().to_string(), b.to_string(), ii)) {
                            if var.contains('.') {
                                ensure_path(var, &mut rel_lines);
                                rel_lines.get_mut("path_assigned_at_base").unwrap().push(format!("{}\t{}", var, pk));
                            } else {
                                rel_lines.get_mut("var_defined_at").unwrap().push(format!("{}\t{}", var, pk));
                            }
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("use ") {
                let mut p = rest.split_whitespace();
                if let (Some(_f), Some(b), Some(i), Some(var)) = (p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        if let Some(pk) = point_map.get(&(rest.split_whitespace().next().unwrap().to_string(), b.to_string(), ii)) {
                            if var.contains('.') {
                                ensure_path(var, &mut rel_lines);
                                rel_lines.get_mut("path_accessed_at_base").unwrap().push(format!("{}\t{}", var, pk));
                                // mark root var used
                                let root = var.split('.').next().unwrap();
                                rel_lines.get_mut("var_used_at").unwrap().push(format!("{}\t{}", root, pk));
                            } else {
                                rel_lines.get_mut("var_used_at").unwrap().push(format!("{}\t{}", var, pk));
                            }
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("move ") {
                let mut p = rest.split_whitespace();
                if let (Some(_f), Some(b), Some(i), Some(src), Some(dest)) = (p.next(), p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        if let Some(pk) = point_map.get(&(rest.split_whitespace().next().unwrap().to_string(), b.to_string(), ii)) {
                            // src moved
                            ensure_path(src, &mut rel_lines);
                            rel_lines.get_mut("path_moved_at_base").unwrap().push(format!("{}\t{}", src, pk));
                            // Also mark known child paths as moved so the external
                            // Polonius CLI sees that moving the base invalidates
                            // dotted child paths (aligns with mock semantics).
                            if let Some(children) = parent_map.borrow().get(src).cloned() {
                                let mut stack: Vec<String> = children.clone();
                                while let Some(child) = stack.pop() {
                                    rel_lines.get_mut("path_moved_at_base").unwrap().push(format!("{}\t{}", child, pk));
                                    if let Some(grand) = parent_map.borrow().get(&child).cloned() {
                                        for c in grand { stack.push(c); }
                                    }
                                }
                            }
                            // dest assigned
                            if dest.contains('.') {
                                ensure_path(dest, &mut rel_lines);
                                rel_lines.get_mut("path_assigned_at_base").unwrap().push(format!("{}\t{}", dest, pk));
                            } else {
                                rel_lines.get_mut("var_defined_at").unwrap().push(format!("{}\t{}", dest, pk));
                            }
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("linear_move ") {
                let mut p = rest.split_whitespace();
                if let (Some(_f), Some(b), Some(i), Some(src), Some(dest)) = (p.next(), p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        if let Some(pk) = point_map.get(&(rest.split_whitespace().next().unwrap().to_string(), b.to_string(), ii)) {
                            ensure_path(src, &mut rel_lines);
                            rel_lines.get_mut("path_moved_at_base").unwrap().push(format!("{}\t{}", src, pk));
                            if dest.contains('.') {
                                ensure_path(dest, &mut rel_lines);
                                rel_lines.get_mut("path_assigned_at_base").unwrap().push(format!("{}\t{}", dest, pk));
                            } else {
                                rel_lines.get_mut("var_defined_at").unwrap().push(format!("{}\t{}", dest, pk));
                            }
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("jump ") {
                let mut p = rest.split_whitespace();
                if let (Some(_f), Some(b), Some(i), Some(target)) = (p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        if let Some(src_pk) = point_map.get(&(rest.split_whitespace().next().unwrap().to_string(), b.to_string(), ii)) {
                            // find target point; prefer instr 0, else first known
                            let tgt_pk = block_first.get(&(rest.split_whitespace().next().unwrap().to_string(), target.to_string()));
                            if let Some(tgt) = tgt_pk {
                                rel_lines.get_mut("cfg_edge").unwrap().push(format!("{}\t{}", src_pk, tgt));
                            }
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("jump_if ") {
                let mut p = rest.split_whitespace();
                if let (Some(_f), Some(b), Some(i), Some(_cond), Some(target)) = (p.next(), p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        if let Some(src_pk) = point_map.get(&(rest.split_whitespace().next().unwrap().to_string(), b.to_string(), ii)) {
                            if let Some(tgt) = block_first.get(&(rest.split_whitespace().next().unwrap().to_string(), target.to_string())) {
                                rel_lines.get_mut("cfg_edge").unwrap().push(format!("{}\t{}", src_pk, tgt));
                            }
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("field ") {
                let mut p = rest.split_whitespace();
                if let (Some(_f), Some(b), Some(i), Some(path)) = (p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        if let Some(pk) = point_map.get(&(rest.split_whitespace().next().unwrap().to_string(), b.to_string(), ii)) {
                            ensure_path(path, &mut rel_lines);
                            rel_lines.get_mut("path_accessed_at_base").unwrap().push(format!("{}\t{}", path, pk));
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("struct_field ") {
                let mut p = rest.split_whitespace();
                if let (Some(_f), Some(b), Some(i), Some(path)) = (p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        if let Some(pk) = point_map.get(&(rest.split_whitespace().next().unwrap().to_string(), b.to_string(), ii)) {
                            ensure_path(path, &mut rel_lines);
                            rel_lines.get_mut("path_accessed_at_base").unwrap().push(format!("{}\t{}", path, pk));
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("index ") {
                let mut p = rest.split_whitespace();
                if let (Some(_f), Some(b), Some(i), Some(expr)) = (p.next(), p.next(), p.next(), p.next()) {
                    if let Ok(ii) = i.parse::<usize>() {
                        if let Some(pk) = point_map.get(&(rest.split_whitespace().next().unwrap().to_string(), b.to_string(), ii)) {
                            let base = if let Some(pos) = expr.find('[') { &expr[..pos] } else { expr };
                            ensure_path(base, &mut rel_lines);
                            rel_lines.get_mut("path_accessed_at_base").unwrap().push(format!("{}\t{}", base, pk));
                        }
                    }
                }
            }
        }

        // Write all relation files (create empty files if no lines)
        for &r in &rel_names {
            let path = tmpdir.join(format!("{}.facts", r));
            let content = rel_lines.remove(r).unwrap_or_default().join("\n");
            std::fs::write(&path, content).map_err(|e| format!("adapter: failed to write {}: {}", path.display(), e))?;
        }

        let cmd = std::env::var("OMNI_POLONIUS_CMD").unwrap_or_else(|_| "polonius".to_string());
        let output = Command::new(&cmd).arg(tmp_display.clone()).output()
            .map_err(|e| format!("adapter: failed to spawn polonius '{}': {} (facts dir: {})", cmd, e, tmp_display))?;

        if output.status.success() {
            return Ok(());
        }

        // Parse stderr lines into a compact diagnostic string and include the
        // facts dir path to make debugging easier.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut diags: Vec<String> = Vec::new();
        for line in stderr.lines() {
            let t = line.trim();
            if t.is_empty() { continue; }
            diags.push(t.to_string());
        }
        if diags.is_empty() {
            return Err(format!("polonius failed (exit {}). facts dir: {}. stderr: {}", output.status, tmp_display, stderr));
        }
        return Err(format!("polonius failed. facts dir: {}. diagnostics: {}", tmp_display, diags.join("; ")));
    }

    // Fallback to the in-repo mock solver.
    match polonius_engine_mock::solve(facts) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
