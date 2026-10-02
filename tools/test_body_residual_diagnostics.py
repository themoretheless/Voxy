import unittest
import hashlib
import tempfile
from pathlib import Path
from diagnose_body_residuals import components, diagnose


class ResidualDiagnosticsTests(unittest.TestCase):
    def test_audit_provenance_completeness_and_pair_geometry_are_checked(self):
        with tempfile.TemporaryDirectory() as folder:
            source=Path(folder)/'mesh.obj'
            source.write_text('v -1 -1 0\nv 1 -1 0\nv 0 1 0\nv 0 0 -1\nv 0 0 1\nv 0 0.5 0\nv 4 4 1\nv 5 4 1\nv 4 5 1\nf 1 2 3\nf 4 5 6\nf 7 8 9\n')
            audit={'candidate_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
                   'after':1,'remaining':{'intersection_limit_reached':False,'intersection_pairs':[[0,1]]}}
            self.assertTrue(diagnose(source,audit)['reported_pairs_rechecked'])
            audit['remaining']['intersection_pairs']=[[0,2]]
            with self.assertRaisesRegex(ValueError,'geometry recheck'):diagnose(source,audit)
            audit['remaining']['intersection_limit_reached']=True
            with self.assertRaisesRegex(ValueError,'capped'):diagnose(source,audit)
            audit['candidate_sha256']='wrong'
            with self.assertRaisesRegex(ValueError,'hash'):diagnose(source,audit)
    def test_pair_links_and_shared_vertices_form_components_with_support(self):
        points = [(float(i), 0., 0.) for i in range(15)]
        faces = [(0, 1, 2), (3, 4, 5), (5, 6, 7), (8, 9, 10), (11, 12, 13), (0, 2, 14)]
        patches = components(points, faces, [(0, 1), (1, 2), (3, 4)])
        self.assertEqual([patch['faces'] for patch in patches], [[0, 1, 2], [3, 4]])
        self.assertEqual(patches[0]['one_ring_faces'], [0, 1, 2, 5])
        self.assertEqual(patches[0]['bounds_m'], {'minimum': [0., 0., 0.], 'maximum': [7., 0., 0.]})
        self.assertEqual(sum(len(p['forbidden_pairs']) for p in patches), 3)

    def test_shared_topology_merges_otherwise_disjoint_pair_groups(self):
        points = [(float(i), 0., 0.) for i in range(11)]
        faces = [(0, 1, 2), (3, 4, 5), (5, 6, 7), (8, 9, 10)]
        patches = components(points, faces, [(0, 1), (2, 3), (1, 0)])
        self.assertEqual(len(patches), 1)
        self.assertEqual(len(patches[0]['forbidden_pairs']), 2)

    def test_empty_and_invalid_pairs(self):
        self.assertEqual(components([], [], []), [])
        for pairs in [[(0, 0)], [(-1, 0)], [(0, 2)]]:
            with self.assertRaises(ValueError):
                components([(0., 0., 0.)]*3, [(0, 1, 2)], pairs)
