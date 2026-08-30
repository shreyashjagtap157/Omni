#!/usr/bin/env python3
"""Add trait bound tracking to the Omni type checker."""

filepath = "crates/omni-compiler/src/type_checker.rs"

with open(filepath, "r") as f:
    content = f.read()

# Find the location after symbols.entry and add bound validation
old_code = """            symbols.entry(name.clone()).or_insert(Type::Fn {
                params: ptypes,
                ret: Box::new(rtype),
                effects: efmask,
            });"""

new_code = """            symbols.entry(name.clone()).or_insert(Type::Fn {
                params: ptypes,
                ret: Box::new(rtype),
                effects: efmask,
            });

            // Validate trait bounds on generic parameters
            for (gp_name, gp_bounds) in type_params {
                for bound in gp_bounds {
                    let is_negative = bound.starts_with('!');
                    let trait_name = if is_negative { &bound[1..] } else { bound };
                    // Store (fn_name, trait_name) -> is_negative for later checking
                    ctx.trait_bounds
                        .insert((name.clone(), trait_name.to_string()), is_negative);
                }
            }"""

if old_code in content:
    content = content.replace(old_code, new_code)
    with open(filepath, "w") as f:
        f.write(content)
    print("Successfully added trait bound validation code")
else:
    print("Could not find target code - searching for partial match...")
    if 'symbols.entry(name.clone()).or_insert' in content:
        print("Found symbols.entry insertion")
    else:
        print("Could not find symbols.entry insertion either")