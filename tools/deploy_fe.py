#!/usr/bin/env python3
import paramiko, os, sys, stat

HOST = "192.168.10.107"
PORT = 22
USER = "mulq"
PASS = "162832"
LOCAL_DIST = r"C:/Users/慕洛清Mulq/Desktop/Agent_Desktop/AstrBot_WebManager/dist"
REMOTE_DIST = "/var/www/astrbot-web/dist"

def connect():
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(HOST, PORT, USER, PASS, timeout=30, allow_agent=False, look_for_keys=False)
    return c

def ensure_dir(sftp, path):
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

def clear_remote_dir(sftp, path):
    try:
        sftp.stat(path)
    except IOError:
        return
    # remove all entries under path
    for entry in sftp.listdir_attr(path):
        entry_path = f"{path}/{entry.filename}"
        if stat.S_ISDIR(entry.st_mode):
            clear_remote_dir(sftp, entry_path)
            sftp.rmdir(entry_path)
        else:
            sftp.remove(entry_path)

def put_dir(local, remote):
    c = connect()
    sftp = c.open_sftp()
    ensure_dir(sftp, remote)
    clear_remote_dir(sftp, remote)
    for root, dirs, files in os.walk(local):
        rel = os.path.relpath(root, local)
        rr = remote if rel == "." else remote + "/" + rel.replace(os.sep, "/")
        ensure_dir(sftp, rr)
        for f in files:
            lp = os.path.join(root, f)
            rp = rr + "/" + f
            sftp.put(lp, rp)
            print(f"[put] {lp} -> {rp}")
    sftp.close()
    c.close()
    print("[done] frontend deployed to", REMOTE_DIST)

if __name__ == "__main__":
    put_dir(LOCAL_DIST, REMOTE_DIST)
