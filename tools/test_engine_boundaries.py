"""Regression checks for manifest shapes that can conceal layer violations."""
import unittest
import tempfile
from pathlib import Path

from check_engine_boundaries import dependencies, violations, load_graph


class EngineBoundariesTest(unittest.TestCase):
    def test_nested_member_and_nonmember_path_bridge_are_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = {
                'Cargo.toml': '[workspace]\nmembers = ["engine/*"]\n[workspace.dependencies]\nbridge = { path = "support/bridge" }\n',
                'engine/scene/Cargo.toml': '[package]\nname = "voxy_scene"\n[dependencies]\nbridge = { workspace = true }\n',
                'engine/editor/Cargo.toml': '[package]\nname = "voxy_editor"\n',
                'support/bridge/Cargo.toml': '[package]\nname = "bridge"\n[target."cfg(unix)".build-dependencies]\nui = { package = "voxy_editor", path = "../../engine/editor" }\n',
            }
            for relative, content in files.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
            self.assertEqual(violations(load_graph(root), {'voxy_scene': {'voxy_editor'}}),
                             ['voxy_scene -> bridge -> voxy_editor'])

    def test_missing_member_is_an_error(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'Cargo.toml').write_text('[workspace]\nmembers = ["missing/*"]\n')
            with self.assertRaisesRegex(ValueError, 'no matches'):
                load_graph(root)

    def test_inherited_alias_in_optional_target_build_dependency(self):
        manifest = {'target': {'cfg(unix)': {'build-dependencies': {
            'backend': {'workspace': True, 'optional': True},
        }}}}
        inherited = {'backend': {'package': 'voxy_editor', 'path': 'crates/voxy_editor'}}
        graph = {'voxy_scene': set(dependencies(manifest, inherited)), 'voxy_editor': set()}
        self.assertEqual(violations(graph, {'voxy_scene': {'voxy_editor'}}),
                         ['voxy_scene -> voxy_editor'])

    def test_inherited_transitive_violation(self):
        graph = {
            'voxy_assets': set(dependencies({'dependencies': {
                'bridge': {'workspace': True}}}, {'bridge': {'package': 'adapter'}})),
            'adapter': {'voxy_scene'},
            'voxy_scene': set(),
        }
        self.assertEqual(violations(graph, {'voxy_assets': {'voxy_scene'}}),
                         ['voxy_assets -> adapter -> voxy_scene'])

    def test_missing_inheritance_fails_closed(self):
        with self.assertRaisesRegex(ValueError, 'missing inherited workspace dependency: backend'):
            list(dependencies({'dependencies': {'backend': {'workspace': True}}}))

    def test_development_dependency_remains_excluded(self):
        self.assertEqual(list(dependencies({'dev-dependencies': {
            'backend': {'workspace': True}}})), [])

    def test_direct_alias_and_external_string_dependency(self):
        self.assertEqual(set(dependencies({'dependencies': {
            'alias': {'package': 'voxy_render'}, 'serde': '1'}})),
                         {'voxy_render', 'serde'})


if __name__ == '__main__':
    unittest.main()
