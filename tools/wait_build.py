import paramiko, time

HOST = "192.168.10.107"
USER = "mulq"
PASS = "162832"

BIN = "/home/mulq/astrbot_build/src-tauri/target/release/astrbot-launcher"

state = "WAIT_START"
while True:
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    try:
        c.connect(HOST, username=USER, password=PASS, timeout=15, allow_agent=False, look_for_keys=False)
        chk = (
            "if pgrep -f 'cargo build' >/dev/null || pgrep -f 'rustc' >/dev/null; then echo BUILDING; "
            "elif [ -f %s ]; then echo BUILD_DONE; "
            "else echo NOTSTARTED; fi" % BIN
        )
        _, out, _ = c.exec_command(chk, timeout=20, get_pty=True)
        txt = out.read().decode(errors="replace").strip()
        if state == "WAIT_START" and txt == "BUILDING":
            state = "BUILDING"
        if state == "BUILDING" and txt == "BUILD_DONE":
            print(time.strftime("%H:%M:%S"), "BUILD_DONE")
            break
        if txt == "BUILD_DONE" and state != "WAIT_START":
            print(time.strftime("%H:%M:%S"), "BUILD_DONE(early)")
            break
        print(time.strftime("%H:%M:%S"), "state=%s raw=%s" % (state, txt[:80]))
    except Exception as e:
        print(time.strftime("%H:%M:%S"), "check err:", e)
    finally:
        c.close()
    time.sleep(20)
