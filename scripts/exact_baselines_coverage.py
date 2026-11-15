"""Cargo-derived subtraction from the exact-baseline registry's listed paths."""
import hashlib
import json
from pathlib import Path
import shlex
import subprocess


def inside(path, folder):
    return path == folder or path.startswith(folder.rstrip('/') + '/')


def metadata(root, manifest=None):
    command = ['cargo', 'metadata', '--no-deps', '--offline', '--format-version', '1']
    if manifest:
        command += ['--manifest-path', str(manifest)]
    result = subprocess.run(command, cwd=root, capture_output=True, text=True)
    if result.returncode:
        raise ValueError('Cargo cannot load workspace: ' + result.stderr.strip())
    return json.loads(result.stdout)


class Coverage:
    def __init__(self, root, registry):
        self.root = root.resolve()
        self.listed = registry['covered_paths']
        graph = metadata(self.root)
        members = set(graph['workspace_members'])
        packages = {Path(p['manifest_path']).resolve().parent: p for p in graph['packages']}
        built = registry['built_packages']
        pending = []
        for name in built:
            matches = [folder for folder, p in packages.items() if p['name'] == name and p['id'] in members]
            if len(matches) != 1:
                raise ValueError(f'built package {name!r} is not a unique workspace member')
            pending.extend(matches)
        linked = {}
        while pending:
            folder = pending.pop()
            if folder in linked:
                continue
            if not folder.is_relative_to(self.root):
                raise ValueError(f'linked package outside repository: {folder}')
            if folder not in packages:
                extra = metadata(self.root, folder / 'Cargo.toml')
                packages.update({Path(p['manifest_path']).resolve().parent: p for p in extra['packages']})
            if folder not in packages:
                raise ValueError(f'Cargo cannot classify linked package: {folder}')
            package = packages[folder]
            linked[folder] = package
            # Declared edges include disabled optional and all platform dependencies.
            # Registry/git dependencies cannot own files tracked in this repository.
            pending.extend(Path(d['path']).resolve() for d in package['dependencies']
                           if d.get('path') and d.get('kind') != 'dev')
        self.linked = {str(f.relative_to(self.root)): p['name'] for f, p in linked.items()}
        self.unlinked = {str(f.relative_to(self.root)): p['name'] for f, p in packages.items()
                         if p['id'] in members and f not in linked and f.is_relative_to(self.root)}

    def includes(self, path):
        if any(inside(path, folder) for folder in self.linked):
            return True
        # Every member manifest is needed to load Cargo's workspace.
        if any(path == folder + '/Cargo.toml' for folder in self.unlinked):
            return True
        listed = [p.rstrip('/') for p in self.listed if inside(path, p.rstrip('/'))]
        if not listed:
            return False
        for folder in self.unlinked:
            if inside(path, folder):
                # Explicit registry entries inside a withheld package restore inputs.
                return any(inside(p, folder) for p in listed)
        return True

    def objects(self):
        entries = subprocess.check_output(['git', 'ls-tree', '-r', '-z', 'HEAD'], cwd=self.root)
        return [entry for entry in entries.split(b'\0') if entry and
                self.includes(entry.split(b'\t', 1)[1].decode())]

    def key(self):
        return hashlib.sha256(json.dumps(self.listed).encode() + b'\0' +
                              b'\0'.join(self.objects()) + b'\0').hexdigest()

    def summary(self):
        return {'linked_packages': dict(sorted(self.linked.items())),
                'unlinked_packages': dict(sorted(self.unlinked.items())),
                'covered_paths': len(self.objects())}


def check_build_reads(root, coverage, records, build_folder=None):
    """Read Cargo's aggregate Make depfiles, normalising includes with ../ segments."""
    root = root.resolve()
    build_folder = (build_folder or root / 'target').resolve()
    covered = {entry.split(b'\t', 1)[1].decode() for entry in coverage.objects()}
    checked = []
    for record in records:
        record = root / record
        try:
            text = record.read_text()
        except OSError as error:
            raise ValueError(f'missing/unreadable build dependency record {record}: {error}') from error
        # Cargo emits one aggregate target rule followed by empty phony rules.
        rules = text.replace('\\\n', '').splitlines()
        inputs = []
        for rule in rules:
            if ': ' not in rule:
                continue
            inputs.extend(shlex.split(rule.split(': ', 1)[1]))
        if not inputs:
            raise ValueError(f'unreadable build dependency record (no inputs): {record}')
        repository_inputs = []
        for name in inputs:
            path = (root / name).resolve()
            if path.is_relative_to(root) and not path.is_relative_to(build_folder):
                relative = str(path.relative_to(root))
                if relative not in covered:
                    raise ValueError(f'build read uncovered file {relative}; record {record}; list it in the registry')
                repository_inputs.append(relative)
        checked.append({'record': str(record.relative_to(root)),
                        'repository_inputs': sorted(set(repository_inputs))})
    return checked
