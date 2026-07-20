import paramiko, os, sys, json, time

HOST = os.environ.get('HOST', '192.168.10.107')
USER = os.environ.get('USER', 'mulq')
PASS = os.environ.get('PASS', '162832')

def run(cmd, sudo=False, timeout=120):
    client = paramiko.SSHClient()
    client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    client.connect(HOST, username=USER, password=PASS, timeout=30)
    if sudo:
        cmd = f"echo {PASS} | sudo -S {cmd}"
    stdin, stdout, stderr = client.exec_command(cmd, timeout=timeout)
    out = stdout.read().decode('utf-8', errors='replace')
    err = stderr.read().decode('utf-8', errors='replace')
    code = stdout.channel.recv_exit_status()
    client.close()
    return out, err, code

if __name__ == '__main__':
    cmds = [
        ("curl -L -I --max-time 20 'https://gh-proxy.org/https://github.com/AstrBotDevs/AstrBot/releases/download/v4.26.5/AstrBot-v4.26.5-dashboard.zip' 2>&1 | head -40", False, 60),
        ("curl -L -I --max-time 20 'https://github.com/AstrBotDevs/AstrBot/releases/download/v4.26.5/AstrBot-v4.26.5-dashboard.zip' 2>&1 | head -40", False, 60),
        ("curl -L -I --max-time 20 'https://astrbot-registry.soulter.top/download/astrbot-dashboard/v4.26.5/dist.zip' 2>&1 | head -40", False, 60),
        ("ls -la ~/.astrbot_launcher/instances/4c6d9a97-f127-442b-8f8e-ebc0c00cacc5/", False, 30),
        ("ls -la ~/.astrbot_launcher/instances/4c6d9a97-f127-442b-8f8e-ebc0c00cacc5/core/ 2>/dev/null || echo 'no core'", False, 30),
        ("ls -la ~/.astrbot_launcher/instances/4c6d9a97-f127-442b-8f8e-ebc0c00cacc5/data/ 2>/dev/null || echo 'no data'", False, 30),
        ("ls -la ~/.astrbot_launcher/instances/4c6d9a97-f127-442b-8f8e-ebc0c00cacc5/data/dist/ 2>/dev/null || echo 'no dist'", False, 30),
        ("ls -la ~/.astrbot_launcher/instances/17406780-5c25-412e-b757-51666b1ad029/data/dist/ 2>/dev/null || echo 'no dist'", False, 30),
        ("ls -la ~/.astrbot_launcher/instances/d72de757-1b62-4dfd-b0ef-129f9375b816/data/dist/ 2>/dev/null || echo 'no dist'", False, 30),
    ]
    for cmd, sudo, timeout in cmds:
        print(f"\n=== {cmd} ===")
        out, err, code = run(cmd, sudo=sudo, timeout=timeout)
        print(out)
        if err.strip():
            print(f"STDERR: {err}")
        print(f"EXIT: {code}")
