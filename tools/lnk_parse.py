import re

path = r'C:\Users\慕洛清Mulq\Desktop\MSLX 控制台.lnk'
data = open(path, 'rb').read()

# ASCII 可读串
ascii_strs = re.findall(rb'[\x20-\x7e]{5,}', data)
# UTF-16LE 串
u16 = data.decode('utf-16-le', errors='ignore')
u16_strs = re.findall(r'[ -~\u4e00-\u9fff\\/:.\-_]{4,}', u16)

def interesting(s):
    return (':\\' in s) or ('/home' in s) or ('MSLX' in s) or ('.py' in s) or ('.exe' in s) or ('http' in s)

print("=== ASCII path-ish ===")
for s in ascii_strs:
    s = s.decode('ascii', 'ignore')
    if interesting(s):
        print(repr(s))

print("=== UTF-16 path-ish ===")
seen = set()
for s in u16_strs:
    if interesting(s) and s not in seen:
        seen.add(s)
        print(repr(s))
