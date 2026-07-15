#!/usr/bin/env python3
# tools/ssh_run.py — SSH 自动化助手（paramiko）
# 用法: python tools/ssh_run.py <probe|put-src|build|tail|deploy>
import paramiko, os, sys, time, shlex

# 加载同目录 .env 文件（不提交到 GitHub）
_env_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), ".env")
if os.path.exists(_env_path):
    with open(_env_path, encoding="utf-8") as _f:
        for _line in _f:
            _line = _line.strip()
            if not _line or _line.startswith("#") or "=" not in _line:
                continue
            _k, _v = _line.split("=", 1)
            os.environ.setdefault(_k.strip(), _v.strip())

HOST = os.environ.get("SSH_HOST", "192.168.10.153")
PORT = 22
USER = os.environ.get("SSH_USER", "mulq")
PASS = os.environ.get("SSH_PASS", "")

LOCAL_SRC = r"C:/Users/慕洛清Mulq/Desktop/Agent_Desktop/AstrBot_WebManager/src-tauri"
REMOTE_BASE = "/home/mulq/astrbot_build"
REMOTE_SRC = f"{REMOTE_BASE}/src-tauri"
BIN = "astrbot-launcher"  # Cargo package name


def connect():
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(HOST, PORT, USER, PASS, timeout=30,
              allow_agent=False, look_for_keys=False)
    return c


def run(cmd, timeout=None, pty=True):
    c = connect()
    stdin, stdout, stderr = c.exec_command(cmd, timeout=timeout, get_pty=pty)
    out = stdout.read().decode(errors="replace")
    err = stderr.read().decode(errors="replace")
    rc = stdout.channel.recv_exit_status()
    c.close()
    return rc, out, err


def ensure_remote_dir(sftp, path):
    parts = path.strip("/").split("/")
    cur = ""
    for p in parts:
        cur += "/" + p
        try:
            sftp.stat(cur)
        except IOError:
            try:
                sftp.mkdir(cur)
            except IOError:
                pass


def put_dir(local, remote):
    c = connect()
    sftp = c.open_sftp()
    ensure_remote_dir(sftp, remote)
    for root, dirs, files in os.walk(local):
        rel = os.path.relpath(root, local)
        rr = remote if rel == "." else remote + "/" + rel.replace(os.sep, "/")
        ensure_remote_dir(sftp, rr)
        for f in files:
            lp = os.path.join(root, f)
            rp = rr + "/" + f
            sftp.put(lp, rp)
    sftp.close()
    c.close()
    print(f"[put_dir] done: {local} -> {remote}")


def probe():
    cmds = [
        "uname -a",
        "cat /etc/os-release | head -3",
        "df -h /home | tail -2",
        "free -h | head -2",
        "nproc",
        "which cargo rustc 2>&1 || echo NO_RUST",
        "ls $HOME/.cargo/bin 2>/dev/null || echo NO_CARGO_HOME",
        "curl -sI --max-time 8 https://sh.rustup.rs | head -1 || echo NO_NET",
        "id -un && pwd",
    ]
    for cmd in cmds:
        print(f"\n===== $ {cmd} =====")
        rc, out, err = run(cmd)
        sys.stdout.write(out)
        if err.strip():
            sys.stderr.write(err)
        print(f"[rc={rc}]")


def build():
    script = (
        f"set -e; "
        f"export PATH=$HOME/.cargo/bin:$PATH; "
        f"if ! command -v cargo >/dev/null 2>&1; then "
        f"  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal; "
        f"fi; "
        f"source $HOME/.cargo/env; "
        f"cd {REMOTE_SRC} && cargo build --release > $HOME/build.log 2>&1; "
        f"echo BUILD_DONE >> $HOME/build.log"
    )
    rc, out, err = run(f"nohup bash -c '{script}' > $HOME/build_nohup.log 2>&1 &", pty=False)
    print(f"[build] launched rc={rc}; tail build.log to follow")


def tail():
    while True:
        rc, out, err = run("tail -n 40 $HOME/build.log 2>/dev/null")
        sys.stdout.write(out)
        if "BUILD_DONE" in out:
            print("[tail] build finished OK")
            break
        if ("error: could not compile" in out or "error: aborting due to" in out
                or "build failed, waiting" in out):
            print("[tail] BUILD FAILED — see build.log")
            break
        time.sleep(20)


def deploy():
    unit = f"""[Unit]
Description=AstrBot Web Manager - Launcher HTTP backend (port 6190)
After=network.target

[Service]
Type=simple
User={USER}
Environment=ASTRBOT_HTTP_PORT=6190
Environment=ASTRBOT_DATA_DIR=/home/{USER}/.astrbot-launcher
ExecStart=/usr/local/bin/{BIN}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
"""
    # 1. copy binary
    run(f"mkdir -p /usr/local/bin", pty=False)
    rc, out, err = run(
        f"cp {REMOTE_SRC}/target/release/{BIN} /usr/local/bin/{BIN} && chmod +x /usr/local/bin/{BIN} && echo COPIED",
        pty=False,
    )
    print("[deploy] copy binary:", out.strip(), err.strip())
    # 2. write unit
    c = connect()
    sftp = c.open_sftp()
    with sftp.open("/etc/systemd/system/astrbot-launcher.service", "w") as f:
        f.write(unit)
    sftp.close()
    c.close()
    print("[deploy] unit written")
    # 3. enable + start
    for cmd in ["sudo systemctl daemon-reload",
                "sudo systemctl enable astrbot-launcher",
                "sudo systemctl restart astrbot-launcher",
                "sleep 2",
                "sudo systemctl status astrbot-launcher --no-pager | head -8",
                "ss -tlnp | grep 6190 || echo NO_LISTEN"]:
        rc, out, err = run(cmd, pty=False)
        sys.stdout.write(f"$ {cmd}\n{out}{err}\n")
    # 4. probe endpoint
    rc, out, err = run("curl -s --max-time 5 -X POST http://127.0.0.1:6190/api/get_version -H 'Content-Type: application/json' -d '{}' || echo NO_EP", pty=False)
    print("[deploy] endpoint:", out.strip())


def log():
    rc, out, err = run("grep -n -B1 -A14 -E '^error(\\[|:)' $HOME/build.log 2>/dev/null; echo '---NOHUP---'; tail -n 8 $HOME/build_nohup.log 2>/dev/null")
    sys.stdout.write(out + err)


def patch():
    files = ["src/setup.rs", "src/lib.rs", "Cargo.toml"]
    c = connect()
    sftp = c.open_sftp()
    for rel in files:
        lp = os.path.join(LOCAL_SRC, rel)
        rp = REMOTE_SRC + "/" + rel
        d = os.path.dirname(rp)
        ensure_remote_dir(sftp, d.replace(os.sep, "/"))
        sftp.put(lp, rp)
        print(f"[patch] {rel} -> {rp}")
    sftp.close()
    c.close()
    print("[patch] done")


def install_deps():
    passw = shlex.quote(PASS)
    cmds = [
        f"echo {passw} | sudo -S apt-get update -y",
        f"echo {passw} | sudo -S apt-get install -y build-essential pkg-config libssl-dev",
    ]
    for cmd in cmds:
        print(f"\n===== $ {cmd} =====")
        rc, out, err = run(cmd, pty=False, timeout=900)
        sys.stdout.write(out)
        sys.stderr.write(err)
        print(f"[rc={rc}]")


if __name__ == "__main__":
    act = sys.argv[1] if len(sys.argv) > 1 else "probe"
    {"probe": probe, "put-src": lambda: put_dir(LOCAL_SRC, REMOTE_SRC),
     "build": build, "tail": tail, "log": log, "patch": patch, "deps": install_deps,
     "deploy": deploy}[act]()
