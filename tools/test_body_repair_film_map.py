import unittest
from body_repair_film_map import build,compose,refinement_map

class MaterialChartTests(unittest.TestCase):
    def test_refinement_distributes_volume_and_rejects_overlaps_or_gaps(self):
        p=[(0.,0.,0.),(1.,0.,0.),(0.,1.,0.)]
        q=p+[(.5,0.,0.)]
        forward,reverse,_=refinement_map(p,[(0,1,2)],q,[(0,3,2),(3,1,2)],[0,0])
        self.assertEqual(forward,[[(0,.5),(1,.5)]])
        self.assertEqual(reverse,[[(0,1.)],[(0,1.)]])
        self.assertEqual(compose(forward,reverse),[[(0,1.)]])
        with self.assertRaises(ValueError):refinement_map(p,[(0,1,2)],q,[(0,3,2),(0,3,2)],[0,0])
        with self.assertRaises(ValueError):refinement_map(p,[(0,1,2)],q,[(0,3,2)],[0])
        bad=q.copy();bad[3]=(.5,0.,.001)
        with self.assertRaises(ValueError):refinement_map(p,[(0,1,2)],bad,[(0,3,2),(3,1,2)],[0,0])
    def test_diagonal_flip_maps_half_to_each_cell_in_both_directions(self):
        p=[(0.,0.,0.),(1.,0.,0.),(1.,1.,0.),(0.,1.,0.)]
        a,b,r=build(p,[(0,1,2),(0,2,3)],[(0,1,3),(1,2,3)])
        self.assertEqual(r['patches'],1)
        for rows in (a,b):
            for row in rows:
                self.assertEqual([i for i,_ in row],[0,1])
                for _,w in row:self.assertAlmostEqual(w,.5)

    def test_composition_preserves_mass_and_unchanged_cell(self):
        rows=compose([[(0,.25),(1,.75)],[(2,1.)]],
                     [[(0,.4),(1,.6)],[(1,1.)],[(2,1.)]])
        self.assertAlmostEqual(rows[0][0][1],.1)
        self.assertAlmostEqual(rows[0][1][1],.9)
        self.assertEqual(rows[1],[(2,1.)])
        self.assertAlmostEqual(sum(w for _,w in rows[0]),1.)

    def test_boundary_change_is_rejected(self):
        p=[(0.,0.,0.),(1.,0.,0.),(1.,1.,0.),(0.,1.,0.),(2.,1.,0.)]
        with self.assertRaises(ValueError):build(p,[(0,1,2),(0,2,3)],[(0,1,4),(0,4,3)])
