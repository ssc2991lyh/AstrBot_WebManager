#!/usr/bin/env python3
# reset2000.py — 在 107 上把 2000 实例(AstrBot, d72de757-...) 的 dashboard 密码重置
import paramiko, sys

HOST = "192.168.10.107"
USER = "mulq"
PASS = "162832"
INSTANCE = "d72de757-1b62-4dfd-b0ef-129f9375b816"
BASE = f"/home/{USER}/.astrbot_launcher/instances/{INSTANCE}"
ROOT = f"{BASE}/core"   # ASTRBOT_ROOT 必须指向 AstrBot 真实根 core/，否则 _load_config 报 not a valid root
NEW_PWD = sys.argv[1] if len(sys.argv) > 1 else "Mulq@0458linyuhao"


def q(s):
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
    f"cp -a {ROOT}/data/cmd_config.json "
    f"{ROOT}/data/cmd_config.json.pwd_bak_$(date +%Y%m%d%H%M%S) && echo BACKUP_OK"
)
print("[backup]", out.strip(), err.strip(), f"rc={rc}")

# 2) 用 venv python 直接读写 cmd_config.json（绕过 get_astrbot_root 的 .astrbot 标记检查，
#    launcher 实例的 core/ 下没有 .astrbot 标记，cmd_conf._load_config 会拒）
setter = (
    "import os, json\n"
    "from astrbot.core.utils.auth_password import (\n"
    "    hash_dashboard_password, hash_md5_dashboard_password)\n"
    "cfg_path = os.environ['CFG']\n"
    "pw = os.environ['NEWPWD']\n"
    "with open(cfg_path, encoding='utf-8-sig') as f:\n"
    "    cfg = json.load(f)\n"
    "db = cfg.setdefault('dashboard', {})\n"
    "db['pbkdf2_password'] = hash_dashboard_password(pw)\n"
    "db['password'] = hash_md5_dashboard_password(pw)\n"
    "db['password_storage_upgraded'] = True\n"
    "db['password_change_required'] = False\n"
    "with open(cfg_path, 'w', encoding='utf-8-sig') as f:\n"
    "    json.dump(cfg, f, ensure_ascii=False, indent=2)\n"
    "print('PASSWORD_SET_OK')\n"
)
cmd = (
    f"cd {BASE} && "
    f"CFG={ROOT}/data/cmd_config.json NEWPWD={NEW_PWD} {BASE}/venv/bin/python -c {q(setter)}"
)
rc, out, err = run(cmd, timeout=120)
print("[set]", out.strip(), err.strip(), f"rc={rc}")

# 3) 验证：读取 dashboard 块关键字段
verify = (
    "import json\n"
    f"p = '{ROOT}/data/cmd_config.json'\n"
    "d = json.load(open(p, encoding='utf-8-sig'))\n"
    "db = d.get('dashboard', {})\n"
    "print('username:', db.get('username'))\n"
    "print('has_pbkdf2:', bool(db.get('pbkdf2_password')))\n"
    "print('change_required:', db.get('password_change_required'))\n"
)
rc, out, err = run(f"cd {BASE} && {BASE}/venv/bin/python -c {q(verify)}", timeout=60)
print("[verify]\n" + out.strip())
if err.strip():
    print("[verify-err]", err.strip())
