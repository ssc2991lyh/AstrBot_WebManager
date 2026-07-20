#!/usr/bin/env python3
# do_reset.py — 在 107 上把小雨实例的 AstrBot dashboard 密码改成目标值
import paramiko, sys

HOST = "192.168.10.107"
USER = "mulq"
PASS = "162832"
INSTANCE = "4c6d9a97-f127-442b-8f8e-ebc0c00cacc5"
BASE = f"/home/{USER}/.astrbot_launcher/instances/{INSTANCE}"
ROOT = f"{BASE}/core"  # launcher 实例里 AstrBot 真正的数据根是 core/（data/cmd_config.json 在此）
NEW_PWD = sys.argv[1] if len(sys.argv) > 1 else "astrbot"


def paramiko_quote(s):
    return "'" + s.replace("'", "'\\''") + "'"


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


# 1) 备份原配置
rc, out, err = run(
    f"cp -a {BASE}/core/data/cmd_config.json "
    f"{BASE}/core/data/cmd_config.json.pwd_bak_$(date +%Y%m%d%H%M%S) && echo BACKUP_OK"
)
print("[backup]", out.strip(), err.strip(), f"rc={rc}")

# 2) 用 venv python 调用 astrbot 内部函数设密码（含官方复杂度校验）
setter = (
    "import sys, os\n"
    "from astrbot.cli.commands.cmd_conf import (\n"
    "    _load_config, _save_config, _set_dashboard_password, _validate_dashboard_password)\n"
    "pw = os.environ['NEWPWD']\n"
    "v = _validate_dashboard_password(pw)\n"
    "cfg = _load_config()\n"
    "_set_dashboard_password(cfg, v)\n"
    "_save_config(cfg)\n"
    "print('PASSWORD_SET_OK')\n"
)
cmd = (
    f"export ASTRBOT_ROOT={BASE} && "
    f"cd {BASE} && "
    f"NEWPWD={NEW_PWD} venv/bin/python -c {paramiko_quote(setter)}"
)
rc, out, err = run(cmd, timeout=120)
print("[set]", out.strip(), err.strip(), f"rc={rc}")

# 3) 验证：读取 dashboard 块关键字段（用 utf-8-sig 去 BOM）
verify = (
    "import json\n"
    f"p = '{BASE}/core/data/cmd_config.json'\n"
    "d = json.load(open(p, encoding='utf-8-sig'))\n"
    "db = d.get('dashboard', {})\n"
    "print('has_pbkdf2:', 'pbkdf2_password' in db and bool(db['pbkdf2_password']))\n"
    "print('has_md5:', 'password' in db and bool(db['password']))\n"
    "print('storage_upgraded:', db.get('password_storage_upgraded'))\n"
    "print('change_required:', db.get('password_change_required'))\n"
)
rc, out, err = run(f"cd {BASE} && venv/bin/python -c {paramiko_quote(verify)}", timeout=60)
print("[verify]\n" + out.strip())
if err.strip():
    print("[verify-err]", err.strip())
