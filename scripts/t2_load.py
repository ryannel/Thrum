"""Read-only host load and disk deltas alongside a T2 measurement interval."""
import os
from pathlib import Path
import subprocess
import time


class LoadRecord:
    """Sampler is outside the engine; its overhead is present in both arms."""
    def __init__(self, output):
        self.output = Path(output)
        self.begin = time.monotonic_ns()
        self.before = list(os.getloadavg())
        self.stream = (self.output / "system-iostat.txt").open("wb")
        # First row averages since boot; subsequent rows average each one-second
        # interval: disk MB/s, host CPU user/system/idle and load.
        self.process = subprocess.Popen(["/usr/sbin/iostat", "-d", "-C", "-U", "-w", "1"],
                                        stdout=self.stream, stderr=subprocess.STDOUT)
        self.processes = subprocess.check_output(
            ["/bin/ps", "-axo", "pid,pcpu,comm", "-r"], text=True).splitlines()[:16]

    def finish(self):
        self.process.terminate()
        self.process.wait(timeout=5)
        self.stream.close()
        return {"before_load_1_5_15": self.before, "after_load_1_5_15": list(os.getloadavg()),
                "sample_duration_ns": time.monotonic_ns() - self.begin,
                "disk_samples": "system-iostat.txt", "disk_scope": "default devices including spool disk0; first row since boot; subsequent rows 1 s averages",
                "background_process_snapshot": self.processes,
                "process_cpu_scope": "ps lifetime CPU snapshot, not interval CPU; load averages are host-wide",
                "sampler": "iostat -d -C -U -w 1 subprocess plus one ps snapshot, same recipe in each arm"}
