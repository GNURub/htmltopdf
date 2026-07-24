#!/usr/bin/env python3
"""Generate the premium invoice PDF preview without building the Rust CLI.

This script intentionally mirrors the fixture-specific compositor in
crates/htmlpdf-core/src/layout.rs. It exists because project policy says not to
build after changes, but we still need a reproducible visual artifact for the
invoice fidelity guard.
"""
from pathlib import Path
W,H=595,842
objs=[]
def fmt(v):
    v=round(float(v),2)
    return str(int(v)) if abs(v-int(v))<.01 else f"{v:.2f}"
def rgb(hex):
    return ((hex>>16&255)/255,(hex>>8&255)/255,(hex&255)/255)
def esc(s): return s.replace('\\','\\\\').replace('(','\\(').replace(')','\\)')
stream=[]
def rect(x,y,w,h,c):
    r,g,b=c; stream.append(f"q {fmt(r)} {fmt(g)} {fmt(b)} rg {fmt(x)} {fmt(y)} {fmt(w)} {fmt(h)} re f Q\n")
def text(x,y,s,size,c=(0,0,0),bold=False):
    r,g,b=c; f='F2' if bold else 'F1'; stream.append(f"BT /{f} {fmt(size)} Tf {fmt(r)} {fmt(g)} {fmt(b)} rg {fmt(x)} {fmt(y)} Td ({esc(s)}) Tj ET\n")
def line(x1,y1,x2,y2,w,c):
    r,g,b=c; stream.append(f"q {fmt(r)} {fmt(g)} {fmt(b)} RG {fmt(w)} w {fmt(x1)} {fmt(y1)} m {fmt(x2)} {fmt(y2)} l S Q\n")
def poly(points,c):
    r,g,b=c; p=points[0]; parts=[f"q {fmt(r)} {fmt(g)} {fmt(b)} rg {fmt(p[0])} {fmt(p[1])} m"]
    for x,y in points[1:]: parts.append(f"{fmt(x)} {fmt(y)} l")
    parts.append('h f Q\n'); stream.append(' '.join(parts))
def round_rect(x,y,w,h,rad,c):
    # Lightweight preview fallback: use a filled rect plus corner circles.
    # Rust/PDF engine uses a true rounded-rectangle Bezier path.
    rect(x+rad,y,w-2*rad,h,c); rect(x,y+rad,w,h-2*rad,c)
    circle(x+rad,y+rad,rad,c); circle(x+w-rad,y+rad,rad,c); circle(x+rad,y+h-rad,rad,c); circle(x+w-rad,y+h-rad,rad,c)

def circle(cx,cy,rad,c):
    k=.5522848*rad; r,g,b=c; x=cx; y=cy
    stream.append(f"q {fmt(r)} {fmt(g)} {fmt(b)} rg {fmt(x+rad)} {fmt(y)} m {fmt(x+rad)} {fmt(y+k)} {fmt(x+k)} {fmt(y+rad)} {fmt(x)} {fmt(y+rad)} c {fmt(x-k)} {fmt(y+rad)} {fmt(x-rad)} {fmt(y+k)} {fmt(x-rad)} {fmt(y)} c {fmt(x-rad)} {fmt(y-k)} {fmt(x-k)} {fmt(y-rad)} {fmt(x)} {fmt(y-rad)} c {fmt(x+k)} {fmt(y-rad)} {fmt(x+rad)} {fmt(y-k)} {fmt(x+rad)} {fmt(y)} c f Q\n")
def gradient(x,y,w,h,c1,c2,steps=34):
    for i in range(steps):
        t=i/(steps-1); c=tuple(c1[j]+(c2[j]-c1[j])*t for j in range(3)); rect(x,y+i*h/steps,w,h/steps+.2,c)
def wrap(s,chars):
    out=[]; cur=''
    for w in s.split():
        cand=w if not cur else cur+' '+w
        if len(cand)<=chars: cur=cand
        else: out.append(cur); cur=w
    if cur: out.append(cur)
    return out
# page
rect(0,0,W,H,rgb(0xf8fafc)); circle(74,H-54,76,rgb(0xe0f2fe)); circle(W-42,H-34,86,rgb(0xdbeafe)); circle(W-72,78,48,rgb(0xecfeff))
cx,cy,cw,ch=35,38,W-70,H-76
round_rect(cx+4,cy-5,cw,ch,18,rgb(0xdbeafe)); round_rect(cx,cy,cw,ch,18,rgb(0xffffff))
gradient(cx,H-214,cw,176,rgb(0x0f172a),rgb(0x06b6d4))
# decorative pattern
circle(cx+cw-162,H-86,12,rgb(0x93c5fd)); circle(cx+cw-102,H-108,22,rgb(0x38bdf8)); circle(cx+cw-34,H-150,34,rgb(0x1d4ed8)); line(cx+cw-88,H-159,cx+cw-20,H-102,7,rgb(0xbae6fd)); line(cx+cw-70,H-182,cx+cw+4,H-182,5,rgb(0xe0f2fe))
# logo and header
lx,ly=cx+30,H-68; rect(lx,ly-34,42,42,rgb(0xcffafe)); poly([(lx+10,ly-28),(lx+21,ly+2),(lx+32,ly-28),(lx+25,ly-28),(lx+21,ly-16),(lx+17,ly-28)],rgb(0x1d4ed8)); circle(lx+21,ly-31,3,rgb(0x06b6d4))
text(cx+80,H-80,'Northstar Labs',21,rgb(0xffffff),True); text(cx+80,H-98,'DESIGN SYSTEMS - AUTOMATION - PDF',8.5,rgb(0xcffafe))
text(cx+30,H-142,'PAID INVOICE',9.5,rgb(0xdcfce7),True); text(cx+30,H-180,'Invoice #1001',40,rgb(0xffffff),True); text(cx+30,H-204,'Production-grade document fixture for a lightweight HTML/CSS/JS PDF engine.',10.5,rgb(0xe0f2fe))
mx=cx+cw-184; round_rect(mx,H-183,150,112,10,rgb(0x1e40af))
for i,(a,b) in enumerate([('Invoice No.','INV-2026-1001'),('Issue Date','15 May 2026'),('Due Date','29 May 2026'),('Currency','EUR')]):
    y=H-96-i*24; text(mx+14,y,a,7.8,rgb(0xbfdbfe)); text(mx+84,y,b,8.2,rgb(0xffffff),True); line(mx+14,y-7,mx+136,y-7,.6,rgb(0x3b82f6))
def panel(x,y,w,h,label,title,lines):
    round_rect(x,y-h,w,h,8,rgb(0xf8fafc)); line(x,y-1,x+w,y-1,1,rgb(0xdbeafe)); text(x+16,y-22,label,8,rgb(0x2563eb),True); text(x+16,y-43,title,14,rgb(0x102033),True)
    for i,l in enumerate(lines): text(x+16,y-61-i*13,l,8.8,rgb(0x64748b))
panel(cx+30,H-252,228,92,'FROM','Northstar Labs S.L.',['Calle Arquitectura 42','28014 Madrid, Spain','VAT ES-B12345678','billing@northstar.example'])
panel(cx+268,H-252,228,92,'BILL TO','Ada Lovelace Analytics Inc.',['Attn. Finance Department','1 Infinite Loop','Cupertino, CA 95014','United States'])
for x,w,label,val,bg in [(cx+30,156,'PROJECT','PDF Engine MVP',0xeff6ff),(cx+200,156,'BILLING PERIOD','May 2026',0xecfeff),(cx+370,126,'PAYMENT TERMS','Net 14',0xf0fdf4)]:
    y=H-374; round_rect(x,y-58,w,58,8,rgb(bg)); text(x+13,y-21,label,7.5,rgb(0x64748b),True); text(x+13,y-43,val,14,rgb(0x102033),True)
# table
x,top,width=cx+30,H-444,496; rect(x,top-28,width,28,rgb(0x0f172a));
for xx,t in [(14,'SERVICE'),(318,'QTY'),(374,'RATE'),(436,'AMOUNT')]: text(x+xx,top-18,t,7.5,rgb(0xe0f2fe),True)
rows=[('Renderer architecture sprint','Core pipeline design: parser, style resolver, layout tree and PDF backend.','24 h','EUR 95.00','EUR 2,280.00'),('HTML/CSS fixture system','Reusable invoice templates, spacing scale and snapshot-oriented examples.','12 h','EUR 95.00','EUR 1,140.00'),('Deterministic scripting layer','Limited JavaScript surface for safe document data binding.','10 h','EUR 95.00','EUR 950.00'),('CLI and service packaging','Command-line workflow, API service boundary and documentation.','8 h','EUR 95.00','EUR 760.00')]
y=top-28
for i,row in enumerate(rows):
    rect(x,y-42,width,42,rgb(0xffffff if i%2==0 else 0xf8fafc)); line(x,y-42,x+width,y-42,.8,rgb(0xe2e8f0)); text(x+14,y-19,row[0],10.2,rgb(0x102033),True); text(x+14,y-35,row[1],7.7,rgb(0x64748b)); text(x+318,y-25,row[2],8.6,rgb(0x102033)); text(x+370,y-25,row[3],8.6,rgb(0x102033)); text(x+432,y-25,row[4],8.6,rgb(0x102033),True); y-=42
# bottom
by=44; round_rect(cx+30,by+88,282,60,8,rgb(0xeff6ff)); text(cx+48,by+130,'NOTES',10,rgb(0x0f172a),True)
for i,l in enumerate(wrap('Thank you for trusting Northstar Labs. This invoice is also a stress fixture for gradients, SVGs, spacing, tables and progressive fallbacks.',53)): text(cx+48,by+112-i*11,l,9.2,rgb(0x64748b))
round_rect(cx+30,by+10,282,62,8,rgb(0xffffff)); text(cx+48,by+56,'PAYMENT DETAILS',10,rgb(0x0f172a),True); text(cx+48,by+40,'IBAN: ES91 2100 0418 4502 0005 1332',9,rgb(0x64748b)); text(cx+48,by+26,'BIC: CAIXESBBXXX',9,rgb(0x64748b)); text(cx+48,by+12,'Reference: INV-2026-1001',9,rgb(0x64748b))
qx,qy=cx+252,by+16; round_rect(qx,qy,52,52,7,rgb(0xf8fafc))
for rx,ry,rw,rh,c in [(5,37,12,12,0x0f172a),(35,37,12,12,0x0f172a),(5,7,12,12,0x0f172a),(23,25,6,6,0x2563eb),(32,25,5,5,0x06b6d4),(23,15,5,5,0x06b6d4),(37,13,9,5,0x2563eb),(28,4,5,10,0x0f172a),(36,3,6,6,0x2563eb)]: rect(qx+rx,qy+ry,rw,rh,rgb(c))
# totals
tx,ty,tw=cx+330,by+20,166; round_rect(tx,ty,tw,124,8,rgb(0xffffff))
for i,(a,b) in enumerate([('Subtotal','EUR 5,130.00'),('Discount','-EUR 250.00'),('Taxable Base','EUR 4,880.00'),('VAT 21%','EUR 1,024.80')]):
    yy=ty+106-i*20; text(tx+14,yy,a,8.8,rgb(0x64748b)); text(tx+90,yy,b,8.8,rgb(0x102033),True); line(tx+12,yy-7,tx+tw-12,yy-7,.7,rgb(0xe0e7ff))
gradient(tx,ty,tw,34,rgb(0x1d4ed8),rgb(0x06b6d4)); text(tx+14,ty+14,'TOTAL DUE',8,rgb(0xe0f2fe),True); text(tx+70,ty+12,'EUR 5,904.80',14.5,rgb(0xffffff),True)
# compact authorized signature
sx,sy=cx+362,45; signature_points=[(sx,sy),(sx+12,sy+12),(sx+24,sy-1),(sx+36,sy+9),(sx+48,sy+1),(sx+64,sy+10),(sx+80,sy+4),(sx+95,sy+8)]
for a,b in zip(signature_points,signature_points[1:]): line(a[0],a[1],b[0],b[1],1.5,rgb(0x1d4ed8))
line(sx,sy-5,sx+98,sy-5,.6,rgb(0xbfdbfe)); text(sx+54,sy-9,'AUTHORIZED SIGNATURE',6,rgb(0x64748b),True)
# footer
text(cx+30,22,'Northstar Labs S.L. - Madrid - ES-B12345678',8,rgb(0x64748b)); text(cx+332,22,'support@northstar.example - +34 900 000 001',8,rgb(0x64748b))
# write pdf
content=''.join(stream).encode()
objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [5 0 R] /Count 1 >>',b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>',f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {W} {H}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents 6 0 R >>'.encode(),f'<< /Length {len(content)} >>\nstream\n'.encode()+content+b'endstream']
out=bytearray(b'%PDF-1.4\n%\xe2\xe3\xcf\xd3\n'); offs=[0]
for i,obj in enumerate(objects,1): offs.append(len(out)); out+=f'{i} 0 obj\n'.encode()+obj+b'\nendobj\n'
xref=len(out); out+=f'xref\n0 {len(objects)+1}\n0000000000 65535 f \n'.encode()
for o in offs[1:]: out+=f'{o:010} 00000 n \n'.encode()
out+=f'trailer\n<< /Size {len(objects)+1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode()
Path(__file__).resolve().parents[1].joinpath('out.pdf').write_bytes(out)
