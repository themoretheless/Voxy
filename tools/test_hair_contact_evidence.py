"""Solver-independent regression checks for exact contact evidence tools."""
from fractions import Fraction
import json
from pathlib import Path
import struct
import tempfile
import unittest

from analyze_hair_refinement_trace import analyze
from bound_hair_contact_translation import bound
from verify_hair_qr_infeasibility import verify
from summarize_hair_joint_stages import summarize, PREFIX, SCHEMA, SCOPE, STAGES, COUNTS


class ContactEvidenceTests(unittest.TestCase):
    def test_seed_profile_distinguishes_input_upload_and_legacy_unknown(self):
        record = {"schema": SCHEMA, "scope": SCOPE, **dict.fromkeys(COUNTS, 0),
                  "owners": 3, "stages": dict.fromkeys((*STAGES, "total_wall_ms", "readback_payload_bytes"), 0)}
        seeded = {**record, "incoming_valid_seeded_owners": 3,
                  "incoming_positive_seed_rows": 20, "seeded_upload_owners": 1}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "seeds.log"
            path.write_text(PREFIX + json.dumps(record) + "\n" + PREFIX + json.dumps(seeded) + "\n")
            counts = summarize([path])["warm_start"]["counters"]
            self.assertEqual(counts["seeded_upload_owners"], {"instances": 1, "reported_calls": 1, "unreported_calls": 1})
            self.assertEqual(counts["incoming_positive_seed_rows"]["instances"], 20)
            for change in [{"seeded_upload_owners": 4}, {"incoming_valid_seeded_owners": 0},
                           {"incoming_positive_seed_rows": True}, {"seeded_upload_owners": -1}]:
                path.write_text(PREFIX + json.dumps({**seeded, **change}) + "\n")
                with self.subTest(change=change), self.assertRaises(ValueError):
                    summarize([path])

    def test_workspace_payload_is_separate_from_compact_readback_and_legacy_unknown(self):
        record = {"schema": SCHEMA, "scope": SCOPE, **dict.fromkeys(COUNTS, 0),
                  "stages": dict.fromkeys((*STAGES, "total_wall_ms", "readback_payload_bytes"), 0)}
        record["stages"]["readback_payload_bytes"] = 40
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "profile.log"
            path.write_text(PREFIX + json.dumps(record) + "\n" + PREFIX +
                            json.dumps({**record, "complete_workspace_payload_bytes": 100}) + "\n")
            result = summarize([path])
            self.assertEqual(result["staging_payload_bytes_not_bus_traffic"], 80)
            workspace = result["complete_workspace_payload"]
            self.assertEqual(workspace["bytes_over_profiled_calls_not_peak_memory"], 100)
            self.assertEqual(workspace["profiled_calls"], 1)
            self.assertEqual(workspace["unreported_calls"], 1)
            for value in [-1, True, 1.5, None]:
                path.write_text(PREFIX + json.dumps({**record, "complete_workspace_payload_bytes": value}) + "\n")
                with self.subTest(value=value), self.assertRaises(ValueError):
                    summarize([path])

    def test_full_cycle_profiles_assign_each_frame_and_preserve_incomplete_work(self):
        record = {"schema": SCHEMA, "scope": SCOPE, **dict.fromkeys(COUNTS, 0),
                  "stages": dict.fromkeys((*STAGES, "total_wall_ms", "readback_payload_bytes"), 0)}
        profile = PREFIX + json.dumps(record) + "\n"
        text = ("AUTHORED FULL CYCLE BEGIN frame=1/720 completed=0\n" + profile
                + "AUTHORED FULL CYCLE frame=1/720 time_s=0.008333 elapsed_ms=10\n"
                + "AUTHORED FULL CYCLE BEGIN frame=2/720 completed=1\n" + profile)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "full.log"
            path.write_text(text)
            frames = summarize([path])["coupled_frames"]
        self.assertEqual([f["frame"] for f in frames], [1, 2])
        self.assertEqual(frames[0]["status"], "completed_as_reported_by_coupled_test")
        self.assertEqual(frames[1]["status"], "incomplete")
        self.assertEqual([f["profiled_calls"] for f in frames], [1, 1])

    def test_gpu_profile_frames_require_explicit_completion_and_keep_partial_frame(self):
        stages = dict.fromkeys((*STAGES, "total_wall_ms", "readback_payload_bytes"), 0)
        stages.update(wait_wall_ms=2, total_wall_ms=3, readback_payload_bytes=40)
        record = {"schema": SCHEMA, "scope": SCOPE,
                  **dict.fromkeys(COUNTS, 0), "stages": stages}
        profile = PREFIX + json.dumps(record) + "\n"
        log = (profile + "COUPLED HAIR GPU BEGIN frame=1 bounded_frames=2\n" + profile
               + "COUPLED HAIR GPU passed_frames=1 bounded_frames=2\n"
               + "COUPLED HAIR GPU BEGIN frame=2 bounded_frames=2\n" + profile)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "partial.log"
            path.write_text(log)
            result = summarize([path])
        self.assertEqual(result["profiled_calls"], 3)
        first, second = result["coupled_frames"]
        self.assertEqual(first["status"], "completed_as_reported_by_coupled_test")
        self.assertEqual(second["status"], "incomplete")
        self.assertEqual(first["profiled_calls"], 1)
        self.assertEqual(second["profile_wait_wall_ms"], 2)
        self.assertEqual(first["staging_payload_bytes_not_bus_traffic"], 40)
        self.assertEqual(second["staging_payload_bytes_not_bus_traffic"], 40)
        self.assertEqual(first["workspace_payload_reported_calls"], 0)
        self.assertEqual(second["workspace_payload_reported_calls"], 0)

    def test_gpu_profile_batch_timings_keep_scope_and_bound_retained_records(self):
        records = []
        for duration in range(15):
            stages = dict.fromkeys((*STAGES, "total_wall_ms", "readback_payload_bytes"), 0)
            stages.update(total_wall_ms=duration, wait_wall_ms=duration)
            records.append({"schema": SCHEMA, "scope": SCOPE,
                            **dict.fromkeys(COUNTS, 0), "owners": 2, "stages": stages,
                            "submitted_operator_shapes_rows_coordinates": [[3, 12], [5, 24]]})
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "mixed.log"
            path.write_text("".join(PREFIX + json.dumps(record) + "\n" for record in records))
            result = summarize([path])
        batches = result["slowest_profiled_batches"]
        self.assertEqual(len(batches), 10)
        self.assertEqual(batches[0]["line"], 15)
        self.assertEqual(batches[0]["wait_wall_ms"], 14)
        self.assertEqual(batches[0]["packed_operator_shapes_rows_coordinates"], [[3, 12], [5, 24]])
        self.assertEqual(result["wall_ms"]["wait_wall_ms"], sum(range(15)))
        self.assertEqual(result["packed_operator_shape_instances"][0]["instances"], 15)

    def test_gpu_profile_shapes_preserve_legacy_and_reject_malformed_dimensions(self):
        record = {"schema": SCHEMA, "scope": SCOPE,
                  **dict.fromkeys(COUNTS, 0),
                  "stages": dict.fromkeys((*STAGES, "total_wall_ms", "readback_payload_bytes"), 0)}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "profile.log"
            def check(value):
                path.write_text(PREFIX + json.dumps(value) + "\n")
                return summarize([path])
            self.assertEqual(check(record)["packed_operator_shape_instances"], [])
            record["submitted_operator_shapes_rows_coordinates"] = [[3, 12], [3, 12], [1, 6]]
            self.assertEqual(check(record)["packed_operator_shape_instances"], [
                {"rows": 1, "coordinates": 6, "instances": 1},
                {"rows": 3, "coordinates": 12, "instances": 2}])
            for shapes in ([[0, 12]], [[3, -1]], [[True, 12]], [[3]], "3x12"):
                record["submitted_operator_shapes_rows_coordinates"] = shapes
                with self.subTest(shapes=shapes), self.assertRaises(ValueError):
                    check(record)

    def test_prefix_gate_trace_keeps_rejection_and_motion_scope(self):
        trace = (b'HAIR CONTACT PREFIX CHECK admitted=false gate=mesh_path change_j=-1e-20 error="collision"\n'
                 b'HAIR CONTACT PREFIX CHECK admitted=true gate=complete change_j=-1e-21 max_translation_m=1e-12 max_rotation_rad=1e-8\n')
        result = analyze(trace)
        self.assertEqual(result['prefix_rejection_gates']['mesh_path'], 1)
        self.assertEqual(result['prefix_checks'][1]['max_translation_m'], 1e-12)
        self.assertEqual(result['nonlinear_accepted_steps'], [])
        invalid = b'HAIR CONTACT PREFIX CHECK admitted=false gate=energy change_j=NaN\n'
        self.assertTrue(analyze(invalid)['prefix_checks'][0]['nonfinite_energy'])
        for malformed in (trace.replace(b'change_j=-1e-20', b'change_j=1e-20'),
                          trace.replace(b'admitted=false gate=mesh_path', b'admitted=true gate=mesh_path')):
            with self.subTest(trace=malformed), self.assertRaises(ValueError):
                analyze(malformed)

    def test_prefix_trace_is_not_final_tangent_completion(self):
        prefix = b'HAIR CONTACT FEASIBLE PREFIX iteration=6 probe_rows=3 probe_pairs=0 tangent_complete=false\n'
        step = b'HAIR NONLINEAR MERIT iteration=6 before=0 full=1 accepted=0 scale=0.25 tangent_complete=false\n'
        pending = analyze(prefix)
        self.assertEqual(pending['prefixes_without_following_pose_update'], 1)
        self.assertEqual(pending['nonlinear_accepted_steps'], [])
        observed = analyze(prefix + step)
        self.assertEqual(observed['prefixes_with_following_pose_updates'], 1)
        self.assertEqual(observed['nonlinear_local_solves'][0]['accepted_moves'], 1)
        self.assertEqual(observed['nonlinear_admissions'], [])
        for malformed in (prefix.replace(b'probe_rows=3', b'probe_rows=-1'),
                          prefix.replace(b'tangent_complete=false', b'tangent_complete=true'),
                          prefix + step.replace(b'accepted=0', b'accepted=1')):
            with self.subTest(trace=malformed), self.assertRaises(ValueError):
                analyze(malformed)
        refined = analyze(prefix + b'HAIR CONTACT APPROACH MODEL iteration=7 added_rows=1 added_pairs=0 descent_found=true\n' + step)
        self.assertEqual(refined['prefixes_without_following_pose_update'], 1)

    def test_nonlinear_trace_separates_geometry_from_tangent_completion(self):
        trace = (b'HAIR NONLINEAR ADMISSION iteration=9 tangent_complete=false '
                 b'strand_admitted=true unresolved=false worst_gap_m=Some(-1e-12)\n'
                 b'HAIR NONLINEAR WORK BUDGET attempts=128 accepted_steps=31 '
                 b'attempts_at_pose=3 accepted_step_limit=128 fixed_pose_attempt_limit=128\n')
        result = analyze(trace)
        self.assertEqual(result['geometry_admitted_tangent_incomplete_events'], 1)
        self.assertEqual(result['nonlinear_work_budgets'][0]['accepted_steps'], 31)
        for malformed in (trace.replace(b'-1e-12', b'nan'),
                          trace.replace(b'strand_admitted=true', b'strand_admitted=unknown')):
            with self.subTest(trace=malformed), self.assertRaises(ValueError):
                analyze(malformed)

    def test_trace_local_solve_boundary_can_start_with_model_refinement(self):
        trace = (b'HAIR NONLINEAR MERIT iteration=9 before=0 full=1 accepted=0 scale=0.5 tangent_complete=false\n'
                 b'HAIR NONLINEAR ADMISSION iteration=9 tangent_complete=false strand_admitted=true unresolved=false worst_gap_m=None\n'
                 b'HAIR CONTACT APPROACH MODEL iteration=0 added_rows=1 added_pairs=0 descent_found=true\n'
                 b'HAIR NONLINEAR MERIT iteration=1 before=1 full=0 accepted=0 scale=1 tangent_complete=true\n')
        solves = analyze(trace)['nonlinear_local_solves']
        self.assertEqual(len(solves), 2)
        self.assertEqual(solves[0]['accepted_moves'], 1)
        self.assertEqual(solves[0]['last_observed_admission']['iteration'], 9)
        self.assertIsNone(solves[1]['last_observed_admission'])
        self.assertEqual(solves[1]['model_refinements'], 1)
        self.assertEqual(solves[1]['first_observed_attempt'], 0)

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.capture = Path(self.directory.name) / 'input.vqc'
        self.weights = Path(self.directory.name) / 'weights.json'

    def write_capture(self, a, b, rhs=(1., 1.)):
        # Two rows, one six-coordinate point, identity diagonal band storage.
        matrix = [value for i in range(6) for value in [1.] + [0.] * 8]
        self.capture.write_bytes(
            b'VQC1' + struct.pack('<IIIId', 2, 6, 1, 0, 1e-14)
            + struct.pack('<4d', *rhs, *rhs)
            + struct.pack('<12d', *(a + b))
            + struct.pack('<IIII', 6, 9, 0, 6)
            + struct.pack('<72d', *(matrix + [0.] * 6 + a + b)))

    def witness(self, weights):
        self.weights.write_text(json.dumps(weights))
        return verify(self.capture, self.weights)

    def test_exact_opposite_positive_rows_prove_linear_infeasibility(self):
        self.write_capture([1., 0, 0, 0, 0, 0], [-1., 0, 0, 0, 0, 0])
        self.assertTrue(self.witness(['1/2', '1/2'])['exact_original_linear_infeasibility'])
        self.assertFalse(self.witness(['1', '2'])['exact_original_linear_infeasibility'])

    def test_invalid_dual_weights_are_rejected(self):
        self.write_capture([1., 0, 0, 0, 0, 0], [-1., 0, 0, 0, 0, 0])
        for weights in (['0', '0'], ['-1', '-1'], ['1']):
            with self.subTest(weights=weights), self.assertRaises(ValueError):
                self.witness(weights)

    def test_nearly_opposite_rows_are_not_exact_dual_certificate(self):
        self.write_capture([1., 0, 0, 0, 0, 0], [-1., 2.**-40, 0, 0, 0, 0])
        result = self.witness(['1/2', '1/2'])
        self.assertFalse(result['exact_original_linear_infeasibility'])
        self.assertEqual(result['nonzero_transpose_coordinates'], 1)
        result = bound(self.capture, 0, 1)
        self.assertEqual(Fraction(result['maximum_translation_coordinate_lower_bound_m']), 2**41)

    def test_known_translation_bound(self):
        self.write_capture([1., 0, 0, 0, 0, 0], [-.5, 0, 0, 0, 0, 0])
        self.assertEqual(Fraction(bound(self.capture, 0, 1)
                                  ['maximum_translation_coordinate_lower_bound_m']), 4)
        self.assertEqual(bound(self.capture, 1, 0),
                         dict(bound(self.capture, 0, 1), rows=[1, 0]))

    def test_angular_load_zero_slope_and_invalid_indices_rejected(self):
        self.write_capture([1., 0, 0, 1., 0, 0], [-.5, 0, 0, 0, 0, 0])
        with self.assertRaises(ValueError):
            bound(self.capture, 0, 1)
        self.write_capture([1., 0, 0, 0, 0, 0], [-1., 0, 0, 0, 0, 0])
        for pair in ((0, 1), (0, 0), (-1, 1), (0, 2)):
            with self.subTest(pair=pair), self.assertRaises(ValueError):
                bound(self.capture, *pair)


if __name__ == '__main__':
    unittest.main()
