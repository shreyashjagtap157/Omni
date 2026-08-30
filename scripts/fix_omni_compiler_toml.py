#!/usr/bin/env python3
"""Fix omni-compiler/Cargo.toml to remove polonius references."""
with open('crates/omni-compiler/Cargo.toml', 'r') as f:
    content = f.read()

# Remove polonius_engine_adapter dependency line
content = content.replace(
    'polonius_engine_adapter = { path = "../polonius_engine_adapter", optional = true }\n',
    ''
)

# Remove the ownership-oracle feature section
old_feature = """# Compatibility feature for the archived ownership oracle. The live v0.1.4
# adapter fails closed; full ownership checking is a v0.2.0 milestone.
ownership-oracle = ["dep:polonius_engine_adapter"]
use_polonius = ["ownership-oracle"]"""
new_feature = """# Ownership oracle is archived; full ownership checking is a v0.2.0 milestone.
# The borrow checker is now implemented in crates/omni-compiler/src/borrow_check/
# use_ownership_oracle = []  # Archived"""
content = content.replace(old_feature, new_feature)

# Also remove any leftover use_polonius line
content = content.replace('use_polonius =', '# use_polonius archived')

with open('crates/omni-compiler/Cargo.toml', 'w') as f:
    f.write(content)
print('Fixed omni-compiler/Cargo.toml - removed polonius references')
"