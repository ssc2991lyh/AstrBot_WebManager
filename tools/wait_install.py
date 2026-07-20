import paramiko, time

HOST = "192.168.10.107"
USER = "mulq"
PASS = "162832"

while True:
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    try:
        c.connect(HOST, username=USER, password=PASS, timeout=15, allow_agent=False, look_for_keys=False)
        chk = (
            "if [ -f /tmp/install_state.log ]; then echo STATE_DONE; "
            "elif [ -f ~/.cargo/bin/cargo ]; then echo CARGO_READY; "
            "else echo PENDING; fi; "
            "tail -2 /tmp/rustup.log 2>/dev/null; tail -2 /tmp/apt.log 2>/dev/null"
        )
        _, out, _ = c.exec_command(chk, timeout=20, get_pty=True)
        txt = out.read().decode(errors="replace")
        print(time.strftime("%H:%M:%S"), txt.strip()[:300])
        if "STATE_DONE" in txt or "CARGO_READY" in txt:
            print("INSTALL_COMPLETE")
            break
    except Exception as e:
        print(time.strftime("%H:%M:%S"), "check err:", e)
    finally:
        c.close()
    time.sleep(20)
