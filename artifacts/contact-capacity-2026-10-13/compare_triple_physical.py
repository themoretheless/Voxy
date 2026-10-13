import os, re, json, subprocess, statistics
from pathlib import Path
root=Path(__file__).resolve().parent
exe="/tmp/voxy-face-refit-20261011/release/deps/voxy_app-53370c422bb1c776"
env=os.environ.copy();env["VOXY_HAIR_QR_INPUT_FIXTURE"]="/Users/themoretheless/Documents/ChatGPT/Voxy/artifacts/hair-contact-divergence-diagnostics-2026-10-09/newton-addition-qr-input.bin"
results=[]
for index,parallel in enumerate([False,True,True,False]):
 run_env=env.copy();run_env.pop("VOXY_HAIR_TRIPLE_PARALLEL",None)
 if parallel:run_env["VOXY_HAIR_TRIPLE_PARALLEL"]="1"
 result=subprocess.run([exe,"gpu_triple_active_admits_captured_original_physics","--ignored","--nocapture"],env=run_env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 log=root/f"triple-physical-comparison-{index}.log";log.write_text(result.stdout)
 match=re.search(r"accelerated=true submissions=1 elapsed_ms=([0-9.]+) error=None",result.stdout)
 record=dict(index=index,parallel=parallel,exit_code=result.returncode,elapsed_ms=float(match.group(1)) if match else None,log=log.name);results.append(record)
 (root/"triple-physical-comparison.json").write_text(json.dumps(dict(scope="same captured original physical operator; wall time; not FPS",runs=results),indent=2))
 print(record,flush=True)
 if result.returncode or not match:raise SystemExit("physical gate failed")
serial=statistics.median(r["elapsed_ms"] for r in results if not r["parallel"])
parallel=statistics.median(r["elapsed_ms"] for r in results if r["parallel"])
report=dict(scope="same captured original physical operator; wall time; not whole trajectory or FPS",runs=results,serial_median_ms=serial,parallel_median_ms=parallel,speedup=serial/parallel)
(root/"triple-physical-comparison.json").write_text(json.dumps(report,indent=2));print(report,flush=True)
