from pathlib import Path
import argparse,subprocess,shutil
ap=argparse.ArgumentParser();ap.add_argument('--runtime',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);a=ap.parse_args();out=a.output.resolve()
if out.exists():raise SystemExit('Choose a fresh absent output')
out.mkdir(parents=True);(out/'cloud-system-proof').symlink_to(a.runtime.resolve(),target_is_directory=True);task=out/'cloud-forcing-review';task.mkdir();shutil.copy2(Path(__file__).with_name('reproduce.mjs'),task/'reproduce.mjs');subprocess.run(['node',str(task/'reproduce.mjs')],check=True)
