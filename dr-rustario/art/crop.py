#!/usr/bin/env python3
"""Trim the empty rows off the top of a feature shot.

    python3 crop.py <shots> <out>

Every shot of one input is cropped by the *same* amount, worked out from whichever of them has
something highest up, so the five frames of a strip stay comparable. Cropping is in whole cells,
so the grid still divides evenly and an annotation drawn over one still lands on its cell.
"""
import zlib, struct, glob, os, re, sys
def read_png(path):
    d=open(path,'rb').read(); pos=8; idat=b''
    while pos<len(d):
        ln=struct.unpack('>I',d[pos:pos+4])[0]; typ=d[pos+4:pos+8]; data=d[pos+8:pos+8+ln]
        if typ==b'IHDR': w,h,_,_=struct.unpack('>IIBB',data[:10])
        elif typ==b'IDAT': idat+=data
        pos+=12+ln
    raw=zlib.decompress(idat); stride=w*4; px=[]; prev=bytearray(stride); i=0
    for y in range(h):
        f=raw[i]; i+=1; line=bytearray(raw[i:i+stride]); i+=stride
        for x in range(stride):
            a=line[x-4] if x>=4 else 0; b=prev[x]; c=prev[x-4] if x>=4 else 0
            if f==1: line[x]=(line[x]+a)&255
            elif f==2: line[x]=(line[x]+b)&255
            elif f==3: line[x]=(line[x]+(a+b)//2)&255
            elif f==4:
                p=a+b-c; pa,pb,pc=abs(p-a),abs(p-b),abs(p-c)
                pr=a if (pa<=pb and pa<=pc) else (b if pb<=pc else c)
                line[x]=(line[x]+pr)&255
        px.append(bytes(line)); prev=line
    return w,h,px
def write_png(path,w,h,rows):
    raw=b''.join(b'\x00'+r for r in rows)
    def chunk(t,d):
        c=t+d; return struct.pack('>I',len(d))+c+struct.pack('>I',zlib.crc32(c)&0xffffffff)
    open(path,'wb').write(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,6,0,0,0))
        +chunk(b'IDAT',zlib.compress(raw,9))+chunk(b'IEND',b''))
CELL=32
groups={}
SHOTS = sys.argv[1] if len(sys.argv) > 1 else 'shots'
OUT = sys.argv[2] if len(sys.argv) > 2 else 'shots-cropped'
for f in glob.glob(os.path.join(SHOTS, '*.png')):
    groups.setdefault(re.match(r'(\d\d)-', os.path.basename(f)).group(1), []).append(f)
os.makedirs(OUT, exist_ok=True)
for key, files in sorted(groups.items()):
    loaded={f: read_png(f) for f in files}
    first=16
    for f,(w,h,px) in loaded.items():
        for row in range(16):
            band=px[row*CELL:(row+1)*CELL]
            if any(any(line[x:x+3]!=b'\x00\x00\x00' for x in range(0,len(line),4)) for line in band):
                first=min(first,row); break
    top=max(0, first-1)*CELL
    for f,(w,h,px) in loaded.items():
        write_png(os.path.join(OUT, os.path.basename(f)), w, len(px[top:]), px[top:])
print("cropped", sum(len(v) for v in groups.values()))
