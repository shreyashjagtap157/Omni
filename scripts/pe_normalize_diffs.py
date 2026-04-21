import sys, struct, hashlib


def read_pe_sections(path):
    with open(path,'rb') as f:
        data=f.read()
    e_lfanew = struct.unpack_from('<I', data, 0x3C)[0]
    number_of_sections = struct.unpack_from('<H', data, e_lfanew+6)[0]
    size_of_optional_header = struct.unpack_from('<H', data, e_lfanew+20)[0]
    section_table_offset = e_lfanew + 4 + 20 + size_of_optional_header
    sections={}
    for i in range(number_of_sections):
        off = section_table_offset + i*40
        name = data[off:off+8].rstrip(b'\x00').decode('ascii', errors='replace')
        size_of_raw_data = struct.unpack_from('<I', data, off+16)[0]
        pointer_to_raw_data = struct.unpack_from('<I', data, off+20)[0]
        sections[name] = (pointer_to_raw_data, size_of_raw_data)
    return sections


def find_diffs(b1,b2,max_diffs=1000):
    minlen = min(len(b1), len(b2))
    diffs=[]
    for i in range(minlen):
        if b1[i]!=b2[i]:
            diffs.append(i)
            if len(diffs)>=max_diffs:
                break
    return diffs


def cluster(diffs, gap=16):
    if not diffs:
        return []
    ranges=[]
    start=diffs[0]
    last=diffs[0]
    for d in diffs[1:]:
        if d - last <= gap:
            last=d
        else:
            ranges.append((start,last))
            start=d; last=d
    ranges.append((start,last))
    return ranges


def zero_ranges(data, ranges, pad=16):
    arr = bytearray(data)
    for s,e in ranges:
        ss = max(0, s-pad)
        ee = min(len(arr)-1, e+pad)
        for i in range(ss, ee+1):
            arr[i]=0
    return bytes(arr)

if __name__=='__main__':
    if len(sys.argv)<3:
        print('usage: pe_normalize_diffs.py file1 file2'); sys.exit(1)
    f1=sys.argv[1]; f2=sys.argv[2]
    s1 = read_pe_sections(f1)
    s2 = read_pe_sections(f2)
    key = '.rdata' if '.rdata' in s1 else 'rdata'
    if key not in s1:
        print('rdata not found'); sys.exit(1)
    off1,sz1 = s1[key]
    off2,sz2 = s2[key]
    with open(f1,'rb') as a, open(f2,'rb') as b:
        a.seek(off1); b.seek(off2)
        b1 = a.read(sz1); b2 = b.read(sz2)
    diffs = find_diffs(b1,b2, max_diffs=10000)
    ranges = cluster(diffs, gap=64)
    print('found diffs', len(diffs), 'clusters', ranges)
    nb1 = zero_ranges(b1, ranges, pad=32)
    nb2 = zero_ranges(b2, ranges, pad=32)
    # write out normalized files by replacing only rdata section
    with open(f1,'rb') as a:
        whole = bytearray(a.read())
    with open(f2,'rb') as b:
        whole2 = bytearray(b.read())
    whole[off1:off1+len(nb1)] = nb1
    whole2[off2:off2+len(nb2)] = nb2
    out1 = f1 + '.norm2.exe'
    out2 = f2 + '.norm2.exe'
    open(out1,'wb').write(whole)
    open(out2,'wb').write(whole2)
    h1 = hashlib.sha256(open(out1,'rb').read()).hexdigest()
    h2 = hashlib.sha256(open(out2,'rb').read()).hexdigest()
    print(out1, h1)
    print(out2, h2)
