#!/usr/bin/env python3
# reset_xiaoyu_pwd.py — 连接 107，探查 astrbot password 命令用法
import paramiko, sys

HOST = "192.168.10.107"
USER = "mulq"
PASS = "162832"

INSTANCE = "4c6d9a97-f127-442b-8f8e-ebc0c00cacc5"
BASE = f"/home/{USER}/.astrbot_launcher/instances/{INSTANCE}"


def run(cmd, timeout=120):
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(HOST, username=USER, password=PASS, timeout=30,
              allow_agent=False, look_for_keys=False)
    stdin, stdout, stderr = c.exec_command(cmd, timeout=timeout, get_pty=True)
    out = stdout.read().decode("utf-8", errors="replace")
    err = stderr.read().decode("utf-8", errors="replace")
    rc = stdout.channel.recv_exit_status()
    c.close()
    return rc, out, err


if __name__ == "__main__":
    mode = sys.argv[1] if len(sys.argv) > 1 else "probe"
    if mode == "probe":
        # 确认 CLI 存在 + 查看 password 子命令帮助
        cmds = [
            f"ls -la {BASE}/venv/bin/astrbot",
            f"cd {BASE} && venv/bin/astrbot --help 2>&1 | head -40",
            f"cd {BASE} && venv/bin/astrbot password --help 2>&1 | head -40",
            f"cat {BASE}/core/data/cmd_config.json | head -60",
        ]
        for cmd in cmds:
            print(f"\n===== $ {cmd} =====")
            rc, out, err = run(cmd)
            print(out)
            if err.strip():
                print("[stderr]", err)
            print(f"[rc={rc}]")
    elif mode == "reset":
        newpwd = sys.argv[2]
        cmd = f"cd {BASE} && venv/bin/astrbot password {newpwd} 2>&1"
        print(f"\n===== $ {cmd} =====")
        rc, out, err = run(cmd, timeout=120)
        print(out)
        if err.strip():
            print("[stderr]", err)
        print(f"[rc={rc}]")
        # 复核配置
        rc, out, err = run(f"cat {BASE}/core/data/cmd_config.json | head -60")
        print("\n===== cmd_config.json (head) =====")
        print(out)
