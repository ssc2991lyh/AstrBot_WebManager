import paramiko, os, sys

HOST = "192.168.10.107"
USER = "mulq"
PASS = "162832"
LOCAL = r"C:\Users\慕洛清Mulq\Desktop\Agent_Desktop\AstrBot_WebManager\src-tauri"
REMOTE_BASE = "/home/mulq/astrbot_build/src-tauri"

SKIP_DIRS = {"target", "node_modules", ".git"}

c = paramiko.SSHClient()
c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect(HOST, username=USER, password=PASS, timeout=30, allow_agent=False, look_for_keys=False)
sftp = c.open_sftp()


def remote_mkdir(path):
    try:
        sftp.stat(path)
    except FileNotFoundError:
        parent = os.path.dirname(path)
        if parent and parent != path:
            remote_mkdir(parent)
        try:
            sftp.mkdir(path)
        except IOError:
            pass


def upload(local, remote):
    for entry in os.scandir(local):
        if entry.name in SKIP_DIRS:
            continue
        l = entry.path
        r = remote + "/" + entry.name
        if entry.is_dir():
            remote_mkdir(r)
            upload(l, r)
        else:
            remote_mkdir(os.path.dirname(r))
            with open(l, "rb") as f:
                sftp.put(l, r)
            print("PUT", r)


remote_mkdir(REMOTE_BASE)
upload(LOCAL, REMOTE_BASE)
sftp.close()
c.close()
print("UPLOAD_DONE")
