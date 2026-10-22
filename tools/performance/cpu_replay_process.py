"""Run one CPU phase; observe real stop statuses and never resume a stopped sample."""
import json
import os
from pathlib import Path
try:
    import resource
except ImportError:
    resource = None
import subprocess
import time
from datetime import datetime, timezone


def run(command, log_path, *, timeout=120):
    if resource is None or not hasattr(os, 'WCONTINUED'):
        raise RuntimeError('CPU lifecycle collection requires POSIX wait statuses and rusage')
    log_path = Path(log_path)
    metadata = log_path.with_suffix('.run.json')
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.monotonic()
    record = {'started_utc': datetime.now(timezone.utc).isoformat(),
              'load_start': os.getloadavg(), 'events': [], 'timing_valid': False}
    with log_path.open('w') as output:
        child = subprocess.Popen(command, stdout=output, stderr=subprocess.STDOUT,
                                 stdin=subprocess.DEVNULL, start_new_session=True)
        record['pid'] = child.pid
        failure = None
        try:
            while True:
                pid, status = os.waitpid(child.pid, os.WNOHANG | os.WUNTRACED | os.WCONTINUED)
                if pid:
                    if os.WIFSTOPPED(status):
                        record['events'].append({'kind': 'stopped', 'signal': os.WSTOPSIG(status),
                                                 'elapsed_seconds': time.monotonic()-started})
                        failure = 'CPU phase stopped externally; timing invalid, no automatic resume'
                        break
                    if os.WIFCONTINUED(status):
                        record['events'].append({'kind': 'continued'})
                        failure = 'CPU phase continued externally; timing invalid'
                        break
                    if os.WIFEXITED(status) or os.WIFSIGNALED(status):
                        child.returncode = os.waitstatus_to_exitcode(status)
                        break
                if time.monotonic()-started > timeout:
                    failure = 'CPU phase timeout'
                    break
                time.sleep(.02)
        finally:
            if child.returncode is None:
                # Kill only this owned child, even if stopped; do not send SIGCONT.
                child.kill()
                child.wait()
            after = resource.getrusage(resource.RUSAGE_CHILDREN)
            record.update(returncode=child.returncode, elapsed_seconds=time.monotonic()-started,
                          load_end=os.getloadavg(), failure=failure,
                          child_user_cpu_seconds=after.ru_utime-before.ru_utime,
                          child_system_cpu_seconds=after.ru_stime-before.ru_stime,
                          voluntary_context_switches=after.ru_nvcsw-before.ru_nvcsw,
                          involuntary_context_switches=after.ru_nivcsw-before.ru_nivcsw,
                          timing_valid=failure is None and child.returncode == 0)
            metadata.write_text(json.dumps(record, indent=2)+'\n')
    if failure:
        raise RuntimeError(failure)
    if child.returncode:
        raise subprocess.CalledProcessError(child.returncode, command)
    return record
