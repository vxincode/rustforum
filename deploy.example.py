#!/usr/bin/env python3
"""One-click deployment script - Upload code to server and rebuild/restart"""

import paramiko
import os
import sys
import time

# ============ Configuration (modify before use) ============
SERVER_IP = "your-server-ip"
SERVER_USER = "root"
SERVER_PASS = "your-password"
REMOTE_DIR = "/path/to/rustforum"
SERVICE_NAME = "rustforum.service"
# =============================

# Local project directory (same level as Cargo.toml)
LOCAL_DIR = "."

# Directories and files to upload
UPLOAD_DIRS = ["src", "static", "migrations"]
UPLOAD_FILES = ["Cargo.toml", "Cargo.lock"]


def ssh_connect():
    ssh = paramiko.SSHClient()
    ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    print(f"Connecting to {SERVER_IP}...")
    ssh.connect(SERVER_IP, username=SERVER_USER, password=SERVER_PASS, timeout=15)
    return ssh


def ensure_remote_dir(sftp, remote_dir):
    parts = [p for p in remote_dir.split("/") if p]
    for i in range(1, len(parts) + 1):
        path = "/" + "/".join(parts[:i])
        try:
            sftp.stat(path)
        except FileNotFoundError:
            sftp.mkdir(path)


def upload_files(sftp, ssh):
    print("\n[1/3] Uploading files...")
    total = 0

    for d in UPLOAD_DIRS:
        local_dir = os.path.join(LOCAL_DIR, d)
        if not os.path.isdir(local_dir):
            print(f"  Skip: {d}/ (not found)")
            continue
        count = 0
        for root, dirs, files in os.walk(local_dir):
            for f in files:
                local_path = os.path.join(root, f)
                rel_path = os.path.relpath(local_path, LOCAL_DIR).replace("\\", "/")
                remote_path = REMOTE_DIR + "/" + rel_path
                ensure_remote_dir(sftp, os.path.dirname(remote_path))
                sftp.put(local_path, remote_path)
                count += 1
        print(f"  {d}/ -> {count} files")
        total += count

    for f in UPLOAD_FILES:
        local_path = os.path.join(LOCAL_DIR, f)
        if os.path.exists(local_path):
            sftp.put(local_path, REMOTE_DIR + "/" + f)
            print(f"  {f}")
            total += 1

    ssh.exec_command(f"chown -R www:www {REMOTE_DIR}/src {REMOTE_DIR}/static {REMOTE_DIR}/migrations")
    print(f"  Upload complete: {total} files")


def build_project(ssh):
    print("\n[2/3] Building project (release)...")
    stdin, stdout, stderr = ssh.exec_command(
        f"cd {REMOTE_DIR} && cargo build --release 2>&1",
        timeout=600
    )
    output = stdout.read().decode()

    lines = output.strip().split("\n")
    for line in lines[-3:]:
        print(f"  {line}")

    if "error" in output.lower():
        print("  Build failed! Check errors above.")
        sys.exit(1)
    print("  Build succeeded")


def restart_service(ssh):
    print(f"\n[3/3] Restarting service {SERVICE_NAME}...")
    stdin, stdout, stderr = ssh.exec_command(f"systemctl restart {SERVICE_NAME}")
    stdout.read()
    time.sleep(2)

    stdin, stdout, stderr = ssh.exec_command(f"systemctl status {SERVICE_NAME}")
    status = stdout.read().decode()

    if "active (running)" in status:
        for line in status.split("\n"):
            if "Active:" in line or "Listening" in line:
                print(f"  {line.strip()}")
        print("\nDeployment complete!")
    else:
        print("  Service status error:")
        print(status)
        sys.exit(1)


def main():
    start = time.time()
    print("=" * 50)
    print("  RustForum - One-click Deployment")
    print(f"  Local:  {LOCAL_DIR}")
    print(f"  Remote: {REMOTE_DIR}")
    print("=" * 50)

    ssh = ssh_connect()
    sftp = ssh.open_sftp()

    try:
        upload_files(sftp, ssh)
        build_project(ssh)
        restart_service(ssh)
    except Exception as e:
        print(f"\nDeployment failed: {e}")
        sys.exit(1)
    finally:
        sftp.close()
        ssh.close()

    elapsed = time.time() - start
    print(f"  Time: {elapsed:.1f}s")


if __name__ == "__main__":
    main()
