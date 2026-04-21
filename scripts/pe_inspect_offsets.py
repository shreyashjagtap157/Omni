import sys, struct, binascii


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


def hexdump_region(data, start, length=64):
    end = min(len(data), start + length)
    chunk = data[start:end]
    hexb = ' '.join(f"{b:02x}" for b in chunk)
    asci = ''.join((chr(b) if 32 <= b < 127 else '.') for b in chunk)
    return hexb, asci, start, end


if __name__=='__main__':
    if len(sys.argv) < 3:
        print('usage: pe_inspect_offsets.py file1 file2 [context_bytes]')
        sys.exit(1)
    p1=sys.argv[1]
    p2=sys.argv[2]
    ctx = int(sys.argv[3]) if len(sys.argv)>=4 else 64

    s1=read_pe_sections(p1)
    s2=read_pe_sections(p2)
    key = '.rdata' if '.rdata' in s1 else 'rdata'
    if key not in s1:
        print('rdata section not found in', p1); sys.exit(1)
    off1, sz1 = s1[key]
    off2, sz2 = s2[key]
    with open(p1,'rb') as f1, open(p2,'rb') as f2:
        f1.seek(off1); f2.seek(off2)
        b1=f1.read(sz1); b2=f2.read(sz2)
    minlen = min(len(b1), len(b2))
    diffs = []
    for i in range(minlen):
        if b1[i] != b2[i]:
            diffs.append(i)
            if len(diffs) >= 200:
                break
    if not diffs:
        print('No diffs found in .rdata')
        sys.exit(0)
    print(f'Found {len(diffs)} differing offsets (showing up to 200).')
    for off in diffs:
        b1v = b1[off]
        b2v = b2[off]
        print('---')
        print(f'offset: 0x{off:x} (rdata-relative)  file1_byte=0x{b1v:02x} file2_byte=0x{b2v:02x}')
        f1_file_off = off1 + off
        f2_file_off = off2 + off
        print(f'file offsets: file1=0x{f1_file_off:x} file2=0x{f2_file_off:x}')
        pre = max(0, off-ctx)
        hex1, ascii1, s1r, e1r = hexdump_region(b1, pre, (ctx*2))
        hex2, ascii2, s2r, e2r = hexdump_region(b2, pre, (ctx*2))
        print(f'file1 (rdata[{s1r}:{e1r}]):')
        print(hex1)
        print(ascii1)
        print(f'file2 (rdata[{s2r}:{e2r}]):')
        print(hex2)
        print(ascii2)
    print('Done.')
