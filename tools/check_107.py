import paramiko, os, sys, json, time

HOST = os.environ.get('HOST', '192.168.10.107')
USER = os.environ.get('USER', 'mulq')
PASS = os.environ.get('PASS', '162832')

def run(cmd, sudo=False):
    client = paramiko.SSHClient()
    client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    client.connect(HOST, username=USER, password=PASS, timeout=30)
    if sudo:
        cmd = f"echo {PASS} | sudo -S {cmd}"
    stdin, stdout, stderr = client.exec_command(cmd, timeout=120)
    out = stdout.read().decode('utf-8', errors='replace')
    err = stderr.read().decode('utf-8', errors='replace')
    code = stdout.channel.recv_exit_status()
    client.close()
    return out, err, code

if __name__ == '__main__':
    cmds = [
        ("systemctl status astrbot-launcher --no-pager -l", False),
        ("ss -tlnp | grep -E '6190|6180|6185|1635|1636|1637'", False),
        ("ps aux | grep -i astrbot | grep -v grep", False),
        ("ls -la ~/.astrbot_launcher/instances/", False),
        ("ls -la ~/.astrbot_launcher/", False),
        ("journalctl -u astrbot-launcher -n 100 --no-pager", False),
    ]
    for cmd, sudo in cmds:
        print(f"\n=== {cmd} {'(sudo)' if sudo else ''} ===")
        out, err, code = run(cmd, sudo=sudo)
        print(out)
        if err.strip():
            print(f"STDERR: {err}")
        print(f"EXIT: {code}")
