#!/usr/bin/env python3
"""Archive Polonius crates and mark as historical."""
import os

archive_dir = 'docs/archive/polonius'

# Update polonius_engine_mock Cargo.toml
mock_cargo = os.path.join(archive_dir, 'polonius_engine_mock', 'Cargo.toml')
if os.path.exists(mock_cargo):
    with open(mock_cargo, 'r') as f:
        content = f.read()
    # Mark version as archived
    content = content.replace('version = "0.2.0"', 'version = "0.2.0-archived"')
    # Add archived marker comment
    marker_line = '\n# [ARCHIVED] This crate is historical; not qualified in v0.1.4.x. '\
                  'Ownership-sensitive MIR reserved for v0.2.0 milestone.\n'
    if marker_line not in content:
        content += marker_line
    with open(mock_cargo, 'w') as f:
        f.write(content)
    print(f"Updated {mock_cargo}")

# Update polonius_engine_adapter Cargo.toml
adapter_cargo = os.path.join(archive_dir, 'polonius_engine_adapter', 'Cargo.toml')
if os.path.exists(adapter_cargo):
    with open(adapter_cargo, 'r') as f:
        content = f.read()
    content = content.replace('version = "0.2.0"', 'version = "0.2.0-archived"')
    marker_line = '\n# [ARCHIVED] This crate is historical; not qualified in v0.1.4.x. '\
                  'Ownership-sensitive MIR reserved for v0.2.0 milestone.\n'\
                  'OWNERSHIP_ORACLE_QUALIFIED = False\n'
    if marker_line not in content:
        content += marker_line
    with open(adapter_cargo, 'w') as f:
        f.write(content)
    print(f"Updated {adapter_cargo}")

print("Done archiving Polonius crates")