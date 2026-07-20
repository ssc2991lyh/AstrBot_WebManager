import paramiko, time, sys

HOST="192.168.10.107"; USER="mulq"; PASS="162832"; SUDO="162832"

wrapper = """#!/bin/bash
export DEBIAN_FRONTEND=noninteractive
echo "START $(date)" > /tmp/rust_install.log
echo 162832 | sudo -S apt-get update -qq >> /tmp/rust_install.log 2>&1
echo 162832 | sudo -S apt-get install -y -qq build-essential pkg-config libssl-dev >> /tmp/rust_install.log 2>&1
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y >> /tmp/rust_install.log 2>&1
echo "DONE $(date)" >> /tmp/rust_install.log
"""

c=paramiko.SSHClient(); c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect(HOST,username=USER,password=PASS,timeout=30,allow_agent=False,look_for_keys=False)
# 写 wrapper
with c.open_sftp() as sftp:
    with sftp.open("/tmp/rust_install.sh","w") as f:
        f.write(wrapper)
        f.close()

# 用 transport 直接开 session，setsid 脱离，立即关 channel（不读 stdout，避免杀进程）
transport=c.get_transport()
ch=transport.open_session()
ch.get_pty()
ch.exec_command("setsid bash /tmp/rust_install.sh </dev/null >/dev/null 2>&1 & echo launched")
time.sleep(2)
ch.close()
c.close()
print("launched install detached on 107")
