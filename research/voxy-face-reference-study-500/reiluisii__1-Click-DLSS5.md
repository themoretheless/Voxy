# Facial rendering source scan: reiluisii/1-Click-DLSS5

Repository: https://github.com/reiluisii/1-Click-DLSS5

Pinned source revision: 0c6ad6106727dd5160426e4a449d32eb4fabfabc

Discovery topic: DLSS

Description: Universal Steam-style Game Center & 1-Click Neural DLSS 5 Injector for ALL PC Games (DX11/DX12/Vulkan) & All NVIDIA GeForce RTX 20, 30, 40 & 50 Series. Featuring 100% Native DLAA Feeder Mode, OptiScaler Bridge & Direct Injection.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"sss": ["core/payload/feeder/shaders/lumenite_SSSR.fx"], "shader": ["core/payload/feeder/shaders/CAS.fx", "core/payload/feeder/shaders/DLSS5_Feed.fx", "core/payload/feeder/shaders/DrawText.fxh", "core/payload/feeder/shaders/ReShade.fxh", "core/payload/feeder/shaders/ReShadeUI.fxh", "core/payload/feeder/shaders/Splitscreen.fx", "core/payload/feeder/shaders/Tonemap.fx", "core/payload/feeder/shaders/Vibrance.fx", "core/payload/feeder/shaders/include/lumenite_ColorManagement.fxh", "core/payload/feeder/shaders/include/lumenite_Compute.fxh", "core/payload/feeder/shaders/include/lumenite_Helpers.fxh", "core/payload/feeder/shaders/include/lumenite_Projections.fxh", "core/payload/feeder/shaders/lumenite_AnamorphicBloom.fx", "core/payload/feeder/shaders/lumenite_Kernel.fx", "core/payload/feeder/shaders/lumenite_LSAO.fx", "core/payload/feeder/shaders/lumenite_QuantAO.fx", "core/payload/feeder/shaders/lumenite_QuantMotion.fx", "core/payload/feeder/shaders/lumenite_RTAO.fx", "core/payload/feeder/shaders/lumenite_SSSR.fx", "core/payload/feeder/shaders/lumenite_TRAA.fx"], "dlss": ["1-Click-DLSS5.exe", "core/1-Click-DLSS5.ps1", "core/engine/DLSS5-Detection.ps1", "core/engine/DLSS5-Journal.ps1", "core/engine/DLSS5-PeEngine.ps1", "core/engine/DLSS5-Pipeline.ps1", "core/payload/feeder/dlss5-feed.addon32", "core/payload/feeder/dlss5-feed.addon64", "core/payload/feeder/dlss5-feed.cfg", "core/payload/feeder/host64/dlss5-feed-host64.exe", "core/payload/feeder/shaders/DLSS5_Feed.fx", "core/payload/renodx-dlss5.addon64", "core/payload/sl.dlss_nr.dll"]}

## Source file: src/Launcher.cs

{
  "path": "src/Launcher.cs",
  "blob_sha": "3fde4515033a1439dcd3da03c766f3b89a0f7106",
  "sha256": "5d46d4ecaa9925603e81476e8f1c85bcea02213ad3bf182dee11fe458df0a316",
  "lines": 116,
  "bytes": 4994,
  "selection_score": 0,
  "keyword_line_counts": {
    "sss": 1,
    "gum": 1,
    "dlss": 17,
    "rig": 1
  },
  "url": "https://github.com/reiluisii/1-Click-DLSS5/blob/0c6ad6106727dd5160426e4a449d32eb4fabfabc/src/Launcher.cs",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "static int Main(string[] args)"
    }
  ]
}

```text

using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Windows.Forms;

[assembly: AssemblyTitle("1 Click DLSS 5")]
[assembly: AssemblyDescription("Universal Neural Control Center • DLSS 5 (DLSS-NR) Installer")]
[assembly: AssemblyConfiguration("")]
[assembly: AssemblyCompany("1 Click DLSS 5 Project")]
[assembly: AssemblyProduct("1 Click DLSS 5")]
[assembly: AssemblyCopyright("Copyright (c) 2026 MIT License")]
[assembly: AssemblyTrademark("DLSS 5 Neural Control Center")]
[assembly: AssemblyVersion("3.0.2.0")]
[assembly: AssemblyFileVersion("3.0.2.0")]

namespace OneClickDLSS5
{
    static class Program
    {
        [STAThread]
        static int Main(string[] args)
        {

            string baseDir = AppDomain.CurrentDomain.BaseDirectory;
            string scriptPath = Path.Combine(baseDir, @"core\1-Click-DLSS5.ps1");
            bool isPt = System.Globalization.CultureInfo.CurrentUICulture.TwoLetterISOLanguageName.Equals("pt", StringComparison.OrdinalIgnoreCase);

            if (!File.Exists(scriptPath))
            {
                string altScript = Path.Combine(baseDir, "1-Click-DLSS5.ps1");
                if (File.Exists(altScript))
                {
                    scriptPath = altScript;
                }
                else
                {
                    string notFoundMsg = isPt 
                        ? "Não foi possível localizar o script principal do 1 Click DLSS 5 em:\n" + scriptPath 
                        : "Could not locate the main 1 Click DLSS 5 script at:\n" + scriptPath;
                    string notFoundTitle = isPt ? "1 Click DLSS 5 - Arquivo Não Encontrado" : "1 Click DLSS 5 - Script Not Found";
                    MessageBox.Show(
                        notFoundMsg,
                        notFoundTitle,
                        MessageBoxButtons.OK,
                        MessageBoxIcon.Error);
                    return 1;
                }
            }

            // Seleciona o interpretador PowerShell mais recente disponível
            string psExe = "powershell.exe";
            string[] pwshCandidates = new string[]
            {
                Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), @"PowerShell\7\pwsh.exe"),
                Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), @"Microsoft\PowerShell\7\pwsh.exe"),
                Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86), @"PowerShell\7\pwsh.exe")
            };
            foreach (string cand in pwshCandidates)
            {
                if (!string.IsNullOrEmpty(cand) && File.Exists(cand))
                {
                    psExe = cand;
                    break;
                }
            }

            try
            {
                ProcessStartInfo psi = new ProcessStartInfo();
                psi.FileName = psExe;
                psi.Arguments = "-NoProfile -ExecutionPolicy Bypass -STA -File \"" + scriptPath + "\"";
                psi.WorkingDirectory = baseDir;
                psi.UseShellExecute = false;
                psi.CreateNoWindow = true;
                psi.WindowStyle = ProcessWindowStyle.Normal;
                psi.RedirectStandardError = true;
                psi.StandardErrorEncoding = System.Text.Encoding.UTF8;

                using (Process proc = Process.Start(psi))
                {
                    string stderr = proc.StandardError.ReadToEnd();
                    proc.WaitForExit();

                    if (proc.ExitCode != 0 && !string.IsNullOrEmpty(stderr) && !stderr.Contains("OperationCanceledException"))
                    {
                        string errMsg = isPt
                            ? "Ocorreu um erro durante a inicialização do 1 Click DLSS 5:\n\n" + stderr
                            : "An error occurred during 1 Click DLSS 5 startup:\n\n" + stderr;
                        string errTitle = isPt ? "1 Click DLSS 5 - Erro" : "1 Click DLSS 5 - Error";
                        MessageBox.Show(
                            errMsg,
                            errTitle,
                            MessageBoxButtons.OK,
                            MessageBoxIcon.Warning);
                    }
                    return proc.ExitCode;
                }
            }
            catch (Exception ex)
            {
                string failMsg = isPt
                    ? "Falha ao iniciar o 1 Click DLSS 5:\n\n" + ex.Message
                    : "Failed to launch 1 Click DLSS 5:\n\n" + ex.Message;
                string failTitle = isPt ? "1 Click DLSS 5 - Falha Crítica" : "1 Click DLSS 5 - Launch Failure";
                MessageBox.Show(
                    failMsg,
                    failTitle,
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error);
                return 1;
            }
        }
    }
}


```

## Source file: tools/build-release.py

{
  "path": "tools/build-release.py",
  "blob_sha": "fb31e0a2282940a1b422d875827ecc9c4a0a576a",
  "sha256": "f32c5f92a2a35b2c3acd1d4744869a57dbd5b26ad39120f19241f7cc357c627e",
  "lines": 109,
  "bytes": 4522,
  "selection_score": 0,
  "keyword_line_counts": {
    "shader": 4,
    "dlss": 7
  },
  "url": "https://github.com/reiluisii/1-Click-DLSS5/blob/0c6ad6106727dd5160426e4a449d32eb4fabfabc/tools/build-release.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def main():"
    }
  ]
}

```text

import os
import sys
import zipfile
import json
import re

def main():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    dist_dir = os.path.dirname(repo_root)
    zip_output = os.path.join(dist_dir, "1-Click-DLSS5-v3.0.2.zip")

    print(f"=== 1-Click DLSS 5 Release Packager (v3.0.2) ===")
    print(f"Source: {repo_root}")
    print(f"Destination: {zip_output}")

    # 1. Sanity Checks
    log_file = os.path.join(repo_root, "core", "1-Click-DLSS5.log")
    if os.path.exists(log_file) and os.path.getsize(log_file) > 0:
        print("[WARN] Truncating core/1-Click-DLSS5.log to 0 bytes before packaging...")
        open(log_file, "w").close()

    config_file = os.path.join(repo_root, "core", "assets", "config.json")
    if os.path.exists(config_file):
        with open(config_file, "r", encoding="utf-8") as f:
            cfg = json.load(f)
        if cfg.get("LastSelectedGamePath") or cfg.get("CustomGamePaths"):
            print("[WARN] Resetting core/assets/config.json to clean defaults...")
            cfg["LastSelectedGamePath"] = ""
            cfg["CustomGamePaths"] = []
            with open(config_file, "w", encoding="utf-8") as f:
                json.dump(cfg, f, indent=4)

    cache_file = os.path.join(repo_root, "core", "assets", "games_cache.json")
    if os.path.exists(cache_file):
        with open(cache_file, "r", encoding="utf-8") as f:
            cache = json.load(f)
        if len(cache) > 0:
            print("[WARN] Resetting core/assets/games_cache.json to empty list...")
            with open(cache_file, "w", encoding="utf-8") as f:
                json.dump([], f, indent=4)

    # 2. Shader Checks
    shader_dir = os.path.join(repo_root, "core", "payload", "feeder", "shaders")
    forbidden = ["FXAA.fx", "FXAA.fxh", "SMAA.fx", "SMAA.fxh"]
    for fb in forbidden:
        p = os.path.join(shader_dir, fb)
        if os.path.exists(p):
            print(f"[ERROR] Forbidden shader {fb} still present! Removing...")
            os.remove(p)

    # 3. Packaging
    exclude_dirs = {".git", ".vs", ".vscode", "scratch", "bin", "obj", "__pycache__"}
    exclude_files = {
        "1-Click-DLSS5.log",
        ".gitattributes",
        ".gitignore",
        "desktop.ini",
        "Thumbs.db"
    }

    if os.path.exists(zip_output):
        os.remove(zip_output)

    added_files = []
    with zipfile.ZipFile(zip_output, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as zipf:
        for root, dirs, files in os.walk(repo_root):
            dirs[:] = [d for d in dirs if d not in exclude_dirs]
            for file in files:
                if file in exclude_files or file.endswith(".tmp") or file.endswith(".zip"):
                    continue
                full_path = os.path.join(root, file)
                rel_path = os.path.relpath(full_path, repo_root)
                
                # Archive name has folder prefix 1-Click-DLSS5/
                archive_name = ("1-Click-DLSS5/" + rel_path).replace("\\", "/")
                zipf.write(full_path, archive_name)
                added_files.append(archive_name)

    print(f"\n[OK] Release zip generated: {zip_output}")
    print(f"[OK] Total files packaged: {len(added_files)}")
    print(f"[OK] Package size: {os.path.getsize(zip_output):,} bytes ({os.path.getsize(zip_output)/(1024*1024):.2f} MB)")

    # 4. Post-Packaging Audit
    print("\n=== Post-Packaging Security & Privacy Audit ===")
    leak_patterns = [
        re.compile(r"Luis\s+Eduardo", re.IGNORECASE),
        re.compile(r"C:\\OneDrive", re.IGNORECASE),
        re.compile(r"E:\\JOGOS", re.IGNORECASE),
        re.compile(r"FreshWomen", re.IGNORECASE)
    ]
    leaks_found = 0
    with zipfile.ZipFile(zip_output, "r") as zipf:
        for info in zipf.infolist():
            # Check text files for personal data
            if any(info.filename.endswith(ext) for ext in [".json", ".txt", ".md", ".log", ".ini", ".cfg", ".conf"]):
                content = zipf.read(info.filename).decode("utf-8", errors="ignore")
                for pat in leak_patterns:
                    if pat.search(content):
                        print(f"[FAIL] Privacy leak in {info.filename}: matched {pat.pattern}")
                        leaks_found += 1

    if leaks_found > 0:
        print(f"\n[ERROR] Audit failed with {leaks_found} potential leaks!")
        sys.exit(1)
    else:
        print("[PASS] Audit complete! Zero personal data, zero developer logs found in zip.")

if __name__ == "__main__":
    main()


```
