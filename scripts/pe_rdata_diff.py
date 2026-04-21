import sys, struct

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

if __name__=='__main__':
    if len(sys.argv) <3:
        print('usage: pe_rdata_diff.py file1 file2'); sys.exit(1)
    p1=sys.argv[1]
    p2=sys.argv[2]
    s1=read_pe_sections(p1)
    s2=read_pe_sections(p2)
    if ' .rdata' in s1 and ' .rdata' in s2:
        pass
    # try without space
    key='rdata'
    if key not in s1:
        key='.rdata'
    if key not in s1:
        print('rdata section not found'); sys.exit(1)
    off1,sz1 = s1[key]; off2,sz2 = s2[key]
    with open(p1,'rb') as f1, open(p2,'rb') as f2:
        f1.seek(off1); f2.seek(off2)
        b1=f1.read(sz1); b2=f2.read(sz2)
    minlen=min(len(b1),len(b2))
    diffs=0
    for i in range(minlen):
        if b1[i]!=b2[i]:
            print(hex(i), b1[i], b2[i])
            diffs+=1
            if diffs>=50:
                break
    print('total diffs reported (first 50):', diffs)
