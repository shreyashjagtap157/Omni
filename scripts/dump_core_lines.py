from pathlib import Path
p = Path('omni/stdlib/core.omni')
with p.open('r', encoding='utf-8') as f:
    for i, l in enumerate(f, 1):
        if 33 <= i <= 44:
            print(f"{i}: {repr(l.rstrip('\n'))}")
