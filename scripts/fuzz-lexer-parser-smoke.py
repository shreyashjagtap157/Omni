#!/usr/bin/env python3
"""Deterministic black-box lexer/parser smoke fuzz for historical v0.1.0 closure.

This is a release gate, not a replacement for libFuzzer. It deliberately needs only
an installed `omni` binary and Python, so every release host can exercise malformed
and mixed syntax for >=60 seconds. Normal syntax rejection is allowed; crashes,
signals, timeouts, and launcher failures are not.
"""
from __future__ import annotations
import argparse, os, random, string, subprocess, tempfile, time

TOKENS = [
    "fn", "let", "return", "if", "else", "while", "loop", "break", "continue",
    "pub", "mod", "use", "error", "set", "async", "io", "panic", "true", "false",
    "{", "}", "(", ")", "[", "]", ":", ";", ",", ".", "=", "+", "-", "*", "/",
    "%", "<", ">", "!", "->", "@requires", "@ensures", "\n", "    ", '"text"',
]

def case(rng: random.Random) -> str:
    parts=[]
    for _ in range(rng.randint(1, 90)):
        choice=rng.randrange(5)
        if choice < 3:
            parts.append(rng.choice(TOKENS))
        elif choice == 3:
            parts.append(''.join(rng.choice(string.ascii_letters + string.digits + '_') for _ in range(rng.randint(1,12))))
        else:
            parts.append(str(rng.randint(-(2**63), 2**63-1)))
        if rng.random() < .65: parts.append(' ')
    return ''.join(parts)

def run(cmd: list[str], timeout: float) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, text=True, timeout=timeout)

def main() -> int:
    ap=argparse.ArgumentParser(); ap.add_argument('--omni', default='omni'); ap.add_argument('--seconds', type=float, default=60.0); ap.add_argument('--seed', type=int, default=0x0A11CE); args=ap.parse_args()
    if args.seconds < 1: raise SystemExit('--seconds must be >= 1')
    rng=random.Random(args.seed); deadline=time.monotonic()+args.seconds; count=0
    with tempfile.TemporaryDirectory(prefix='omni-fuzz-smoke-') as td:
        path=os.path.join(td,'case.omni')
        while time.monotonic() < deadline:
            with open(path,'w',encoding='utf-8',errors='strict') as f: f.write(case(rng))
            for sub in ('lex','parse'):
                try:
                    cp=run([args.omni,sub,path], 5.0)
                except subprocess.TimeoutExpired:
                    print(f'FAIL: {sub} timed out at case {count}'); return 1
                except OSError as exc:
                    print(f'FAIL: cannot execute {args.omni}: {exc}'); return 1
                if cp.returncode < 0:
                    print(f'FAIL: {sub} terminated by signal {-cp.returncode} at case {count}'); return 1
            count += 1
    print(f'PASS: lexer/parser smoke fuzz ran {args.seconds:.1f}s, {count} cases, seed={args.seed}')
    return 0
if __name__ == '__main__': raise SystemExit(main())
