import paramiko, os

HOST = os.environ.get('SSH_HOST', '192.168.10.107')
USER = os.environ.get('SSH_USER', 'mulq')
PASS = os.environ.get('SSH_PASS', '162832')

NGINX_CONF = """server {
    listen 80;
    server_name _;
    root /var/www/astrbot-web/dist;
    index index.html;

    location /api {
        proxy_pass http://127.0.0.1:6190;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        # SSE / 实时事件流：关闭缓冲，保持长连接
        proxy_set_header Connection "";
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 3600s;
        chunked_transfer_encoding on;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
"""

client = paramiko.SSHClient()
client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
client.connect(HOST, username=USER, password=PASS, timeout=30)

# Write config via sudo tee
cmd = f"echo {PASS} | sudo -S tee /etc/nginx/sites-available/astrbot-web > /dev/null"
stdin, stdout, stderr = client.exec_command(cmd)
stdin.write(NGINX_CONF)
stdin.channel.shutdown_write()
print('write:', stdout.read().decode().strip(), stderr.read().decode().strip())

# Test config
i, o, e = client.exec_command(f"echo {PASS} | sudo -S nginx -t", timeout=30)
print('nginx -t:', o.read().decode().strip(), e.read().decode().strip())

# Reload
i, o, e = client.exec_command(f"echo {PASS} | sudo -S systemctl reload nginx", timeout=30)
print('reload:', o.read().decode().strip(), e.read().decode().strip())

client.close()
print('DONE')
