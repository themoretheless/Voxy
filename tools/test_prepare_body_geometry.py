"""Known geometric fixtures across units; not an anatomical validity proof."""
import unittest
from prepare_body_geometry import crossing, segment_triangle, repair_intersections, split_edges, audit

class IntersectionScaleTests(unittest.TestCase):
    def test_local_fold_objective_reduces_angle_without_crossing(self):
        from repair_body_vertex_descent import repair
        points=[(0.,0.,0.),(.01,0.,0.),(0.,.01,0.),(0.,0.,.01)];faces=[(0,1,2),(1,0,3)]
        _,report=repair(points,faces,points,passes=2,include_adjacent=True,reduce_measure=True,quality_floor=.05,fold_threshold_degrees=60,fairing=True)
        self.assertEqual((report['before'],report['after']),(0,0))
        self.assertLess(report['fold_audit']['final_deficit'],report['fold_audit']['initial_deficit'])
        self.assertEqual(report['fold_audit']['selected_edges'],1)
        self.assertLessEqual(report['fold_audit']['final_maximum_angle_degrees'],report['fold_audit']['initial_maximum_angle_degrees']+1e-8)
        for step in report['steps']:self.assertLessEqual(step['local_fold_deficit_after'],step['local_fold_deficit_before']+1e-12)
        _,outside=repair(points,faces,points,include_adjacent=True,reduce_measure=True,quality_floor=.05,fold_threshold_degrees=60,fold_bounds=[1,2,1,2,1,2])
        self.assertFalse(outside['steps'])
        self.assertEqual(outside['fold_audit']['selected_edges'],0)
        self.assertEqual(outside['fold_audit']['initial_maximum_angle_degrees'],0.)
        self.assertEqual(outside['fold_audit']['final_maximum_angle_degrees'],0.)
        with self.assertRaises(ValueError):repair(points,faces,points,fold_threshold_degrees=180)
        with self.assertRaises(ValueError):repair(points,faces,points,fold_threshold_degrees=60)

    def test_diagonal_quality_gate_preserves_existing_sliver_rank(self):
        from repair_body_diagonals import quality_pair_allowed,repair
        self.assertFalse(quality_pair_allowed([.02,.8],[.021,.04],.05))
        self.assertFalse(quality_pair_allowed([.02,.8],[.019,.9],.05))
        self.assertTrue(quality_pair_allowed([.02,.8],[.03,.1],.05))
        points=[(0.,0.,0.),(.001,0.,0.),(0.,.001,0.),(0.,0.,.001)]
        faces=[(0,2,1),(0,1,3),(1,2,3),(2,0,3)]
        candidate,report=repair(points,faces,include_adjacent=True,quality_floor=.05)
        self.assertEqual(candidate,faces)
        self.assertTrue(report['qualityAudit']['verified'])
        self.assertEqual(report['qualityAudit']['finalBelowFloor'],0)
        with self.assertRaises(ValueError):repair(points,faces,quality_floor=float('nan'))

    def test_sliver_repair_works_without_existing_intersections(self):
        from repair_body_vertex_descent import repair
        points=[(0.,0.,0.),(.01,0.,0.),(.005,.00001,0.)];faces=[(0,1,2)]
        _,ordinary=repair(points,faces,points,passes=2,quality_floor=.05,reduce_measure=True,include_adjacent=True)
        self.assertFalse(ordinary['steps'])
        _,repaired=repair(points,faces,points,passes=2,quality_floor=.05,reduce_measure=True,include_adjacent=True,repair_slivers=True,fairing=True)
        self.assertEqual((repaired['before'],repaired['after']),(0,0))
        self.assertGreater(repaired['quality_audit']['final_minimum'],repaired['quality_audit']['initial_minimum'])
        self.assertTrue(repaired['steps'])
        for step in repaired['steps']:
            self.assertLess(step['local_quality_deficit_after'],step['local_quality_deficit_before'])
        with self.assertRaises(ValueError):repair(points,faces,points,repair_slivers=True,quality_floor=.05)

    def test_quality_gate_rejects_crossing_removal_that_creates_sliver(self):
        from repair_body_vertex_descent import repair,triangle_quality
        points=[(0.,0.,0.),(.01,0.,0.),(-.001541810074139724,.004740480701813368,0.),(.002,.002,-.0001),(.002,.002,.0002),(.008,.002,.0002)]
        faces=[(0,1,2),(3,4,5)]
        ordinary,plain=repair(points,faces,points,passes=1,distances=(.0002,-.0002),reduce_measure=True)
        protected,report=repair(points,faces,points,passes=1,distances=(.0002,-.0002),reduce_measure=True,quality_floor=.05)
        self.assertEqual(plain['after'],0)
        self.assertLess(min(triangle_quality([ordinary[v] for v in t]) for t in faces),.05)
        self.assertGreaterEqual(min(triangle_quality([protected[v] for v in t]) for t in faces),.05)
        self.assertTrue(report['quality_audit']['verified'])
        for floor in (-.1,1.1,float('nan')):
            with self.assertRaises(ValueError):repair(points,faces,points,quality_floor=floor)

    def test_fairing_direction_is_translation_invariant(self):
        from repair_body_vertex_descent import fairing_direction
        points=[(0.,0.,.001),(-.01,0.,0.),(.01,0.,0.),(0.,-.01,0.),(0.,.01,0.)]
        neighbours={0:{1,2,3,4}}
        self.assertEqual(fairing_direction(points,neighbours,(0,)),(0.,0.,-1.))
        shifted=[tuple(p[k]+(.3,-.4,.2)[k] for k in range(3)) for p in points]
        for a,b in zip(fairing_direction(shifted,neighbours,(0,)),(0.,0.,-1.)):self.assertAlmostEqual(a,b)
        points[0]=(0.,0.,0.)
        self.assertIsNone(fairing_direction(points,neighbours,(0,)))

    def test_search_rejects_invalid_coordinates_before_bound_checks(self):
        from repair_body_vertex_descent import repair
        points=[(0.,0.,0.),(.01,0.,0.),(0.,.01,0.)]
        for invalid in [float('nan'),float('inf'),float('-inf')]:
            damaged=list(points);damaged[0]=(invalid,0.,0.)
            with self.assertRaises(ValueError):repair(damaged,[(0,1,2)],points)
            with self.assertRaises(ValueError):repair(points,[(0,1,2)],damaged)
        for faces in [[],[(0,1,3)],[(0,1,-1)],[(0,1,1)],[(0,1)],[(0,1,2.)]]:
            with self.assertRaises(ValueError):repair(points,faces,points)
        with self.assertRaises(ValueError):repair([],[],[])

    def test_ring_search_checks_entire_moving_neighbourhood(self):
        from repair_body_vertex_descent import repair
        points=[(0.,0.,0.),(.01,0.,0.),(0.,.01,0.),(.002,.002,-.0001),(.002,.002,.0002),(.008,.002,.0002),(-.01,0.,0.)]
        faces=[(0,1,2),(3,4,5),(0,2,6)]
        candidate,report=repair(points,faces,points,passes=1,include_adjacent=True,ring_groups=True,reduce_measure=True)
        self.assertEqual((report['before'],report['after']),(1,0))
        moved=[i for i,(a,b) in enumerate(zip(points,candidate)) if a!=b]
        self.assertEqual(moved,[0,1,2,6])
        for i in moved:
            for k in range(3):self.assertAlmostEqual(candidate[i][k]-points[i][k],candidate[0][k]-points[0][k])
        self.assertLessEqual(report['maximum_cumulative_displacement_m'],.003)
        self.assertFalse(report['audit']['degenerate_triangles'])
        for mode in ('edge_groups','face_groups'):
            with self.assertRaises(ValueError):repair(points,faces,points,ring_groups=True,**{mode:True})

    def test_diagonal_proposals_keep_bound_and_remove_rotated_crossing(self):
        from repair_body_vertex_descent import repair,lattice_directions
        import math
        directions=lattice_directions()
        self.assertEqual(len(directions),13)
        self.assertEqual(len(set(directions)),13)
        for n in directions:
            self.assertAlmostEqual(sum(x*x for x in n),1.)
            self.assertNotIn(tuple(-x for x in n),directions)
        original=[(0.,0.,0.),(.01,0.,0.),(0.,.01,0.),(.002,.002,-.0001),(.002,.002,.0002),(.008,.002,.0002)]
        # Rotate the crossing out of the coordinate planes.
        points=[((x-y)/math.sqrt(2),(x+y-z)/math.sqrt(3),(x+y+2*z)/math.sqrt(6)) for x,y,z in original]
        _,report=repair(points,[(0,1,2),(3,4,5)],points,passes=1,bound=.00015,distances=(.0002,-.0002),project_bound=True,reduce_measure=True,diagonal_directions=True)
        self.assertEqual(report['after'],0)
        self.assertLessEqual(report['maximum_cumulative_displacement_m'],.00015)
        self.assertFalse(report['audit']['degenerate_triangles'])

    def test_measure_descent_reduces_extent_without_hiding_pair(self):
        from repair_body_vertex_descent import repair,intersection_measure
        points=[(0.,0.,0.),(.01,0.,0.),(0.,.01,0.),(.002,.002,-.0001),(.002,.002,.0002),(.008,.002,.0002)]
        faces=[(0,1,2),(3,4,5)]
        _,ordinary=repair(points,faces,points,passes=1,distances=(.000025,-.000025))
        _,measured=repair(points,faces,points,passes=1,distances=(.000025,-.000025),reduce_measure=True)
        self.assertFalse(ordinary['steps'])
        self.assertEqual((measured['before'],measured['after']),(1,1))
        self.assertLess(measured['final_measure_m'],measured['initial_measure_m'])
        self.assertTrue(all(s['local_measure_after_m']<s['local_measure_before_m'] for s in measured['steps']))
        a=[(0.,0.,0.),(1.,0.,0.),(0.,1.,0.)]
        self.assertAlmostEqual(intersection_measure(a,a),(.5)**.5)

    def test_projected_search_repairs_without_relaxing_displacement_bound(self):
        from repair_body_vertex_descent import repair
        points=[(0.,0.,0.),(.01,0.,0.),(0.,.01,0.),(.002,.002,-.0001),(.002,.002,.0002),(.008,.002,.0002)]
        faces=[(0,1,2),(3,4,5)]
        _,rejected=repair(points,faces,points,passes=1,bound=.00015,distances=(.0002,-.0002))
        _,projected=repair(points,faces,points,passes=1,bound=.00015,distances=(.0002,-.0002),project_bound=True)
        self.assertEqual(rejected['after'],1)
        self.assertEqual(projected['after'],0)
        self.assertLessEqual(projected['maximum_cumulative_displacement_m'],.00015)

    def test_edge_group_descent_preserves_common_translation(self):
        from repair_body_vertex_descent import repair
        points=[(0.,0.,0.),(.01,0.,0.),(0.,.01,0.),(.002,.002,-.0001),(.002,.002,.0002),(.008,.002,.0002)]
        candidate,report=repair(points,[(0,1,2),(3,4,5)],points,passes=1,include_adjacent=True,edge_groups=True)
        self.assertEqual((report['before'],report['after']),(1,0))
        self.assertEqual(sum(p!=q for p,q in zip(points,candidate)),2)
        for k in range(3):self.assertAlmostEqual(candidate[0][k]-points[0][k],candidate[1][k]-points[1][k])
        with self.assertRaises(ValueError):repair(points,[(0,1,2)],points,face_groups=True,edge_groups=True)

    def test_joint_diagonal_search_preserves_valid_closed_tetrahedron(self):
        from repair_body_diagonals import repair
        points=[(0.,0.,0.),(.001,0.,0.),(0.,.001,0.),(0.,0.,.001)]
        faces=[(0,2,1),(0,1,3),(1,2,3),(2,0,3)]
        candidate,report=repair(points,faces,include_adjacent=True)
        self.assertEqual(candidate,faces)
        self.assertEqual((report['initialCrossings'],report['finalCrossings']),(0,0))
        self.assertFalse(report['audit']['boundary_edges'])
        self.assertFalse(report['remaining']['adjacent']['forbidden_pairs'])
        measured,measured_report=repair(points,faces,include_adjacent=True,reduce_measure=True)
        self.assertEqual(measured,faces)
        self.assertEqual(measured_report['initialMeasureM'],0.)
        self.assertEqual(measured_report['finalMeasureM'],0.)
        self.assertFalse(measured_report['steps'])

    def test_face_group_descent_separates_crossing_with_common_translation(self):
        from repair_body_vertex_descent import repair
        points=[(0.,0.,0.),(.01,0.,0.),(0.,.01,0.),(.002,.002,-.0001),(.002,.002,.0002),(.008,.002,.0002)]
        candidate,report=repair(points,[(0,1,2),(3,4,5)],points,passes=1,include_adjacent=True,face_groups=True)
        self.assertEqual((report['before'],report['after']),(1,0))
        self.assertEqual(sum(p!=q for p,q in zip(points,candidate)),3)
        for k in range(3):
            self.assertAlmostEqual(candidate[0][k]-points[0][k],candidate[1][k]-points[1][k])
            self.assertAlmostEqual(candidate[0][k]-points[0][k],candidate[2][k]-points[2][k])
        self.assertLessEqual(report['maximum_cumulative_displacement_m'],.003)

    def test_vertex_descent_repairs_adjacent_overlap_when_enabled(self):
        from repair_body_vertex_descent import repair
        points=[(0.,0.,0.),(.01,0.,0.),(0.,.01,0.),(.005,.005,0.)]
        faces=[(0,1,2),(1,0,3)]
        _,ordinary=repair(points,faces,points,passes=1)
        self.assertEqual(ordinary['before'],0)
        _,complete=repair(points,faces,points,passes=1,include_adjacent=True)
        self.assertEqual((complete['before'],complete['after']),(1,0))
        self.assertFalse(complete['remaining']['adjacent']['forbidden_pairs'])

    def test_adjacent_audit_ignores_shared_contact_but_detects_overlap(self):
        from audit_body_adjacent_faces import forbidden
        a=[(0,0,0),(2,0,0),(0,2,0)]
        self.assertFalse(forbidden(a,[(2,0,0),(0,0,0),(0,-2,0)],2))
        self.assertTrue(forbidden(a,[(2,0,0),(0,0,0),(1,1,0)],2))
        self.assertTrue(forbidden(a,[(0,0,0),(1,1,-1),(1,1,1)],1))
        self.assertFalse(forbidden(a,[(0,0,0),(-1,-1,-1),(-1,-1,1)],1))
        self.assertFalse(forbidden(a,[(2,0,0),(0,0,0),(1,1,1)],2))

    def test_nearly_parallel_shared_vertex_contact_is_rotation_invariant(self):
        from audit_body_adjacent_faces import forbidden
        a=[(-.08565926,.3334527,.12718192),(-.08530327,.33668995,.12927653),(-.09670098,.33254445,.122700065)]
        b=[(-.08512528,.33830857,.13032383),(-.08494729,.3399272,.13137114),(-.09670098,.33254445,.122700065)]
        for i in range(3):
            for j in range(3):self.assertFalse(forbidden(a[i:]+a[:i],b[j:]+b[:j],1))

    def test_single_vertex_descent_eliminates_known_crossing(self):
        from repair_body_vertex_descent import repair
        points=[(0,0,0),(.01,0,0),(0,.01,0),(.002,.002,-.0001),(.002,.002,.0002),(.008,.002,.0002)]
        candidate,report=repair(points,[(0,1,2),(3,4,5)],points,passes=1)
        self.assertEqual((report['before'],report['after']),(1,0))
        self.assertEqual(sum(p!=q for p,q in zip(points,candidate)),1)
        self.assertLessEqual(report['maximum_cumulative_displacement_m'],.003)
        self.assertFalse(report['audit']['degenerate_triangles'])
        with self.assertRaises(ValueError):repair(points,[(0,1,2)],points[:2])
        for distances in ((),(0.,),(float('nan'),),(float('inf'),)):
            with self.assertRaises(ValueError):repair(points,[(0,1,2)],points,distances=distances)

    def test_vertex_projection_separates_crossing_within_bound(self):
        points=[(0,0,0),(1,0,0),(0,1,0),(.2,.2,-.01),(.2,.2,.01),(.8,.2,0)]
        faces=[(0,1,2),(3,4,5)]
        candidate,report=repair_intersections(points,faces,iterations=3,
            clearance=.001,max_displacement=.03,smoothing_steps=0,
            separation_mode='penetrating_vertices')
        self.assertTrue(report['repair_complete'])
        self.assertLessEqual(report['repair_max_displacement_m'],.03)
        self.assertEqual(candidate[4],points[4])
        self.assertFalse(crossing(candidate[:3],candidate[3:]))
        self.assertFalse(audit(candidate,faces)['degenerate_triangles'])
        with self.assertRaises(ValueError):
            repair_intersections(points,faces,separation_mode='unknown')

    def test_local_repair_support_has_exact_rings_independent_of_face_order(self):
        from repair_body_region import support
        faces=[(0,1,2),(2,3,4),(4,5,6)]
        seeds={0}
        self.assertEqual(support(faces,seeds,0),{0})
        self.assertEqual(support(faces,seeds,1),{0,1,2})
        self.assertEqual(support(faces,seeds,2),{0,1,2,3,4})
        self.assertEqual(support(list(reversed(faces)),seeds,2),{0,1,2,3,4})
        self.assertEqual(seeds,{0})

    def test_local_subdivision_preserves_closed_surface_and_area(self):
        points=[(0,0,0),(1,0,0),(0,1,0),(0,0,1)]
        faces=[(0,2,1),(0,1,3),(1,2,3),(2,0,3)]
        refined,newfaces=split_edges(points,faces,{(0,1)},10)
        before,after=audit(points,faces),audit(refined,newfaces)
        self.assertAlmostEqual(before['area_m2'],after['area_m2'],places=12)
        for key in ('boundary_edges','nonmanifold_edges','inconsistent_winding_edges','duplicate_triangles','degenerate_triangles'):
            self.assertFalse(after[key])
        self.assertEqual(refined[:len(points)],points)
        with self.assertRaises(ValueError):split_edges(points,faces,{(0,1)},9)
        for edge in ((0,0),(0,4)):
            with self.assertRaises(ValueError):split_edges(points,faces,{edge},100)
        with self.assertRaises(ValueError):split_edges(points,[(0,1,2)],{(0,3)},100)
        self.assertEqual(len(points),4)

    def test_repair_rejects_nonfinite_limits(self):
        points=[(0,0,0),(1,0,0),(0,1,0)]
        for value in (float('nan'),float('inf'),-1,0):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    repair_intersections(points,[(0,1,2)],clearance=value)
                with self.assertRaises(ValueError):
                    repair_intersections(points,[(0,1,2)],max_displacement=value)

    def test_repair_reference_preserves_cumulative_displacement_bound(self):
        reference=[(0,0,0),(1,0,0),(0,1,0)]
        points=[(x,y,z+.001) for x,y,z in reference]
        candidate,report=repair_intersections(points,[(0,1,2)],reference_points=reference)
        self.assertEqual(candidate,points)
        self.assertAlmostEqual(report['repair_max_displacement_m'],.001)
        with self.assertRaises(ValueError):
            repair_intersections(points,[(0,1,2)],reference_points=reference,max_displacement=.0005)
        with self.assertRaises(ValueError):
            repair_intersections(points,[(0,1,2)],reference_points=reference[:2])
        for vertices in ({-1},{3},{0.5}):
            with self.assertRaises(ValueError):
                repair_intersections(points,[(0,1,2)],movable_vertices=vertices)

    def test_noncoplanar_crossing_and_separation_across_units(self):
        a=[(0,0,0),(1,0,0),(0,1,0)]
        b=[(.2,.2,-1),(.2,.2,1),(.8,.2,0)]
        apart=[(x+2,y,z) for x,y,z in b]
        for scale in (1e-6,1e-3,1,1e3):
            transform=lambda t:[tuple(x*scale for x in p) for p in t]
            with self.subTest(scale=scale):
                self.assertTrue(crossing(transform(a),transform(b)))
                self.assertFalse(crossing(transform(a),transform(apart)))
                self.assertTrue(segment_triangle((.2*scale,.2*scale,-scale),(.2*scale,.2*scale,scale),transform(a)))

    def test_coplanar_overlap_separation_and_parallel_planes(self):
        a=[(0,0,0),(1,0,0),(0,1,0)]
        b=[(.1,.1,0),(.8,.1,0),(.1,.8,0)]
        for scale in (1e-6,1e-3,1,1e3):
            transform=lambda t:[tuple(x*scale for x in p) for p in t]
            with self.subTest(scale=scale):
                self.assertTrue(crossing(transform(a),transform(b)))
                self.assertFalse(crossing(transform(a),transform([(x+2,y,z) for x,y,z in b])))
                self.assertFalse(crossing(transform(a),transform([(x,y,z+.01) for x,y,z in b])))

if __name__=='__main__':unittest.main()
