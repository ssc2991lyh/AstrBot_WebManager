import paramiko, time

HOST = "192.168.10.107"
USER = "mulq"
PASS = "162832"
BIN = "/home/mulq/astrbot_build/src-tauri/target/release/astrbot-launcher"

c = paramiko.SSHClient()
c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect(HOST, username=USER, password=PASS, timeout=30, allow_agent=False, look_for_keys=False)

# 1) 停服务
print("-- stop service --")
c.exec_command("echo 162832 | sudo -S systemctl stop astrbot-launcher", timeout=30, get_pty=True)
time.sleep(2)

# 2) 替换二进制
print("-- replace binary --")
c.exec_command(f"cp {BIN} /usr/local/bin/astrbot-launcher", timeout=30, get_pty=True)

# 3) 重启
print("-- start service --")
c.exec_command("echo 162832 | sudo -S systemctl start astrbot-launcher", timeout=30, get_pty=True)
time.sleep(3)

# 4) 验证
_, out, _ = c.exec_command(
    "echo 162832 | sudo -S systemctl is-active astrbot-launcher; ss -tlnp | grep 6190 || echo NO_6190",
    timeout=30, get_pty=True,
)
print(out.read().decode(errors="replace").strip())
c.close()
print("DEPLOY_DONE")
