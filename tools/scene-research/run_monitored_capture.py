#!/usr/bin/env python3
"""Record native capture lifecycle using real wait statuses, including stops.

Never resume or restart a stopped child. Raw elapsed time includes pauses.
Lifecycle evidence alone never qualifies frame rate or GPU isolation.
State is the last wait observation; a continued status may be absent before exit.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time


def run(command, directory, *, env=None, cwd=None):
    if not command:
        raise ValueError('a capture command is required')
    if not hasattr(os, 'WUNTRACED') or not hasattr(os, 'WCONTINUED'):
        raise RuntimeError('native lifecycle capture requires POSIX wait-status support')
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    state_path = directory / 'process.json'
    paths = [state_path, directory / 'log.txt', directory / 'process-events.jsonl']
    if any(p.exists() for p in paths):
        raise FileExistsError('capture lifecycle evidence already exists')
    # Claim fresh evidence before starting a child; never overwrite an earlier run.
    with state_path.open('x') as claimed:
        json.dump({'state': 'starting', 'command': list(command)}, claimed)
    started = time.monotonic()
    stops = continued = 0
    with paths[1].open('x') as output, paths[2].open('x', buffering=1) as events:
        child = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                 stdout=output, stderr=subprocess.STDOUT,
                                 start_new_session=True)
        state = dict(pid=child.pid, observer_pid=os.getpid(), process_group=child.pid,
                     command=list(command), state='running')

        def record(kind, **fields):
            event = dict(kind=kind, pid=child.pid, wall_time=time.time(),
                         elapsed_seconds=time.monotonic() - started, **fields)
            events.write(json.dumps(event) + '\n')
            state.update(last_event=kind, stop_events=stops, continue_events=continued,
                         elapsed_seconds=event['elapsed_seconds'],
                         timing_invalidated_by_process_stop=bool(stops or continued),
                         gpu_isolation_established=False, fps_qualified=False)
            temporary = directory / f'.process-{os.getpid()}.tmp'
            temporary.write_text(json.dumps(state, indent=2) + '\n')
            os.replace(temporary, state_path)

        def forward(signum, _frame):
            try:
                os.killpg(child.pid, signum)
            except ProcessLookupError:
                pass

        previous = {s: signal.signal(s, forward) for s in (signal.SIGINT, signal.SIGTERM)}
        record('started')
        try:
            while True:
                try:
                    _, status = os.waitpid(child.pid, os.WUNTRACED | os.WCONTINUED)
                except InterruptedError:
                    continue
                if os.WIFSTOPPED(status):
                    stops += 1
                    state['state'] = 'stopped'
                    record('stopped', signal=signal.Signals(os.WSTOPSIG(status)).name)
                elif os.WIFCONTINUED(status):
                    continued += 1
                    state['state'] = 'running'
                    record('continued')
                elif os.WIFEXITED(status) or os.WIFSIGNALED(status):
                    code = os.WEXITSTATUS(status) if os.WIFEXITED(status) else -os.WTERMSIG(status)
                    child.returncode = code
                    state.update(state='terminal', returncode=code)
                    fields = {'returncode': code}
                    if code < 0:
                        fields['signal'] = signal.Signals(-code).name
                    record('exited' if code >= 0 else 'signalled', **fields)
                    return dict(state)
        finally:
            for sig, handler in previous.items():
                signal.signal(sig, handler)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    result = run(command, args.output)
    code = result['returncode']
    raise SystemExit(code if code >= 0 else 128 - code)
