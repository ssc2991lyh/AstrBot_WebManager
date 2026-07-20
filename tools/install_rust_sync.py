import paramiko
HOST="192.168.10.107"; USER="mulq"; PASS="162832"
c=paramiko.SSHClient(); c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect(HOST,username=USER,password=PASS,timeout=30,allow_agent=False,look_for_keys=False)
cmd=(
    "export DEBIAN_FRONTEND=noninteractive; "
    "echo 162832 | sudo -S apt-get update -qq; "
    "echo 162832 | sudo -S apt-get install -y -qq build-essential pkg-config libssl-dev; "
    "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; "
    "echo RUSTUP_DONE_MARKER; "
    "~/.cargo/bin/cargo --version"
)
stdin,stdout,stderr=c.exec_command(cmd,timeout=580,get_pty=True)
print("STDOUT:\n", stdout.read().decode(errors='replace'))
print("STDERR:\n", stderr.read().decode(errors='replace'))
c.close()
print("INSTALL_SCRIPT_EXITED")
