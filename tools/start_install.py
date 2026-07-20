import paramiko

HOST = "192.168.10.107"
USER = "mulq"
PASS = "162832"

c = paramiko.SSHClient()
c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect(HOST, username=USER, password=PASS, timeout=30, allow_agent=False, look_for_keys=False)

# 用 setsid 让远端进程彻底脱离 SSH 会话；命令末尾 & 使 exec_command 立即返回
cmd = (
    "setsid bash -c '"
    "echo 162832 | sudo -S sh -c \"apt-get update -qq && apt-get install -y -qq build-essential pkg-config libssl-dev\" >/tmp/apt.log 2>&1; "
    "curl --proto=https --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y >/tmp/rustup.log 2>&1; "
    "echo DONE >> /tmp/install_state.log"
    "' >/tmp/wrapper.log 2>&1 &"
)
stdin, stdout, stderr = c.exec_command(cmd, timeout=20, get_pty=True)
print("launch rc:", stdout.read().decode(errors="replace").strip())
c.close()
print("LAUNCHED")
