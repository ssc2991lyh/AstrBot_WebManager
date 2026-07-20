#!/usr/bin/env python3
# tools/deploy_bin_107.py — 把 153 编译好的二进制部署到 107
import paramiko, os, sys, time

SRC_HOST = os.environ.get('SRC_HOST', '192.168.10.153')
SRC_USER = os.environ.get('SRC_USER', 'mulq')
SRC_PASS = os.environ.get('SRC_PASS', '162832')

DST_HOST = os.environ.get('DST_HOST', '192.168.10.107')
DST_USER = os.environ.get('DST_USER', 'mulq')
DST_PASS = os.environ.get('DST_PASS', '162832')

REMOTE_BIN = "/home/mulq/astrbot_build/src-tauri/target/release/astrbot-launcher"
LOCAL_BIN = os.path.join(os.path.dirname(__file__), ".astrbot-launcher.tmp")
DST_PATH = "/usr/local/bin/astrbot-launcher"


def connect(host, user, password):
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(host, username=user, password=password, timeout=30,
              allow_agent=False, look_for_keys=False)
    return c


def run(c, cmd, sudo=False, timeout=60):
    if sudo:
        cmd = f"echo {DST_PASS} | sudo -S {cmd}"
    stdin, stdout, stderr = c.exec_command(cmd, timeout=timeout)
    out = stdout.read().decode('utf-8', errors='replace')
    err = stderr.read().decode('utf-8', errors='replace')
    rc = stdout.channel.recv_exit_status()
    return rc, out, err


if __name__ == '__main__':
    # 1. fetch binary from 153
    print(f"[1/4] Fetch binary from {SRC_HOST}...")
    c = connect(SRC_HOST, SRC_USER, SRC_PASS)
    sftp = c.open_sftp()
    sftp.get(REMOTE_BIN, LOCAL_BIN)
    sftp.close()
    c.close()
    print(f"      -> {LOCAL_BIN} ({os.path.getsize(LOCAL_BIN)} bytes)")

    # 2. upload to 107 /tmp
    print(f"[2/4] Upload binary to {DST_HOST}...")
    c = connect(DST_HOST, DST_USER, DST_PASS)
    sftp = c.open_sftp()
    sftp.put(LOCAL_BIN, "/tmp/astrbot-launcher.new")
    sftp.close()

    # 3. replace + restart service
    print("[3/4] Replace binary and restart service...")
    for cmd in [
        f"sudo mv /tmp/astrbot-launcher.new {DST_PATH}",
        f"sudo chmod +x {DST_PATH}",
        "sudo systemctl daemon-reload",
        "sudo systemctl restart astrbot-launcher",
    ]:
        rc, out, err = run(c, cmd, sudo=True, timeout=120)
        print(f"$ {cmd}")
        print(out)
        if err.strip():
            print(f"STDERR: {err}")
        if rc != 0:
            print(f"FAILED rc={rc}")
            sys.exit(1)

    # 4. wait and verify
    print("[4/4] Verify service...")
    time.sleep(3)
    rc, out, err = run(c, "systemctl status astrbot-launcher --no-pager -l && curl -s --max-time 5 -X POST http://127.0.0.1:6190/api/get_version -H 'Content-Type: application/json' -d '{}'", timeout=30)
    print(out)
    if err.strip():
        print(f"STDERR: {err}")
    c.close()

    # cleanup local temp
    try:
        os.remove(LOCAL_BIN)
    except OSError:
        pass
    print("[done]")
