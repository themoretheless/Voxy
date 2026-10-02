import unittest
from plot_film_checkpoint import fields, analyze


class FilmCheckpointDiagnosticsTests(unittest.TestCase):
    def test_thickness_uses_three_dimensional_area_even_when_projection_collapses(self):
        areas,thickness,mass=fields([[0.,0.,0.],[1.,0.,0.],[0.,0.,1.]],[[0,1,2]],[1e-6],1000.)
        self.assertAlmostEqual(areas[0],.5)
        self.assertAlmostEqual(thickness[0],2e-6)
        self.assertAlmostEqual(mass,.001)

    def test_invalid_geometry_volumes_indices_and_density_reject(self):
        points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]]
        for triangles,volumes,density in [([[0,1,1]],[1e-6],1000.),([[0,1,3]],[1e-6],1000.),
                ([[0.,1.,2.]],[1e-6],1000.),([[0,1,2]],[-1.],1000.),([[0,1,2]],[float('nan')],1000.),
                ([[0,1,2]],[],1000.),([[0,1,2]],[1e-6],0.)]:
            with self.assertRaises(ValueError):fields(points,triangles,volumes,density)

    def test_capture_format_units_and_mass_are_checked(self):
        snapshot={'capture':'numericalFilmState','filmEnabled':True,'state':{'format':'voxy.surface-film-state.v1','units':'SI',
            'physics':{'pointsM':[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]],'triangles':[[0,1,2]],'cellVolumesM3':[1e-6],
                'material':{'density':1000.}},'measurements':{'massKg':.001}}}
        self.assertAlmostEqual(analyze(snapshot)[2],.001)
        snapshot['state']['measurements']['massKg']=.002
        with self.assertRaisesRegex(ValueError,'mass'):analyze(snapshot)
        snapshot['state']['units']='mm'
        with self.assertRaisesRegex(ValueError,'units'):analyze(snapshot)

    def test_density_does_not_change_volume_based_thickness(self):
        points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]]
        first=fields(points,[[0,1,2]],[1e-6],1000.)
        second=fields(points,[[0,1,2]],[1e-6],2000.)
        self.assertEqual(first[1][0],second[1][0]);self.assertEqual(second[2],2*first[2])
