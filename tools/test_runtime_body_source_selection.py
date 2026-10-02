import tempfile
import unittest
from pathlib import Path
from audit_runtime_body_geometry import runtime_sources


class RuntimeSourceSelectionTests(unittest.TestCase):
    def test_reads_adopted_mesh_instead_of_historical_name(self):
        with tempfile.TemporaryDirectory() as directory:
            source=Path(directory)/'female_demo.rs'
            source.write_text('const BODY: &str = include_str!("updated-female.obj");\n'
                              'pub(crate) fn new_male() -> Result<Self, Error> { Self::male_from_assets(\n'
                              'include_str!("updated-male.obj"), include_str!("skin.json")) }')
            self.assertEqual(runtime_sources(source),{
                'female':(source.parent/'updated-female.obj').resolve(),
                'male':(source.parent/'updated-male.obj').resolve()})

    def test_unknown_declarations_fail_without_historical_fallback(self):
        with tempfile.TemporaryDirectory() as directory:
            source=Path(directory)/'female_demo.rs'
            source.write_text('const BODY: &str = make_body();')
            with self.assertRaises(ValueError):runtime_sources(source)


if __name__=='__main__':unittest.main()
