p = r"d:\Project\Omni\omni\stdlib\core.omni"
with open(p, 'r', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        # show line number, leading whitespace as count and repr
        leading = len(line) - len(line.lstrip('\t '))
        # show exact leading chars
        leading_chars = line[:len(line) - len(line.lstrip('\t '))]
        print(f"{i:03}: lead={leading} chars={leading_chars!r} | {line.rstrip()!r}")
