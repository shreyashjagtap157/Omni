#!/usr/bin/env python3
"""Add trait bound tracking to the Omni type checker."""

import sys

filepath = "crates/omni-compiler/src/type_checker.rs"

with open(filepath, "r") as f:
    lines = f.readlines()

# 1. Add trait_bounds field to InferCtx struct
# Find "subs: HashMap<u32, Type>," and add field after it
insert_idx = None
for i, line in enumerate(lines):
    if 'subs: HashMap<u32, Type>,' in line:
        insert_idx = i + 1
        break

if insert_idx is None:
    print("ERROR: Could not find 'subs: HashMap<u32, Type>,' field")
    sys.exit(1)

# Insert trait_bounds field
lines.insert(insert_idx, '    trait_bounds: HashMap<(String, String), bool>,\n')  # (fn_name, trait_name) -> is_negative
print(f"Added trait_bounds field at line {insert_idx + 1}")

# 2. Add record_trait_bound method to impl InferCtx block
# Find "impl InferCtx {" 
impl_start = None
for i, line in enumerate(lines):
    if 'impl InferCtx {' in line:
        impl_start = i
        break

if impl_start is None:
    print("ERROR: Could not find 'impl InferCtx {'")
    sys.exit(1)

# Find fresh_var method and add record_trait_bound after it
insert_method_idx = None
for i in range(impl_start, min(impl_start + 100, len(lines))):
    if 'fn fresh_var' in lines[i]:
        # Insert after the closing brace of fresh_var
        for j in range(i, min(i + 20, len(lines))):
            if lines[j].strip() == '}' and j > i:
                # Insert after this closing brace
                method_code = '''
            fn record_trait_bound(&mut self, fn_name: &str, trait_name: &str, is_negative: bool) {
                self.trait_bounds.insert((fn_name.to_string(), trait_name.to_string()), is_negative);
            }
'''
                lines.insert(j + 1, method_code)
                print(f"Added record_trait_bound method after line {j + 1}")
                insert_method_idx = j + 1
                break
        break

if insert_method_idx is None:
    print("ERROR: Could not insert record_trait_bound method")
    sys.exit(1)

# 3. Add the actual bound validation code after function insertion into symbols
# Find the symbols.entry insertion point
target_code = '''            symbols.entry(name.clone()).or_insert(Type::Fn {\n                params: ptypes,\n                ret: Box::new(rtype),\n                effects: efmask,\n            });'''

old_code_pattern = '''            symbols.entry(name.clone()).or_insert(Type::Fn {\n                params: ptypes,\n                ret: Box::new(rtype),\n                effects: efmask,\n            });'''

new_code = '''            symbols.entry(name.clone()).or_insert(Type::Fn {\n                params: ptypes,\n                ret: Box::new(rtype),\n                effects: efmask,\n            });

            // Validate trait bounds on generic parameters
            for (gp_name, gp_bounds) in type_params {
                for bound in gp_bounds {
                    let is_negative = bound.starts_with('!');
                    let trait_name = if is_negative { &bound[1..] } else { bound };
                    ctx.record_trait_bound(&name, &gp_name, is_negative, &trait_name);
                }
            }'''

if old_code_pattern in "".join(lines):
    # Find and replace
    content = "".join(lines)
    content = content.replace(old_code_pattern, new_code)
    with open(filepath, "w") as f:
        f.write(content)
    print("Added trait bound validation after function insertion")
else:
    print("Could not find target code for replacement")
    # Show what we have around line 2583
    for i in range(2578, min(2595, len(lines))):
        print(f"{i+1}: {lines[i]}", end='')

print("\nDone!")