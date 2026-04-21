import sys, hashlib, struct


def read_pe_sections(path):
    with open(path,'rb') as f:
        data=f.read()
    if len(data) < 0x40:
        raise SystemExit("file too small")
    e_lfanew = struct.unpack_from('<I', data, 0x3C)[0]
    if data[e_lfanew:e_lfanew+4] != b'PE\x00\x00':
        raise SystemExit("not PE")
    # IMAGE_FILE_HEADER at e_lfanew+4
    number_of_sections = struct.unpack_from('<H', data, e_lfanew+6)[0]
    size_of_optional_header = struct.unpack_from('<H', data, e_lfanew+20)[0]
    section_table_offset = e_lfanew + 4 + 20 + size_of_optional_header
    sections=[]
    for i in range(number_of_sections):
        off = section_table_offset + i*40
        name = data[off:off+8].rstrip(b'\x00').decode('ascii', errors='replace')
        virtual_size = struct.unpack_from('<I', data, off+8)[0]
        virtual_address = struct.unpack_from('<I', data, off+12)[0]
        size_of_raw_data = struct.unpack_from('<I', data, off+16)[0]
        pointer_to_raw_data = struct.unpack_from('<I', data, off+20)[0]
        raw = data[pointer_to_raw_data:pointer_to_raw_data+size_of_raw_data]
        sha = hashlib.sha256(raw).hexdigest()
        sections.append((name, pointer_to_raw_data, size_of_raw_data, sha))
    return sections


def main(p1,p2):
    s1=read_pe_sections(p1)
    s2=read_pe_sections(p2)
    print("sections for", p1)
    for name,off,size,sha in s1:
        print(name, hex(off), size, sha[:16])
    print()
    print("sections for", p2)
    for name,off,size,sha in s2:
        print(name, hex(off), size, sha[:16])
    print()
    # compare
    print("differences:")
    for i, (name,off,size,sha) in enumerate(s1):
        if i < len(s2):
            if sha != s2[i][3] or size != s2[i][2]:
                print(f"{name}: {sha[:16]} vs {s2[i][3][:16]} size {size} vs {s2[i][2]}")
        else:
            print(f"{name}: only in {p1}")

if __name__=='__main__':
    if len(sys.argv) <3:
        print("usage: pe_compare.py file1 file2"); sys.exit(1)
    main(sys.argv[1], sys.argv[2])
