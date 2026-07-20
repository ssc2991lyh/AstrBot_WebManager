import paramiko, sys, os, time

HOST="192.168.10.107"; USER="mulq"; PASS="162832"
LOCAL=r"C:\Users\慕洛清Mulq\Desktop\Agent_Desktop\AstrBot_WebManager\src-tauri\src\file_manager.rs"
REMOTE="astrbot_build/src-tauri/src/file_manager.rs"

c=paramiko.SSHClient(); c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect(HOST,username=USER,password=PASS,timeout=30,allow_agent=False,look_for_keys=False)

# 1) 上传修正后的 file_manager.rs
sftp=c.open_sftp()
sftp.put(LOCAL, REMOTE)
sftp.close()
print("uploaded file_manager.rs ->", REMOTE, flush=True)

# 2) 同步阻塞编译（远端 shell 一直被占用）
build_cmd=(
"source $HOME/.cargo/env; "
"cd ~/astrbot_build/src-tauri; "
"export CARGO_NET_GIT_FETCH_WITH_CLI=true; "
"echo BUILD_START $(date -u) > /tmp/build_state.log; "
"cargo build --release > /tmp/build.log 2>&1; "
"echo BUILD_EXIT_$? $(date -u) >> /tmp/build_state.log; "
"ls -la target/release/astrbot-launcher >> /tmp/build_state.log"
)
stdin,stdout,stderr=c.exec_command(build_cmd, timeout=1800, get_pty=True)
out=stdout.read().decode(errors='replace')
err=stderr.read().decode(errors='replace')
print(out)
if err.strip(): print("[err]", err)
c.close()
print("DONE")
