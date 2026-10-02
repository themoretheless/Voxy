import unittest
from classify_body_intersections import classify
from prepare_body_geometry import intersection_segment_length

class ClassificationTests(unittest.TestCase):
 def test_intersection_segment_length_is_symmetric_and_scales(self):
  a=[(0,0,0),(1,0,0),(0,1,0)]
  b=[(.2,.2,-1),(.2,.2,1),(.8,.2,0)]
  for scale in (1e-6,1e-3,1,1e3):
   transform=lambda t:[tuple((x+offset)*scale for x,offset in zip(p,(4,3,2))) for p in t]
   aa,bb=transform(a),transform(b)
   self.assertAlmostEqual(intersection_segment_length(aa,bb)/scale,.6,places=10)
   self.assertAlmostEqual(intersection_segment_length(bb,aa)/scale,.6,places=10)
  self.assertIsNone(intersection_segment_length(a,a))

 def test_known_contact_and_crossing_types_across_units(self):
  a=[(0,0,0),(1,0,0),(0,1,0)]
  fixtures=[
   ([(.2,.2,-1),(.2,.2,1),(.8,.2,0)],'transverse_segment_crossing'),
   ([(.1,.1,0),(.8,.1,0),(.1,.8,0)],'coplanar_area_overlap'),
   ([(.5,0,0),(1.5,0,0),(1,-1,0)],'coplanar_boundary_contact'),
   ([(0,0,0),(0,0,1),(-1,-1,0)],'point_contact'),
   ([(0,0,0),(1,0,0),(.5,0,1)],'tangential_segment_contact'),
   ([(0,0,.01),(1,0,.01),(0,1,.01)],'parallel_separated')]
  for scale in (1e-6,1e-3,1,1e3):
   transform=lambda t:[tuple(x*scale for x in p) for p in t]
   for b,expected in fixtures:
    with self.subTest(scale=scale,expected=expected):
     self.assertEqual(classify(transform(a),transform(b)),expected)
     self.assertEqual(classify(transform(b),transform(a)),expected)

if __name__=='__main__':unittest.main()
