import math,unittest
import numpy as np
from audit_body_surface_quality import quality

class SurfaceQualityTests(unittest.TestCase):
    def test_equilateral_quality_is_one_across_scales(self):
        points=np.array([[0.,0.,0.],[1.,0.,0.],[.5,math.sqrt(3)/2,0.]])
        for scale in (.001,1.,1000.):
            report=quality(points*scale,[(0,1,2)])
            self.assertAlmostEqual(report['minimum_triangle_quality'],1.)
            self.assertEqual(report['zero_area_triangles'],0)

    def test_sliver_and_collapsed_triangles_are_reported(self):
        for height,zeros in [(1e-5,0),(0.,1)]:
            report=quality([(0.,0.,0.),(1.,0.,0.),(.5,height,0.)],[(0,1,2)])
            self.assertLess(report['minimum_triangle_quality'],.05)
            self.assertEqual(report['triangles_below_quality_005'],1)
            self.assertEqual(report['zero_area_triangles'],zeros)

    def test_known_right_angle_and_region_selection(self):
        points=[(0.,0.,0.),(1.,0.,0.),(0.,1.,0.),(0.,0.,1.)]
        report=quality(points,[(0,1,2),(1,0,3)])
        self.assertAlmostEqual(report['maximum_adjacent_face_angle_degrees'],90.)
        report=quality(points,[(0,1,2),(1,0,3)],[-1,2,.1,2,-.1,.1])
        self.assertEqual(report['selected_triangles'],1)
        self.assertAlmostEqual(report['maximum_adjacent_face_angle_degrees'],90.)
        with self.assertRaises(ValueError):quality(points,[(0,1,2)],[5,6,5,6,5,6])
