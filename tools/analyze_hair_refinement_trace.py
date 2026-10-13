#!/usr/bin/env python3
"""Summarize observed contact refinement events; does not prove convergence."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re


def analyze(data):
    temporal_retries = []
    temporal_admissions = []
    profiles = []
    refinement_profiles = []
    transactions = []
    pending = None
    current = []
    nonlinear_steps = []
    nonlinear_admissions = []
    nonlinear_work_budgets = []
    model_deferrals = []
    feasible_prefixes = []
    prefix_checks = []
    elastic_directions = []
    elastic_full_steps = []
    elastic_trials = []
    for line_number, line in enumerate(data.decode('utf-8', errors='replace').splitlines(), 1):
        if line.startswith('HAIR NONLINEAR MERIT iteration='):
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            step = {'line': line_number, 'iteration': int(fields['iteration'])}
            for key in ('before', 'full', 'accepted', 'scale'):
                value = float(fields[key])
                if not math.isfinite(value) or value < 0 or (key == 'scale' and not 0 < value <= 1):
                    raise ValueError(f'invalid nonlinear {key} at line {line_number}')
                step[key] = value
            if fields['tangent_complete'] not in ('true', 'false') or step['iteration'] < 0:
                raise ValueError(f'invalid nonlinear completion at line {line_number}')
            step['tangent_complete'] = fields['tangent_complete'] == 'true'
            nonlinear_steps.append(step)
        elif line.startswith('HAIR NONLINEAR ADMISSION iteration='):
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            event = {'line': line_number, 'iteration': int(fields['iteration'])}
            if event['iteration'] < 0:
                raise ValueError(f'invalid admission iteration at line {line_number}')
            for key in ('tangent_complete', 'strand_admitted', 'unresolved'):
                if fields[key] not in ('true', 'false'):
                    raise ValueError(f'invalid admission {key} at line {line_number}')
                event[key] = fields[key] == 'true'
            gap = fields['worst_gap_m']
            if gap == 'None':
                event['worst_gap_m'] = None
            elif gap.startswith('Some(') and gap.endswith(')'):
                value = float(gap[5:-1])
                if not math.isfinite(value):
                    raise ValueError(f'invalid admission gap at line {line_number}')
                event['worst_gap_m'] = value
            else:
                raise ValueError(f'invalid admission gap at line {line_number}')
            nonlinear_admissions.append(event)
        elif line.startswith('HAIR NONLINEAR WORK BUDGET '):
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            event = {'line': line_number}
            for key in ('attempts', 'accepted_steps', 'attempts_at_pose',
                        'accepted_step_limit', 'fixed_pose_attempt_limit'):
                event[key] = int(fields[key])
                if event[key] < 0:
                    raise ValueError(f'invalid work budget {key} at line {line_number}')
            nonlinear_work_budgets.append(event)
        elif line.startswith('HAIR ELASTIC TRIAL iteration='):
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            event = {'line': line_number, 'iteration': int(fields['iteration'])}
            if event['iteration'] < 0:
                raise ValueError(f'invalid elastic trial iteration at line {line_number}')
            keys = ('scale', 'before_j', 'trial_j', 'global_delta_j', 'rod_delta_sum_j')
            if 'residual_delta_j' in fields:
                keys += ('residual_delta_j',)
            for key in keys:
                value = float(fields[key])
                if not math.isfinite(value) or (key in ('before_j', 'trial_j') and value < 0) or (key == 'scale' and not 0 < value <= 1):
                    raise ValueError(f'invalid elastic trial {key} at line {line_number}')
                event[key] = value
            elastic_trials.append(event)
        elif line.startswith('HAIR ELASTIC DIRECTION iteration=') or line.startswith('HAIR ELASTIC FULL iteration='):
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            event = {'line': line_number, 'iteration': int(fields['iteration'])}
            if event['iteration'] < 0:
                raise ValueError(f'invalid elastic iteration at line {line_number}')
            direction = line.startswith('HAIR ELASTIC DIRECTION')
            keys = ('slope_j', 'full_merit') if direction else ('before_j', 'trial_j', 'delta_j')
            magnitudes = ('max_translation_m', 'max_rotation_rad',
                          'max_velocity_change_m_s', 'max_angular_velocity_change_rad_s')
            if direction and any(key in fields for key in magnitudes):
                if not all(key in fields for key in magnitudes):
                    raise ValueError(f'incomplete elastic direction magnitudes at line {line_number}')
                keys += magnitudes
            for key in keys:
                value = float(fields[key])
                if not math.isfinite(value) or (key not in ('slope_j', 'delta_j') and value < 0):
                    raise ValueError(f'invalid elastic {key} at line {line_number}')
                event[key] = value
            (elastic_directions if direction else elastic_full_steps).append(event)
        elif line.startswith('HAIR CONTACT PREFIX CHECK '):
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            gate = fields.get('gate')
            admitted = fields.get('admitted')
            if gate not in ('prescribed_roots', 'stretch', 'energy', 'strand_path', 'mesh_path', 'complete') or admitted not in ('true', 'false') or (admitted == 'true') != (gate == 'complete'):
                raise ValueError(f'invalid prefix gate at line {line_number}')
            event = {'line': line_number, 'gate': gate, 'admitted': admitted == 'true'}
            if 'change_j' in fields:
                change = float(fields['change_j'])
                event['change_j'] = change if math.isfinite(change) else None
                event['nonfinite_energy'] = not math.isfinite(change)
                if gate != 'energy' and (not math.isfinite(change) or change >= 0):
                    raise ValueError(f'invalid descending prefix energy at line {line_number}')
            if event['admitted']:
                if 'change_j' not in event:
                    raise ValueError(f'missing prefix energy at line {line_number}')
                for key in ('max_translation_m', 'max_rotation_rad'):
                    value = float(fields[key])
                    if not math.isfinite(value) or value < 0:
                        raise ValueError(f'invalid prefix displacement at line {line_number}')
                    event[key] = value
            prefix_checks.append(event)
        elif line.startswith('HAIR CONTACT FEASIBLE PREFIX iteration='):
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            try:
                event = {'line': line_number, **{key: int(fields[key])
                    for key in ('iteration', 'probe_rows', 'probe_pairs')}}
                valid = (all(event[key] >= 0 for key in ('iteration', 'probe_rows', 'probe_pairs'))
                    and event['probe_rows'] + event['probe_pairs'] > 0
                    and fields['tangent_complete'] == 'false')
            except (KeyError, ValueError) as error:
                raise ValueError(f'invalid feasible prefix at line {line_number}') from error
            if not valid:
                raise ValueError(f'invalid feasible prefix at line {line_number}')
            feasible_prefixes.append(event)
        elif line.startswith('HAIR CONTACT APPROACH MODEL iteration='):
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            event = {'line': line_number}
            for key in ('iteration', 'added_rows', 'added_pairs'):
                event[key] = int(fields[key])
                if event[key] < 0:
                    raise ValueError(f'invalid model {key} at line {line_number}')
            if fields['descent_found'] not in ('true', 'false'):
                raise ValueError(f'invalid model descent at line {line_number}')
            event['descent_found'] = fields['descent_found'] == 'true'
            model_deferrals.append(event)
        elif 'HAIR TEMPORAL REFINEMENT retry ' in line:
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            before, after, dt = int(fields['from']), int(fields['to']), float(fields['dt'])
            if not 0 < before < after <= 32 or not math.isfinite(dt) or dt <= 0:
                raise ValueError(f'invalid temporal retry at line {line_number}')
            temporal_retries.append({'line': line_number, 'from': before, 'to': after, 'dt': dt})
        elif 'HAIR TEMPORAL REFINEMENT admitted ' in line:
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            substeps, dt = int(fields['substeps']), float(fields['dt'])
            if not 0 < substeps <= 32 or not math.isfinite(dt) or dt <= 0:
                raise ValueError(f'invalid temporal admission at line {line_number}')
            temporal_admissions.append({'line': line_number, 'substeps': substeps, 'dt': dt})
        elif 'HAIR SWEPT CUT UPDATE ' in line:
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            pending = {key: int(fields[key]) for key in ('added', 'clock_changes', 'cuts')}
            for key in ('minimum_scale', 'minimum_time'):
                value = float(fields[key])
                if not math.isfinite(value) or not 0 < value <= 1:
                    raise ValueError(f'invalid {key} at line {line_number}')
                pending[key] = value
            pending['line'] = line_number
        elif 'HAIR SWEPT REFINEMENT count=' in line:
            count = int(re.search(r'count=(\d+)', line)[1])
            if current and count <= current[-1]['refinement']:
                transactions.append(current)
                current = []
            event = {'refinement': count, 'line': line_number, 'admission': line.split('constraints=', 1)[1].split(' ', 1)[1]}
            if pending is not None:
                event['cut_update'] = pending
                pending = None
            current.append(event)
        elif 'HAIR SWEPT REFINEMENT PROFILE ' in line:
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            profile = {'line': line_number}
            for key in ('witness_ms', 'projection_ms', 'admission_ms'):
                value = float(fields[key])
                if not math.isfinite(value) or value < 0:
                    raise ValueError(f'invalid {key} at line {line_number}')
                profile[key] = value
            refinement_profiles.append(profile)
        elif 'HAIR SWEPT SOLVE PROFILE ' in line:
            fields = dict(re.findall(r'(\w+)=([^ ]+)', line))
            profile = {'line': line_number}
            for key in ('elapsed_ms', 'initial_projection_ms', 'initial_admission_ms'):
                value = float(fields[key])
                if not math.isfinite(value) or value < 0:
                    raise ValueError(f'invalid {key} at line {line_number}')
                profile[key] = value
            setup = float(fields.get('initial_setup_ms', 0))
            if not math.isfinite(setup) or setup < 0:
                raise ValueError(f'invalid initial_setup_ms at line {line_number}')
            profile['initial_setup_ms'] = setup
            profile['setup_separately_measured'] = 'initial_setup_ms' in fields
            profile['refinements'] = int(fields['refinements'])
            profile['refinement_and_final_check_ms'] = max(0., profile['elapsed_ms']
                - setup - profile['initial_projection_ms'] - profile['initial_admission_ms'])
            profiles.append(profile)
            if current:
                transactions.append(current)
                current = []
    if current:
        transactions.append(current)
    events = [event for transaction in transactions for event in transaction]
    updates = [event['cut_update'] for event in events if 'cut_update' in event]
    # Attempt indices restart for each local solve, potentially many times
    # inside a single physical frame. Admission repeats the matching attempt
    # index, so only merit and model-refinement events establish boundaries.
    nonlinear_solves = []
    primary = sorted([(event, 'accepted') for event in nonlinear_steps]
                     + [(event, 'refinement') for event in model_deferrals],
                     key=lambda item: item[0]['line'])
    for index, prefix in enumerate(feasible_prefixes):
        boundary = feasible_prefixes[index + 1]['line'] if index + 1 < len(feasible_prefixes) else math.inf
        following = next(((event, kind) for event, kind in primary
            if prefix['line'] < event['line'] < boundary), None)
        prefix['following_accepted_step'] = None
        if following is not None:
            step, kind = following
            if kind == 'accepted' and step['iteration'] == prefix['iteration']:
                if step['before'] != 0 or step['accepted'] != 0 or step['tangent_complete']:
                    raise ValueError(f'contradictory feasible prefix at line {prefix["line"]}')
                prefix['following_accepted_step'] = step
        # An isolated prefix log can end before the pose update is recorded.
        # Neither event establishes final tangent completion or frame publication.
    for event, kind in primary:
        if not nonlinear_solves or event['iteration'] <= nonlinear_solves[-1]['last_attempt']:
            nonlinear_solves.append({'first_line': event['line'], 'last_line': event['line'],
                                     'first_observed_attempt': event['iteration'],
                                     'last_attempt': event['iteration'],
                                     'accepted_moves': 0, 'model_refinements': 0,
                                     'minimum_accepted_scale': None})
        solve = nonlinear_solves[-1]
        solve['last_line'] = event['line']
        solve['last_attempt'] = event['iteration']
        if kind == 'accepted':
            solve['accepted_moves'] += 1
            previous = solve['minimum_accepted_scale']
            solve['minimum_accepted_scale'] = event['scale'] if previous is None else min(previous, event['scale'])
            solve['last_accepted_step'] = event
        else:
            solve['model_refinements'] += 1
    for index, solve in enumerate(nonlinear_solves):
        next_line = nonlinear_solves[index + 1]['first_line'] if index + 1 < len(nonlinear_solves) else math.inf
        solve['last_observed_admission'] = next((event for event in reversed(nonlinear_admissions)
            if solve['first_line'] <= event['line'] < next_line), None)
        solve['terminal_work_budgets'] = [event for event in nonlinear_work_budgets
            if solve['first_line'] <= event['line'] < next_line]
        # A new local solve is not proof that the preceding frame was published.
    return {'scope': 'observed refinement trace only; no physical feasibility or full-cycle proof',
            'log_sha256': hashlib.sha256(data).hexdigest(), 'transactions': transactions,
            'nonlinear_accepted_steps': nonlinear_steps,
            'nonlinear_local_solves': nonlinear_solves,
            'nonlinear_model_deferrals': model_deferrals,
            'prefix_checks': prefix_checks,
            'prefix_rejection_gates': {gate: sum(not event['admitted'] and event['gate'] == gate for event in prefix_checks) for gate in ('prescribed_roots', 'stretch', 'energy', 'strand_path', 'mesh_path')},
            'observed_feasible_prefixes': feasible_prefixes,
            'prefixes_with_following_pose_updates': sum(event['following_accepted_step'] is not None for event in feasible_prefixes),
            'prefixes_without_following_pose_update': sum(event['following_accepted_step'] is None for event in feasible_prefixes),
            'nonlinear_admissions': nonlinear_admissions,
            'nonlinear_work_budgets': nonlinear_work_budgets,
            'geometry_admitted_tangent_incomplete_events': sum(
                not event['unresolved'] and not event['tangent_complete']
                for event in nonlinear_admissions),
            'elastic_directions': elastic_directions,
            'elastic_full_steps': elastic_full_steps,
            'elastic_trials': elastic_trials,
            'trials_with_residual_work': sum('residual_delta_j' in event for event in elastic_trials),
            'residual_descents_hidden_by_total_energy': sum(event.get('residual_delta_j', 0) < 0 and event['global_delta_j'] >= 0 for event in elastic_trials),
            'directions_with_measured_magnitudes': sum('max_translation_m' in event for event in elastic_directions),
            'positive_elastic_direction_slopes': sum(event['slope_j'] > 0 for event in elastic_directions),
            'observed_uphill_full_steps': sum(event['delta_j'] > 0 for event in elastic_full_steps),
            'model_deferrals_with_trial_descent': sum(event['descent_found'] for event in model_deferrals),
            'feasible_elastic_accepted_steps': sum(step['before'] == 0 and step['accepted'] == 0 for step in nonlinear_steps),
             'completed_profile_count': len(profiles),
            'setup_separately_measured_profile_count': sum(p['setup_separately_measured'] for p in profiles),
            'detailed_refinement_profile_count': len(refinement_profiles),
            'detailed_refinement_totals_ms': {key: sum(profile[key] for profile in refinement_profiles)
                for key in ('witness_ms', 'projection_ms', 'admission_ms')},
            'profile_stage_totals_ms': {key: sum(profile[key] for profile in profiles)
                for key in ('elapsed_ms', 'initial_setup_ms', 'initial_projection_ms', 'initial_admission_ms', 'refinement_and_final_check_ms')},
            'slowest_completed_profile': max(profiles, key=lambda profile: profile['elapsed_ms'], default=None),
             'temporal_retries': temporal_retries, 'temporal_admissions': temporal_admissions,
            'refinement_events': len(events), 'unpaired_cut_update': pending,
            'clock_only_events': sum(update['added'] == 0 and update['clock_changes'] > 0 for update in updates),
            'minimum_observed_trust_scale': min((update['minimum_scale'] for update in updates), default=None)}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('log', type=Path)
    args = parser.parse_args()
    print(json.dumps(analyze(args.log.read_bytes()), indent=2))
