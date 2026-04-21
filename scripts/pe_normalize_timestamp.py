import sys, struct, hashlib

def read_e_lfanew(data):
    return struct.unpack_from('<I', data, 0x3C)[0]

def normalize(path, other_ts_set, out_path):
    with open(path,'rb') as f:
        data = bytearray(f.read())
    e_lfanew = read_e_lfanew(data)
    ts_off = e_lfanew + 8
    if ts_off + 4 <= len(data):
        original_ts = struct.unpack_from('<I', data, ts_off)[0]
        # zero the header timestamp
        data[ts_off:ts_off+4] = b"\x00\x00\x00\x00"
    else:
        original_ts = None
    # replace any occurrences of any known ts values (including other file)
    candidates = set([original_ts])
    candidates.update(other_ts_set)
    if None in candidates:
        candidates.discard(None)
    for ts in list(candidates):
        if ts is None:
            continue
        pat = struct.pack('<I', ts)
        idx = data.find(pat)
        while idx != -1:
            data[idx:idx+4] = b"\x00\x00\x00\x00"
            idx = data.find(pat, idx+4)
    with open(out_path,'wb') as f:
        f.write(data)
    return original_ts

if __name__=='__main__':
    if len(sys.argv) <3:
        print('usage: pe_normalize_timestamp.py file1 file2'); sys.exit(1)
    f1 = sys.argv[1]
    f2 = sys.argv[2]
    # read both to get ts values
    with open(f1,'rb') as a:
        d1 = a.read()
    with open(f2,'rb') as b:
        d2 = b.read()
    e1 = struct.unpack_from('<I', d1, 0x3C)[0]
    e2 = struct.unpack_from('<I', d2, 0x3C)[0]
    ts1_off = e1 + 8
    ts2_off = e2 + 8
    ts1 = struct.unpack_from('<I', d1, ts1_off)[0] if ts1_off+4 <= len(d1) else None
    ts2 = struct.unpack_from('<I', d2, ts2_off)[0] if ts2_off+4 <= len(d2) else None
    # normalize each, replacing any occurrences of either ts
    tset = set([x for x in (ts1, ts2) if x is not None])
    out1 = sys.argv[1] + '.norm.exe'
    out2 = sys.argv[2] + '.norm.exe'
    normalize(sys.argv[1], tset, out1)
    normalize(sys.argv[2], tset, out2)
    import subprocess
    import os
    h1 = hashlib.sha256(open(out1,'rb').read()).hexdigest()
    h2 = hashlib.sha256(open(out2,'rb').read()).hexdigest()
    print('normalized hashes:')
    print(out1, h1)
    print(out2, h2)
