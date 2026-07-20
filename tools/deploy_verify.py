import paramiko
HOST="192.168.10.107"; USER="mulq"; PASS="162832"
SRC="/home/mulq/astrbot_build/src-tauri/target/release/astrbot-launcher"
DST="/usr/local/bin/astrbot-launcher"
INST="4c6d9a97-f127-442b-8f8e-ebc0c00cacc5"

c=paramiko.SSHClient(); c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect(HOST,username=USER,password=PASS,timeout=30,allow_agent=False,look_for_keys=False)

def run(cmd):
    stdin,stdout,stderr=c.exec_command(cmd,timeout=90,get_pty=True)
    out=stdout.read().decode(errors='replace')
    err=stderr.read().decode(errors='replace')
    return out,err

# 1) 停服务
o,e=run("echo 162832 | sudo -S systemctl stop astrbot-launcher 2>&1; sleep 1; echo STOP_DONE")
print("[stop]", o.strip(), e.strip())

# 2) 复制新二进制
o,e=run(f"cp -f {SRC} {DST} && ls -la {DST} && echo COPY_OK")
print("[copy]", o.strip(), e.strip())

# 3) 启动服务
o,e=run("echo 162832 | sudo -S systemctl start astrbot-launcher 2>&1; sleep 3; echo START_DONE")
print("[start]", o.strip(), e.strip())

# 4) 检查 6190 监听
o,e=run("ss -tlnp 2>/dev/null | grep 6190 || echo NO_6190")
print("[listen]", o.strip(), e.strip())

# 5) 服务状态
o,e=run("echo 162832 | sudo -S systemctl is-active astrbot-launcher 2>&1")
print("[active]", o.strip(), e.strip())

# 6) 验证文件管理路由：列小雨实例 core 根目录
o,e=run(f"curl -s --max-time 10 'http://127.0.0.1:6190/api/files/instance/{INST}/lists?path=/' | head -c 800")
print("[lists-resp]", o.strip())
print("[lists-err]", e.strip())

# 7) 验证写+读回（在 core/data 下建临时文件再读再删）
test_path="/data/_wb_probe.txt"
o,e=run(f"curl -s --max-time 10 -X POST 'http://127.0.0.1:6190/api/files/instance/{INST}/content?path={test_path}' -H 'Content-Type: text/plain' -d 'hello-wb' ; echo ; echo WRITE_DONE")
print("[write]", o.strip(), e.strip())
o,e=run(f"curl -s --max-time 10 'http://127.0.0.1:6190/api/files/instance/{INST}/content?path={test_path}' | head -c 200")
print("[read-back]", o.strip())
del_body = '["' + test_path + '"]'
o,e=run("curl -s --max-time 10 -X POST 'http://127.0.0.1:6190/api/files/instance/%s/delete' -H 'Content-Type: application/json' -d '%s' ; echo ; echo DELETE_DONE" % (INST, del_body))
print("[cleanup]", o.strip(), e.strip())

c.close()
