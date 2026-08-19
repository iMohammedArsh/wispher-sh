import sys
import subprocess
import os
import shutil

cmd = sys.argv[1:]
if cmd and cmd[0] == "rustc":
    rustc_bin = shutil.which("rustc") or r"C:\Users\Administrator\.cargo\bin\rustc.exe"
    cmd[0] = rustc_bin

result = subprocess.run(cmd)

if result.returncode == 0:
    for i, arg in enumerate(cmd):
        if arg == "-o" and i + 1 < len(cmd):
            out_file = cmd[i + 1]
            if out_file.endswith(".exe") and "tauri-app" in out_file and "build_script" not in out_file and os.path.exists(out_file):
                # Sign generated main executable using trusted local self-signed cert
                sign_cmd = (
                    "$cert = (Get-ChildItem Cert:\\CurrentUser\\My -CodeSigningCert)[0]; "
                    f"Set-AuthenticodeSignature -Certificate $cert -FilePath '{out_file}'"
                )
                subprocess.run(["powershell", "-NoProfile", "-Command", sign_cmd], capture_output=True)

sys.exit(result.returncode)
