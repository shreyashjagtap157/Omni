#!/usr/bin/env python3
"""Remove Polonius crates from workspace members and archive them."""
import re

with open('Cargo.toml', 'r') as f:
    content = f.read()

# Remove polonius_engine_mock from members
content = content.replace('"crates/polonius_engine_mock",\n', '')
# Remove polonius_engine_adapter from members
content = content.replace('"crates/polonius_engine_adapter",\n', '')

# Remove from default-members if present
content = content.replace('"polonius_engine_mock"', '"# [archived] polonius_engine_mock"')
content = content.replace('"polonius_engine_adapter"', '"# [archived] polonius_engine_adapter"')

with open('Cargo.toml', 'w') as f:
    f.write(content)

print("Updated Cargo.toml")

# Verify the members
m = re.search(r'\[workspace\]\s*\n\s*members\s*=\s*\[(.*?)\]', content, re.DOTALL)
if m:
    print("Workspace members:")
    print(m.group(1))