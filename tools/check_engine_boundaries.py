#!/usr/bin/env python3
"""Check production dependency boundaries, including optional/target dependencies.

Development dependencies are intentionally excluded: integration tests/examples
may compose the editor and engine. This verifies package layering, not runtime
resource ownership or system access enforcement.
"""
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = {
    'voxy_assets': {'voxy_editor', 'voxy_render', 'voxy_scene', 'voxy_runtime'},
    'voxy_scene': {'voxy_editor', 'voxy_render', 'voxy_runtime'},
    'voxy_render': {'voxy_editor', 'voxy_scene', 'voxy_runtime'},
    'voxy_runtime': {'voxy_editor', 'voxy_render'},
}


def dependency_specs(manifest, workspace_dependencies=None):
    workspace_dependencies = workspace_dependencies or {}
    tables = [manifest]
    tables.extend(manifest.get('target', {}).values())
    for table in tables:
        for section in ('dependencies', 'build-dependencies'):
            for name, spec in table.get(section, {}).items():
                if isinstance(spec, dict) and spec.get('workspace'):
                    if name not in workspace_dependencies:
                        raise ValueError(f'missing inherited workspace dependency: {name}')
                    spec = workspace_dependencies[name]
                yield name, spec


def dependencies(manifest, workspace_dependencies=None):
    for name, spec in dependency_specs(manifest, workspace_dependencies):
        yield spec.get('package', name) if isinstance(spec, dict) else name


def load_graph(root):
    root = root.resolve()
    workspace = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']
    inherited = workspace.get('dependencies', {})
    pending = []
    excluded = {path.resolve() for pattern in workspace.get('exclude', [])
                for path in root.glob(pattern)}
    for pattern in workspace['members']:
        members = sorted(root.glob(pattern))
        if not members:
            raise ValueError(f'workspace member pattern has no matches: {pattern}')
        pending.extend(path / 'Cargo.toml' for path in members if path.resolve() not in excluded)
    graph, manifests, seen = {}, {}, set()
    while pending:
        path = pending.pop().resolve()
        if path in seen:
            continue
        seen.add(path)
        data = tomllib.loads(path.read_text())
        local_root, local_inherited = root, inherited
        for parent in path.parents:
            candidate = parent / 'Cargo.toml'
            if candidate.exists():
                owner = tomllib.loads(candidate.read_text())
                if 'workspace' in owner:
                    local_root = parent
                    local_inherited = owner['workspace'].get('dependencies', {})
                    break
        package = data['package']['name']
        if package in manifests and manifests[package] != path:
            raise ValueError(f'ambiguous local package {package}: {manifests[package]} and {path}')
        manifests[package] = path
        graph[package] = set(dependencies(data, local_inherited))
        for _, spec in dependency_specs(data, local_inherited):
            if isinstance(spec, dict) and 'path' in spec:
                # Workspace-inherited paths are relative to the workspace root.
                origin = local_root if any(spec is value for value in local_inherited.values()) else path.parent
                pending.append(origin / spec['path'] / 'Cargo.toml')
    return graph


def violations(graph, boundaries):
    errors = []
    for owner, forbidden in boundaries.items():
        if owner not in graph:
            errors.append(f'missing required package: {owner}')
            continue
        pending = [(owner, [owner])]
        seen = {owner}
        while pending:
            current, path = pending.pop()
            for dependency in sorted(graph.get(current, ())):
                chain = path + [dependency]
                if dependency in forbidden:
                    errors.append(' -> '.join(chain))
                if dependency in graph and dependency not in seen:
                    seen.add(dependency)
                    pending.append((dependency, chain))
    return errors


def main():
    graph = load_graph(ROOT)
    errors = violations(graph, FORBIDDEN)
    if errors:
        print('Engine boundary violations:\n' + '\n'.join(errors), file=sys.stderr)
        return 1
    print(f'Engine boundaries passed: {len(FORBIDDEN)} owners, {len(graph)} packages; production transitive dependencies checked.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
