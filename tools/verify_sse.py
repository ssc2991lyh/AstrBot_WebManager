import paramiko, os

HOST = os.environ.get('SSH_HOST', '192.168.10.107')
USER = os.environ.get('SSH_USER', 'mulq')
PASS = os.environ.get('SSH_PASS', '162832')
ASTRBOT = 'd72de757-1b62-4dfd-b0ef-129f9375b816'  # AstrBot (stopped)

script = f'''
set -e
echo {PASS} | sudo -S rm -f /tmp/sse_cap.txt /tmp/sse.txt
curl -sN http://127.0.0.1:6190/api/events > /tmp/sse_cap.txt 2>&1 &
CURL_PID=$!
sleep 3
echo "=== start AstrBot ==="
curl -s --max-time 200 -X POST http://127.0.0.1:6190/api/start_instance -H 'Content-Type: application/json' -d '{{"instance_id":"{ASTRBOT}"}}'
echo
sleep 40
kill $CURL_PID 2>/dev/null || true
echo "=== CAPTURED EVENTS (wc -l) ==="
wc -l /tmp/sse_cap.txt
echo "=== head of capture ==="
head -c 2500 /tmp/sse_cap.txt
echo
echo "=== stop AstrBot to restore state ==="
curl -s --max-time 30 -X POST http://127.0.0.1:6190/api/stop_instance -H 'Content-Type: application/json' -d '{{"instance_id":"{ASTRBOT}"}}'
echo
'''

client = paramiko.SSHClient()
client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
client.connect(HOST, username=USER, password=PASS, timeout=30)

i, o, e = client.exec_command(script, timeout=300)
print(o.read().decode('utf-8', errors='replace'))
print('ERR:', e.read().decode('utf-8', errors='replace')[:500])
client.close()
print('DONE')
