#!/usr/bin/env python3
"""Remove polonius entries from Cargo.lock."""
with open('Cargo.lock', 'r') as f:
    content = f.read()

# Remove polonius_engine_adapter and polonius_engine_mock entries
# Carefully remove the package entries
import re

# Remove the package entries by finding them
lines = content.split('\n')
new_lines = []
skip = False
for line in lines:
    # Check if this line starts a polonius package entry
    if re.match(r'^\["polonius_engine_adapter"\]', line) or re.match(r'^\["polonius_engine_mock"\]', line):
        skip = True
        continue
    # If we're skipping, continue until we hit a new package or end
    if skip:
        # Check if this is the start of a new package
        if re.match(r'^\["', line) and not re.match(r'^\["polonius', line):
            skip = False
            new_lines.append(line)
        continue
    new_lines.append(line)

with open('Cargo.lock', 'w') as f:
    f.write('\n'.join(new_lines))
print('Removed polonius entries from Cargo.lock')
"