import paramiko

HOST = "192.168.10.107"
USER = "mulq"
PASS = "162832"

c = paramiko.SSHClient()
c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect(HOST, username=USER, password=PASS, timeout=30, allow_agent=False, look_for_keys=False)

# 同步阻塞远端编译：本地 Python 脚本本身作为 Bash 后台任务跑，远端 shell 被持续占用，进程不会被关 channel 杀掉。
cmd = (
    "export PATH=$HOME/.cargo/bin:$PATH; "
    "cd ~/astrbot_build/src-tauri && "
    "echo BUILD_START $(date) >> /tmp/build_state.log; "
    "cargo build --release >> /tmp/build.log 2>&1; "
    "echo BUILD_EXIT_$? $(date) >> /tmp/build_state.log; "
    "echo BUILD_DONE_MARKER"
)
stdin, stdout, stderr = c.exec_command(cmd, timeout=2400, get_pty=True)
print("STDOUT:", stdout.read().decode(errors="replace"))
print("STDERR:", stderr.read().decode(errors="replace"))
c.close()
print("BUILD_SCRIPT_EXITED")
