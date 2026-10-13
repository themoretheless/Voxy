"""Regression coverage for mixed source provenance and tampering rejection."""
import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('engine_audit', pathlib.Path(__file__).with_name('audit.py'))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class SourceAudit(unittest.TestCase):
    def test_mixed_manifests_and_tampering(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            mechanism = root / 'mechanisms' / 'rig'
            mechanism.mkdir(parents=True)
            commit = 'a' * 40
            payload = b'original pinned source\n'
            digest = hashlib.sha256(payload).hexdigest()
            local = mechanism / 'sources' / 'owner__repo' / 'shader.wgsl'
            local.parent.mkdir(parents=True)
            local.write_bytes(payload)
            source = dict(repo='owner/repo', sha=commit, path='shader.wgsl',
                          local='sources/owner__repo/shader.wgsl', sha256=digest,
                          url=f'https://github.com/owner/repo/blob/{commit}/shader.wgsl')
            manifest = mechanism / 'sources.json'
            manifest.write_text(json.dumps([source]))
            nested = local.parent / 'source-manifest.json'
            nested.write_text(json.dumps(dict(repository='https://github.com/owner/repo', commit=commit,
                files=[dict(path='shader.wgsl', sha256=digest,
                            url=f'https://raw.githubusercontent.com/owner/repo/{commit}/shader.wgsl')])))
            self.assertEqual(audit.audit_mechanisms(root), dict(manifests=2, verified_sources=2, unique_pinned_sources=1, duplicate_source_records=1, repositories_with_saved_sources=1, repository_source_counts={"owner/repo":1}))
            # Individually valid digests cannot justify contradictory copies of
            # the same repository/revision/path in separate manifests.
            conflicting = local.parent / 'copy.wgsl'
            conflicting.write_bytes(b'different pinned source')
            nested.write_text(json.dumps(dict(repository='https://github.com/owner/repo', commit=commit,
                files=[dict(file='copy.wgsl', path='shader.wgsl', sha256=hashlib.sha256(conflicting.read_bytes()).hexdigest(),
                            url=f'https://raw.githubusercontent.com/owner/repo/{commit}/shader.wgsl')])))
            with self.assertRaisesRegex(ValueError, 'conflicting copies'):
                audit.audit_mechanisms(root)
            nested.unlink()
            for field, value in [('url', 'https://github.com/owner/repo/blob/main/shader.wgsl'),
                                 ('local', '../outside'), ('sha256', '0' * 64), ('sha', 'main')]:
                with self.subTest(field=field):
                    manifest.write_text(json.dumps([{**source, field: value}]))
                    with self.assertRaises(ValueError):
                        audit.audit_mechanisms(root)
            manifest.write_text(json.dumps([source]))
            local.write_bytes(b'changed source')
            with self.assertRaises(ValueError):
                audit.audit_mechanisms(root)
            # The older single-repository/flat-file format remains checked.
            flat = mechanism / 'source.wgsl'
            flat.write_bytes(payload)
            manifest.write_text(json.dumps(dict(repository='owner/repo', commit=commit,
                sources=[dict(file='source.wgsl', path='shader.wgsl', sha256=digest,
                              url=f'https://raw.githubusercontent.com/owner/repo/{commit}/shader.wgsl')])))
            self.assertEqual(audit.audit_mechanisms(root), dict(manifests=1, verified_sources=1, unique_pinned_sources=1, duplicate_source_records=0, repositories_with_saved_sources=1, repository_source_counts={"owner/repo":1}))


if __name__ == '__main__':
    unittest.main()
