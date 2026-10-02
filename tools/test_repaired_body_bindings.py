import unittest
import numpy as np
from rebind_repaired_body import closest,rebind

class BindingProjectionTests(unittest.TestCase):
    def test_added_vertex_projection_preserves_source_bindings_and_shell(self):
        reference=[(0.,0.,0.),(1.,0.,0.),(0.,1.,0.)]
        bindings=[{'vertex':i,'position':list(p),'triangle':0,'weights':[float(i==j) for j in range(3)]} for i,p in enumerate(reference)]
        data={'positions':[list(p) for p in reference],'triangles':[[0,1,2]],'bindings':bindings,'render_vertices':3,'max_binding_distance_m':0.}
        candidate=reference+[(.25,.25,.1)]
        with self.assertRaises(ValueError):rebind(reference,candidate,data)
        output,report=rebind(reference,candidate,data,allow_added=True,added_fades={3:.3})
        self.assertEqual(output['bindings'][:3],bindings)
        self.assertEqual(data['bindings'],bindings)
        self.assertEqual(len(data['bindings']),3)
        self.assertEqual(data['render_vertices'],3)
        self.assertEqual(output['render_vertices'],len(candidate))
        self.assertEqual(output['render_vertices'],len(output['bindings']))
        self.assertEqual(output['positions'],data['positions'])
        self.assertEqual(output['triangles'],data['triangles'])
        self.assertEqual(report['added_vertices'],1)
        self.assertEqual(report['preserved_vertices'],3)
        self.assertAlmostEqual(output['max_binding_distance_m'],.1)
        self.assertEqual(output['bindings'][3]['fade'],.3)
        with self.assertRaises(ValueError):rebind(reference,candidate,data,allow_added=True,added_fades={3:float('nan')})
        np.testing.assert_allclose(output['bindings'][3]['weights'],[.5,.25,.25],atol=1e-12)

    def test_interior_projection(self):
        tri=np.array([[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]]])
        face,weights,distance=closest(tri,np.array([.2,.3,.4]))
        self.assertEqual(face,0)
        np.testing.assert_allclose(weights,[.5,.2,.3],atol=1e-12)
        self.assertAlmostEqual(distance,.4)

    def test_edge_and_vertex_projection(self):
        tri=np.array([[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]]])
        _,weights,distance=closest(tri,np.array([.5,-.2,0.]))
        np.testing.assert_allclose(weights,[.5,.5,0.],atol=1e-12)
        self.assertAlmostEqual(distance,.2)
        _,weights,_=closest(tri,np.array([2.,0.,0.]))
        np.testing.assert_allclose(weights,[0.,1.,0.],atol=1e-12)

    def test_nearest_face_is_selected(self):
        tri=np.array([[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]],[[0.,0.,2.],[1.,0.,2.],[0.,1.,2.]]])
        face,_,distance=closest(tri,np.array([.2,.2,1.8]))
        self.assertEqual(face,1)
        self.assertAlmostEqual(distance,.2)
