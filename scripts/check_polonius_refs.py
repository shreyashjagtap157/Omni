#!/usr/bin/env python3
import tomllib
import pathlib

ROOT = pathlib.Path(r'C:\Users\siddh\Downloads\ABC\Omni')
with open(ROOT / 'Cargo.toml', 'rb') as f:
    data = tomllib.load(f)
members = data.get('workspace', {}).get('members', [])
for m in members:
    p = ROOT / m
    if p.exists():
        with open(p / 'Cargo.toml', 'rb') as f2:
            pkg = tomllib.load(f2).get('package', {})
            deps = pkg.get('dependencies', {})
            for d, s in deps.items():
                if isinstance(s, dict) and 'path' in s:
                    if 'polonius' in d.lower() or 'polonius' in s['path'].lower():
                        print(f'{m} references polonius: {d} -> {s["path"]}')
print("Check complete")