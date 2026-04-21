import sys, hashlib


def strip_codeview(path, out_path):
    data = bytearray(open(path,'rb').read())
    idx = 0
    found = 0
    while True:
        idx = data.find(b'RSDS', idx)
        if idx == -1:
            break
        found += 1
        guid_start = idx + 4
        # zero GUID (16 bytes) and age (4 bytes)
        if guid_start + 20 <= len(data):
            data[guid_start:guid_start+20] = b'\x00' * 20
            # zero filename bytes until null or up to 256 bytes
            fname_start = guid_start + 20
            end = fname_start
            while end < len(data) and end < fname_start + 1024:
                if data[end] == 0:
                    break
                data[end] = 0
                end += 1
        idx = guid_start + 20
    open(out_path,'wb').write(data)
    return found

if __name__=='__main__':
    if len(sys.argv) < 3:
        print('usage: pe_strip_codeview.py file1 file2'); sys.exit(1)
    f1 = sys.argv[1]
    f2 = sys.argv[2]
    out1 = f1 + '.strip.exe'
    out2 = f2 + '.strip.exe'
    n1 = strip_codeview(f1, out1)
    n2 = strip_codeview(f2, out2)
    h1 = hashlib.sha256(open(out1,'rb').read()).hexdigest()
    h2 = hashlib.sha256(open(out2,'rb').read()).hexdigest()
    print('stripped entries:', n1, n2)
    print(out1, h1)
    print(out2, h2)
