import paramiko
HOST="192.168.10.107"; USER="mulq"; PASS="162832"
INST="4c6d9a97-f127-442b-8f8e-ebc0c00cacc5"
c=paramiko.SSHClient(); c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect(HOST,username=USER,password=PASS,timeout=30,allow_agent=False,look_for_keys=False)
def run(cmd):
    i,o,e=c.exec_command(cmd,timeout=30,get_pty=True)
    return o.read().decode(errors='replace').strip(), e.read().decode(errors='replace').strip()
tp="/data/_wb_probe.txt"
o,e=run('curl -s --max-time 10 -X POST "http://127.0.0.1:6190/api/files/instance/%s/content" -H "Content-Type: application/json" -d "{\\"path\\":\\"%s\\",\\"content\\":\\"hello-wb-123\\"}"' % (INST,tp))
print("[write]   ", o)
o,e=run('curl -s --max-time 10 "http://127.0.0.1:6190/api/files/instance/%s/content?path=%s"' % (INST,tp))
print("[read]    ", o)
o,e=run('curl -s --max-time 10 -X POST "http://127.0.0.1:6190/api/files/instance/%s/delete" -H "Content-Type: application/json" -d "{\\"paths\\":[\\"%s\\"]}"' % (INST,tp))
print("[delete]  ", o)
o,e=run('curl -s --max-time 10 "http://127.0.0.1:6190/api/files/instance/%s/content?path=%s"' % (INST,tp))
print("[after-del]", o)
o,e=run('curl -s --max-time 10 -X POST "http://127.0.0.1:6190/api/files/instance/%s/directory" -H "Content-Type: application/json" -d "{\\"path\\":\\"/data/_wb_dir\\"}"' % INST)
print("[mkdir]   ", o)
o,e=run('curl -s --max-time 10 -X POST "http://127.0.0.1:6190/api/files/instance/%s/delete" -H "Content-Type: application/json" -d "{\\"paths\\":[\\"/data/_wb_dir\\"]}"' % INST)
print("[rmdir]   ", o)
c.close()
