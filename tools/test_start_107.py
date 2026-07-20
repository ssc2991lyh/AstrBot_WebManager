import paramiko, os, json, time

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
    # Start AstrBot instance (the one that failed in screenshot)
    print("=== Start AstrBot instance ===")
    out, err, code = run("curl -s --max-time 300 -X POST http://127.0.0.1:6190/api/start_instance -H 'Content-Type: application/json' -d '{\"instance_id\":\"d72de757-1b62-4dfd-b0ef-129f9375b816\"}'", timeout=310)
    print(out)
    if err.strip(): print("ERR:", err)
    print("EXIT:", code)
    time.sleep(2)

    # Check status
    print("\n=== App snapshot ===")
    out, err, code = run("curl -s --max-time 30 -X POST http://127.0.0.1:6190/api/get_app_snapshot -H 'Content-Type: application/json' -d '{}' | python3 -m json.tool 2>/dev/null || true", timeout=40)
    print(out[:2000])
    if err.strip(): print("ERR:", err)
