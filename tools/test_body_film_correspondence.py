import unittest
from body_film_correspondence import overlap

class QuadChartOverlapTests(unittest.TestCase):
    def test_diagonal_change_preserves_coverage(self):
        donors=[[(0.,0.),(1.,0.),(1.,1.)],[(0.,0.),(1.,1.),(0.,1.)]]
        recipients=[[(0.,0.),(1.,0.),(0.,1.)],[(1.,0.),(1.,1.),(0.,1.)]]
        for donor in donors:
            areas=[overlap(donor,r) for r in recipients]
            self.assertEqual(areas,[.25,.25])
            self.assertEqual(sum(areas),.5)
    def test_refinement_and_boundary_contact(self):
        triangle=[(0.,0.),(1.,0.),(0.,1.)]
        small=[(0.,0.),(.5,0.),(0.,.5)]
        self.assertEqual(overlap(triangle,small),.125)
        self.assertEqual(overlap(list(reversed(triangle)),list(reversed(small))),.125)
        self.assertEqual(overlap(triangle,[(1.,0.),(2.,0.),(1.,1.)]),0.)
        self.assertEqual(overlap(triangle,[(2.,2.),(3.,2.),(2.,3.)]),0.)
if __name__=='__main__':unittest.main()
