import unittest
import numpy as np
from restore_repaired_body_normals import transport,refined_authored,vertex_transport,smooth_normals,rotate_normals

class TransportTests(unittest.TestCase):
    def test_frame_rotation_preserves_source_normal_offset(self):
        source=np.array([[0.,0.,1.],[0.,1.,0.]])
        target=np.array([[0.,1.,0.],[1.,0.,0.]])
        authored=np.array([[.1,.2,1.],[.3,1.,.1]])
        authored/=np.linalg.norm(authored,axis=1)[:,None]
        output=rotate_normals(source,target,authored)
        np.testing.assert_allclose(np.sum(output*target,axis=1),np.sum(authored*source,axis=1),atol=1e-12)
        np.testing.assert_allclose(rotate_normals(source,source,authored),authored,atol=1e-12)
        with self.assertRaises(ValueError):rotate_normals(source*2,target,authored)

    def test_vertex_frames_allow_different_triangulation(self):
        rest=np.array([[0.,0.,0.],[1.,0.,0.],[1.,1.,0.],[0.,1.,0.]])
        old=np.array([[0,1,2],[0,2,3]]);new=np.array([[0,1,3],[1,2,3]])
        authored=np.tile([.1,.2,1.],(4,1));authored/=np.linalg.norm(authored,axis=1)[:,None]
        np.testing.assert_allclose(vertex_transport(rest,rest,old,new,authored),authored,atol=1e-12)
        rotated=rest.copy();rotated[:,2]=rest[:,1];rotated[:,1]=0.
        result=vertex_transport(rest,rotated,old,new,authored)
        expected=authored[:,[0,2,1]].copy();expected[:,1]*=-1
        np.testing.assert_allclose(result,expected,atol=1e-12)
        np.testing.assert_allclose(np.sum(result*smooth_normals(rotated,new),axis=1),authored[:,2],atol=1e-12)

    def test_vertex_frames_handle_opposite_normals(self):
        rest=np.array([[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]])
        faces=np.array([[0,1,2]]);authored=np.tile([0.,0.,1.],(3,1))
        np.testing.assert_allclose(vertex_transport(rest,rest,faces,faces[:,::-1],authored),-authored,atol=1e-12)

    def test_refinement_interpolates_normals_and_checks_identity(self):
        original=np.array([[0.,0.,0.],[1.,0.,0.]])
        authored=np.array([[0.,0.,1.],[0.,1.,0.]])
        reference=np.array([[0.,0.,0.],[1.,0.,0.],[.5,0.,0.]])
        sources=[[(0,1.)],[(1,1.)],[(0,.5),(1,.5)]]
        normals=refined_authored(reference,original,authored,sources)
        np.testing.assert_allclose(normals[2],[0.,2**-.5,2**-.5])
        np.testing.assert_allclose(normals[:2],authored)
        damaged=reference.copy();damaged[2,0]+=.001
        with self.assertRaises(ValueError):refined_authored(damaged,original,authored,sources)
        for row in [[(0,.4),(1,.4)],[(2,1.)],[(0,float('nan'))],[(0,-.5),(1,1.5)]]:
            with self.assertRaises(ValueError):refined_authored(reference,original,authored,sources[:2]+[row])

    def test_shear_transports_plane_normal(self):
        rest=np.array([[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]])
        current=rest.copy();current[:,2]=current[:,0]*.5
        authored=np.tile([0.,0.,1.],(3,1));faces=np.array([[0,1,2]])
        expected=np.array([-.5,0.,1.]);expected/=np.linalg.norm(expected)
        np.testing.assert_allclose(transport(rest,current,faces,authored),np.tile(expected,(3,1)),atol=1e-12)

    def test_rest_preserves_authored_smoothing(self):
        rest=np.array([[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]])
        authored=np.tile([.1,.2,1.],(3,1));authored/=np.linalg.norm(authored,axis=1)[:,None]
        np.testing.assert_allclose(transport(rest,rest,np.array([[0,1,2]]),authored),authored,atol=1e-12)

    def test_degenerate_frame_rejected(self):
        rest=np.array([[0.,0.,0.],[1.,0.,0.],[2.,0.,0.]])
        with self.assertRaises(ValueError):transport(rest,rest,np.array([[0,1,2]]),np.tile([0.,0.,1.],(3,1)))
