//! Timed linear phase merging and curved/rigid root-motion trigger synchronization.

use crate::character_physics::AcceptedRotationArc;
use crate::convex::{AffineBox, reframe_rotation, rotate_vector};
use crate::{PhysicsError, SupportQueryBudget, TriggerEvent, TriggerPhase};
use glam::{DQuat, DVec3};
use voxy_animation::{RootRigidPath, RootRotationPath};
use voxy_scene::NodeId;

/// Accepted rotational or curved rigid root-motion phase within a fixed physics tick.
#[derive(Clone, Debug)]
pub(crate) enum AcceptedCurvedMotion {
    Arc(AcceptedRotationArc),
    RotationPath {
        start_center: DVec3,
        start_orientation: DQuat,
        basis: DQuat,
        pivot: DVec3,
        path: RootRotationPath,
        completed_spans: usize,
        span_fraction: f64,
        stop: f64,
    },
    RigidPath {
        start_center: DVec3,
        start_orientation: DQuat,
        basis: DQuat,
        origin: DVec3,
        scale: f64,
        path: RootRigidPath,
        completed_spans: usize,
        span_fraction: f64,
        stop: f64,
    },
}

impl AcceptedCurvedMotion {
    pub(crate) const fn start_center(&self) -> DVec3 {
        match self {
            Self::Arc(arc) => arc.pivot,
            Self::RotationPath { start_center, .. } | Self::RigidPath { start_center, .. } => {
                *start_center
            }
        }
    }
}

/// Combines timed controller slide/step/snap segments with optional timed root translation.
pub(crate) fn merge_timed_translation_phases(
    segments: &mut Vec<DVec3>,
    end_fractions: &mut Vec<f64>,
    root_segments: &[(DVec3, Option<f64>)],
) {
    if root_segments.is_empty() {
        return;
    }
    let untimed_prefix = !segments.is_empty() && end_fractions.len() != segments.len();
    let untimed_root = root_segments.iter().any(|(_, end)| end.is_none());
    if untimed_prefix || untimed_root {
        segments.extend(root_segments.iter().map(|(seg, _)| *seg));
        end_fractions.clear();
        return;
    }
    if segments.is_empty() {
        for &(seg, end) in root_segments {
            segments.push(seg);
            end_fractions.push(end.unwrap_or(1.));
        }
        enforce_strict_fractions(end_fractions, 1.);
        return;
    }
    let ctrl_len: f64 = segments.iter().map(|s| s.length()).sum();
    let root_len: f64 = root_segments.iter().map(|(s, _)| s.length()).sum();
    let split = if ctrl_len + root_len > 0. {
        (ctrl_len / (ctrl_len + root_len)).clamp(f64::MIN_POSITIVE, 1.0_f64.next_down())
    } else {
        0.5
    };
    for frac in end_fractions.iter_mut() {
        *frac = (*frac * split).clamp(f64::MIN_POSITIVE, split);
    }
    enforce_strict_fractions(end_fractions, split);
    for &(seg, end) in root_segments {
        let scaled = split + end.unwrap_or(1.) * (1. - split);
        segments.push(seg);
        end_fractions.push(scaled);
    }
    let final_cap = root_segments
        .last()
        .and_then(|(_, f)| *f)
        .unwrap_or(1.)
        .clamp(f64::MIN_POSITIVE, 1.);
    enforce_strict_fractions(end_fractions, split + final_cap * (1. - split));
}

fn enforce_strict_fractions(fractions: &mut [f64], upper: f64) {
    let n = fractions.len();
    if n == 0 {
        return;
    }
    fractions[n - 1] = fractions[n - 1].min(upper).clamp(f64::MIN_POSITIVE, upper);
    for i in (0..n - 1).rev() {
        fractions[i] = fractions[i].min(fractions[i + 1].next_down());
    }
    fractions[0] = fractions[0].max(f64::MIN_POSITIVE);
    for i in 1..n {
        if fractions[i] <= fractions[i - 1] {
            fractions[i] = fractions[i - 1].next_up().min(upper);
        }
    }
}

pub(crate) struct CurvedMotionInput<'a> {
    pub(crate) initial: AffineBox,
    pub(crate) motion: Option<&'a AcceptedCurvedMotion>,
    pub(crate) post_snap: DVec3,
}

#[derive(Clone, Copy)]
struct StepEvent {
    time_u: f64,
    is_moving: bool,
    span_idx: usize,
    limit: f64,
}

struct CurvedView<'a> {
    initial: AffineBox,
    motion: Option<&'a AcceptedCurvedMotion>,
    corners_body: [DVec3; 8],
    cut_span: usize,
    cut_frac: f64,
    cut_time_u: f64,
}

impl<'a> CurvedView<'a> {
    fn new(initial: AffineBox, motion: Option<&'a AcceptedCurvedMotion>) -> Self {
        let (corners_body, cut_span, cut_frac, cut_time_u) = match motion {
            None => ([DVec3::ZERO; 8], 0, 0., 0.),
            Some(AcceptedCurvedMotion::Arc(arc)) => {
                let c = box_corners(initial.center - arc.pivot, initial.edges);
                (c, 0, arc.stop, arc.stop)
            }
            Some(AcceptedCurvedMotion::RotationPath {
                start_center,
                start_orientation,
                path,
                completed_spans,
                span_fraction,
                stop,
                ..
            }) => {
                let c = box_corners(initial.center - *start_center, initial.edges)
                    .map(|v| rotate_vector(start_orientation.conjugate(), v));
                let (cs, cf) = resolve_cutoff(path.spans().len(), *completed_spans, *span_fraction);
                (c, cs, cf, *stop)
            }
            Some(AcceptedCurvedMotion::RigidPath {
                start_center,
                start_orientation,
                path,
                completed_spans,
                span_fraction,
                stop,
                ..
            }) => {
                let c = box_corners(initial.center - *start_center, initial.edges)
                    .map(|v| rotate_vector(start_orientation.conjugate(), v));
                let (cs, cf) = resolve_cutoff(path.spans().len(), *completed_spans, *span_fraction);
                (c, cs, cf, *stop)
            }
        };
        Self {
            initial,
            motion,
            corners_body,
            cut_span,
            cut_frac,
            cut_time_u,
        }
    }

    fn pose_at_span(&self, span_idx: usize, local_frac: f64) -> Result<AffineBox, PhysicsError> {
        let Some(motion) = self.motion else {
            return Ok(self.initial);
        };
        match motion {
            AcceptedCurvedMotion::Arc(arc) => {
                let rot = DQuat::from_scaled_axis(arc.requested * local_frac.min(arc.stop));
                Ok(AffineBox {
                    center: arc.pivot + rotate_vector(rot, self.initial.center - arc.pivot),
                    edges: self.initial.edges.map(|e| rotate_vector(rot, e)),
                })
            }
            AcceptedCurvedMotion::RotationPath {
                start_center,
                start_orientation,
                basis,
                pivot,
                path,
                ..
            } => {
                let Some(span) = path.spans().get(span_idx) else {
                    return Ok(self.initial);
                };
                let q = span
                    .sample(local_frac)
                    .map_err(|_| PhysicsError::InvalidMotion)?;
                let body_rot = reframe_rotation(*basis, q);
                let body_disp =
                    rotate_vector(*start_orientation, *pivot - rotate_vector(body_rot, *pivot));
                Ok(offset_box(
                    self.initial,
                    *start_center,
                    *start_orientation,
                    body_disp,
                    body_rot,
                ))
            }
            AcceptedCurvedMotion::RigidPath {
                start_center,
                start_orientation,
                basis,
                origin,
                scale,
                path,
                ..
            } => {
                let Some(span) = path.spans().get(span_idx) else {
                    return Ok(self.initial);
                };
                let tr = span
                    .sample(local_frac)
                    .map_err(|_| PhysicsError::InvalidMotion)?;
                let body_rot = reframe_rotation(*basis, tr.rotation);
                let body_disp = rotate_vector(
                    *start_orientation,
                    *scale * rotate_vector(*basis, tr.translation) + *origin
                        - rotate_vector(body_rot, *origin),
                );
                Ok(offset_box(
                    self.initial,
                    *start_center,
                    *start_orientation,
                    body_disp,
                    body_rot,
                ))
            }
        }
    }

    fn locate_smooth(&self, u_left: f64, u_right: f64) -> Option<(usize, f64, f64)> {
        let motion = self.motion?;
        match motion {
            AcceptedCurvedMotion::Arc(_) => {
                if u_left >= self.cut_time_u {
                    None
                } else {
                    let end = u_right.min(self.cut_time_u);
                    Some((0, u_left, end))
                }
            }
            AcceptedCurvedMotion::RotationPath { path, .. } => locate_in_spans(
                path.duration(),
                self.cut_span,
                self.cut_frac,
                self.cut_time_u,
                path.spans().iter().map(|s| (s.start(), s.end())),
                u_left,
                u_right,
            ),
            AcceptedCurvedMotion::RigidPath { path, .. } => locate_in_spans(
                path.duration(),
                self.cut_span,
                self.cut_frac,
                self.cut_time_u,
                path.spans().iter().map(|s| (s.start(), s.end())),
                u_left,
                u_right,
            ),
        }
    }

    fn pose_on_smooth(&self, u_left: f64, u_right: f64, u: f64) -> Result<AffineBox, PhysicsError> {
        let Some(motion) = self.motion else {
            return Ok(self.initial);
        };
        if matches!(motion, AcceptedCurvedMotion::Arc(_)) {
            return self.pose_at_span(0, u);
        }
        if let Some((idx, s, e)) = self.locate_smooth(u_left, u_right) {
            let local = ((u - s) / (e - s)).clamp(0., 1.);
            return self.pose_at_span(idx, local);
        }
        if u_left >= self.cut_time_u || u_right > self.cut_time_u {
            return self.pose_at_span(self.cut_span, self.cut_frac);
        }
        let prev = self.last_span_by(u_left);
        match prev {
            Some(idx) => self.pose_at_span(idx, 1.0),
            None => Ok(self.initial),
        }
    }

    fn last_span_by(&self, u: f64) -> Option<usize> {
        let (dur, spans): (f64, Vec<(f64, f64)>) = match self.motion? {
            AcceptedCurvedMotion::RotationPath { path, .. } => (
                path.duration(),
                path.spans().iter().map(|s| (s.start(), s.end())).collect(),
            ),
            AcceptedCurvedMotion::RigidPath { path, .. } => (
                path.duration(),
                path.spans().iter().map(|s| (s.start(), s.end())).collect(),
            ),
            AcceptedCurvedMotion::Arc(_) => return Some(0),
        };
        let mut last = None;
        for (i, (s, e)) in spans.into_iter().enumerate() {
            if i > self.cut_span || (i == self.cut_span && self.cut_frac == 0.) {
                break;
            }
            if dur <= 0. || e <= u * dur || s < u * dur {
                last = Some(i);
            }
        }
        last
    }

    fn span_travel(&self, span_idx: usize) -> Result<f64, PhysicsError> {
        let Some(motion) = self.motion else {
            return Ok(0.);
        };
        match motion {
            AcceptedCurvedMotion::Arc(arc) => {
                let rad = self
                    .corners_body
                    .iter()
                    .map(|c| c.length())
                    .fold(0_f64, f64::max);
                Ok(arc.requested.length() * rad)
            }
            AcceptedCurvedMotion::RotationPath {
                basis, pivot, path, ..
            } => {
                let span = &path.spans()[span_idx];
                let dt = span.end() - span.start();
                let mut bound = 0_f64;
                for c in self.corners_body {
                    let v_src = rotate_vector(basis.conjugate(), c - *pivot);
                    let spd = span
                        .point_speed_bound(v_src)
                        .map_err(|_| PhysicsError::InvalidMotion)?;
                    let corner_travel = match spd {
                        Some(s) => s * dt,
                        None => span
                            .body_angular_displacement()
                            .map_or(0., |ax| ax.cross(v_src).length()),
                    };
                    bound = bound.max(corner_travel);
                }
                Ok(bound)
            }
            AcceptedCurvedMotion::RigidPath {
                basis,
                origin,
                scale,
                path,
                ..
            } => {
                let span = &path.spans()[span_idx];
                let mut bound = 0_f64;
                for c in self.corners_body {
                    let travel = if *scale == 0. {
                        let v_src = rotate_vector(basis.conjugate(), c - *origin);
                        let dt = span.end() - span.start();
                        span.rotation()
                            .point_speed_bound(v_src)
                            .map_err(|_| PhysicsError::InvalidMotion)?
                            .map_or_else(
                                || {
                                    span.rotation()
                                        .body_angular_displacement()
                                        .map_or(0., |ax| ax.cross(v_src).length())
                                },
                                |spd| spd * dt,
                            )
                    } else {
                        let v_src = rotate_vector(basis.conjugate(), c - *origin) / *scale;
                        span.point_speed_bound(v_src)
                            .map_err(|_| PhysicsError::InvalidMotion)?
                            * scale.abs()
                    };
                    bound = bound.max(travel);
                }
                Ok(bound)
            }
        }
    }

    fn smooth_travel(&self, u_left: f64, u_right: f64) -> Result<f64, PhysicsError> {
        let Some((idx, s, e)) = self.locate_smooth(u_left, u_right) else {
            return Ok(0.);
        };
        let span_travel = self.span_travel(idx)?;
        if matches!(self.motion, Some(AcceptedCurvedMotion::Arc(_))) {
            let clamped = u_right.min(self.cut_time_u);
            return Ok(span_travel * (clamped - u_left).max(0.));
        }
        Ok(span_travel * ((u_right - u_left) / (e - s)).clamp(0., 1.))
    }

    fn collect_cuts_and_steps(
        &self,
        is_moving: bool,
        cuts: &mut Vec<f64>,
        steps: &mut Vec<StepEvent>,
    ) {
        let Some(motion) = self.motion else {
            return;
        };
        if self.cut_time_u > 0. && self.cut_time_u < 1. {
            cuts.push(self.cut_time_u);
        }
        match motion {
            AcceptedCurvedMotion::Arc(_) => {}
            AcceptedCurvedMotion::RotationPath { path, .. } => {
                push_path_cuts(
                    path.duration(),
                    self.cut_span,
                    self.cut_frac,
                    path.spans()
                        .iter()
                        .map(|s| (s.start(), s.end(), s.is_step())),
                    is_moving,
                    cuts,
                    steps,
                );
            }
            AcceptedCurvedMotion::RigidPath { path, .. } => {
                push_path_cuts(
                    path.duration(),
                    self.cut_span,
                    self.cut_frac,
                    path.spans()
                        .iter()
                        .map(|s| (s.start(), s.end(), s.is_step())),
                    is_moving,
                    cuts,
                    steps,
                );
            }
        }
    }

    fn end_pose(&self) -> Result<AffineBox, PhysicsError> {
        let Some(motion) = self.motion else {
            return Ok(self.initial);
        };
        match motion {
            AcceptedCurvedMotion::Arc(_) => self.pose_at_span(0, self.cut_frac),
            AcceptedCurvedMotion::RotationPath { path, .. } if path.spans().is_empty() => {
                Ok(self.initial)
            }
            AcceptedCurvedMotion::RigidPath { path, .. } if path.spans().is_empty() => {
                Ok(self.initial)
            }
            _ if self.cut_span == 0 && self.cut_frac == 0. => Ok(self.initial),
            _ => self.pose_at_span(self.cut_span, self.cut_frac),
        }
    }

    fn pose_before_step(
        &self,
        time_u: f64,
        last_step: Option<usize>,
    ) -> Result<AffineBox, PhysicsError> {
        if let Some(step_idx) = last_step {
            let limit = if step_idx == self.cut_span {
                self.cut_frac
            } else {
                1.0
            };
            return self.pose_at_span(step_idx, limit);
        }
        let Some(motion) = self.motion else {
            return Ok(self.initial);
        };
        if matches!(motion, AcceptedCurvedMotion::Arc(_)) {
            return self.pose_at_span(0, time_u);
        }
        let (dur, spans): (f64, Vec<(f64, f64, bool)>) = match motion {
            AcceptedCurvedMotion::RotationPath { path, .. } => (
                path.duration(),
                path.spans()
                    .iter()
                    .map(|s| (s.start(), s.end(), s.is_step()))
                    .collect(),
            ),
            AcceptedCurvedMotion::RigidPath { path, .. } => (
                path.duration(),
                path.spans()
                    .iter()
                    .map(|s| (s.start(), s.end(), s.is_step()))
                    .collect(),
            ),
            AcceptedCurvedMotion::Arc(_) => return Ok(self.initial),
        };
        if dur <= 0. || time_u <= 0. {
            return Ok(self.initial);
        }
        let t = time_u * dur;
        let mut chosen = None;
        for (i, (s, e, is_step)) in spans.into_iter().enumerate() {
            if i > self.cut_span || (i == self.cut_span && self.cut_frac == 0.) {
                break;
            }
            if is_step {
                if s < t {
                    let frac = if i == self.cut_span {
                        self.cut_frac
                    } else {
                        1.0
                    };
                    chosen = Some((i, frac));
                }
            } else if s <= t {
                let raw = ((t - s) / (e - s)).clamp(0., 1.);
                let cap = if i == self.cut_span {
                    self.cut_frac
                } else {
                    1.0
                };
                chosen = Some((i, raw.min(cap)));
            }
        }
        match chosen {
            Some((idx, frac)) => self.pose_at_span(idx, frac),
            None => Ok(self.initial),
        }
    }
}

fn resolve_cutoff(len: usize, completed: usize, span_fraction: f64) -> (usize, f64) {
    if len == 0 {
        (0, 0.)
    } else if completed >= len {
        (len - 1, 1.)
    } else {
        (completed, span_fraction)
    }
}

fn locate_in_spans(
    duration: f64,
    cut_span: usize,
    cut_frac: f64,
    cut_time_u: f64,
    spans: impl Iterator<Item = (f64, f64)>,
    u_left: f64,
    u_right: f64,
) -> Option<(usize, f64, f64)> {
    if duration <= 0. {
        return None;
    }
    for (idx, (start, end)) in spans.enumerate() {
        if end <= start {
            continue;
        }
        let s = start / duration;
        let e = end / duration;
        if u_left >= s && u_right <= e {
            if idx > cut_span || (idx == cut_span && (u_left >= cut_time_u || cut_frac == 0.)) {
                return None;
            }
            return Some((idx, s, e));
        }
    }
    None
}

fn push_path_cuts(
    duration: f64,
    cut_span: usize,
    cut_frac: f64,
    spans: impl Iterator<Item = (f64, f64, bool)>,
    is_moving: bool,
    cuts: &mut Vec<f64>,
    steps: &mut Vec<StepEvent>,
) {
    for (idx, (start, end, is_step)) in spans.enumerate() {
        if is_step {
            if idx < cut_span || (idx == cut_span && cut_frac > 0.) {
                let u = if duration > 0. {
                    (start / duration).clamp(0., 1.)
                } else {
                    0.
                };
                let limit = if idx == cut_span { cut_frac } else { 1. };
                steps.push(StepEvent {
                    time_u: u,
                    is_moving,
                    span_idx: idx,
                    limit,
                });
            }
            continue;
        }
        if duration > 0. {
            let s = (start / duration).clamp(0., 1.);
            let e = (end / duration).clamp(0., 1.);
            cuts.push(s);
            cuts.push(e);
        }
    }
}

fn offset_box(
    initial: AffineBox,
    start_center: DVec3,
    start_orientation: DQuat,
    body_disp: DVec3,
    body_rot: DQuat,
) -> AffineBox {
    let world_orient = (start_orientation * body_rot).normalize();
    let delta_world = (world_orient * start_orientation.conjugate()).normalize();
    AffineBox {
        center: start_center
            + body_disp
            + rotate_vector(delta_world, initial.center - start_center),
        edges: initial.edges.map(|e| rotate_vector(delta_world, e)),
    }
}

fn box_corners(center: DVec3, edges: [DVec3; 3]) -> [DVec3; 8] {
    std::array::from_fn(|i| {
        center
            + edges[0] * if i & 1 == 0 { -1. } else { 1. }
            + edges[1] * if i & 2 == 0 { -1. } else { 1. }
            + edges[2] * if i & 4 == 0 { -1. } else { 1. }
    })
}

fn sat_gap(target: &AffineBox, moving: &AffineBox) -> f64 {
    let offset = moving.center - target.center;
    target
        .axes_for(moving.edges)
        .map(|axis| offset.dot(axis).abs() - target.radius(axis) - moving.radius(axis))
        .fold(f64::NEG_INFINITY, f64::max)
}

fn bisect_sat_interval(
    pair: (NodeId, NodeId),
    total_travel: f64,
    slack: f64,
    mut sample_gap: impl FnMut(f64) -> Result<f64, PhysicsError>,
    touching: &mut bool,
    budget: &mut SupportQueryBudget,
    crossings: &mut Vec<TriggerEvent>,
) -> Result<(), PhysicsError> {
    let mut transition = |next: bool| {
        if next != *touching {
            crossings.push(TriggerEvent {
                trigger: pair.0,
                collider: pair.1,
                phase: if next {
                    TriggerPhase::Enter
                } else {
                    TriggerPhase::Exit
                },
            });
            *touching = next;
        }
    };
    budget.charge_work(2)?;
    let start_gap = sample_gap(0.)?;
    let end_gap = sample_gap(1.)?;
    if start_gap.abs() > slack {
        transition(start_gap < 0.);
    }
    if total_travel <= slack {
        if end_gap.abs() > slack {
            transition(end_gap < 0.);
        }
        return Ok(());
    }
    let mut spans = vec![(0_f64, 1_f64)];
    while let Some((left, right)) = spans.pop() {
        budget.charge_work(1)?;
        let middle = (left + right) * 0.5;
        let gap = sample_gap(middle)?;
        let local_travel = total_travel * (right - left) * 0.5;
        let resolved = gap.abs() > local_travel + slack;
        if resolved || local_travel <= slack || middle <= left || middle >= right {
            if gap.abs() > slack {
                transition(gap < 0.);
            }
            continue;
        }
        spans.push((middle, right));
        spans.push((left, middle));
    }
    if end_gap.abs() > slack {
        transition(end_gap < 0.);
    }
    Ok(())
}

struct StepSyncState<'a> {
    moving: &'a CurvedView<'a>,
    target: &'a CurvedView<'a>,
    last_moving_step: Option<usize>,
    last_target_step: Option<usize>,
    touching: bool,
}

/// Evaluates synchronized curved motion + post-curved floor-snap for a `(trigger, collider)` pair.
pub(crate) fn append_curved_pair_transits(
    pair: (NodeId, NodeId),
    moving_in: CurvedMotionInput<'_>,
    target_in: CurvedMotionInput<'_>,
    previous: bool,
    budget: &mut SupportQueryBudget,
    crossings: &mut Vec<TriggerEvent>,
) -> Result<bool, PhysicsError> {
    let moving = CurvedView::new(moving_in.initial, moving_in.motion);
    let target = CurvedView::new(target_in.initial, target_in.motion);
    let mut cuts = vec![0.0_f64, 1.0_f64];
    let mut steps = Vec::new();
    moving.collect_cuts_and_steps(true, &mut cuts, &mut steps);
    target.collect_cuts_and_steps(false, &mut cuts, &mut steps);
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    steps.sort_by(|a, b| {
        a.time_u
            .total_cmp(&b.time_u)
            .then_with(|| (!a.is_moving).cmp(&(!b.is_moving)))
            .then_with(|| a.span_idx.cmp(&b.span_idx))
    });

    let mut sync = StepSyncState {
        moving: &moving,
        target: &target,
        last_moving_step: None,
        last_target_step: None,
        touching: previous,
    };
    let mut step_cursor = 0usize;

    while step_cursor < steps.len() && steps[step_cursor].time_u == 0.0 {
        flush_step_event(pair, &mut sync, steps[step_cursor], budget, crossings)?;
        step_cursor += 1;
    }

    for pair_u in cuts.windows(2) {
        let (u_a, u_b) = (pair_u[0], pair_u[1]);
        if u_b <= u_a {
            continue;
        }
        let travel = moving.smooth_travel(u_a, u_b)? + target.smooth_travel(u_a, u_b)?;
        let slack = 128. * f64::EPSILON * (1. + travel);
        bisect_sat_interval(
            pair,
            travel,
            slack,
            |frac| {
                let u = u_a + (u_b - u_a) * frac;
                let a = moving.pose_on_smooth(u_a, u_b, u)?;
                let b = target.pose_on_smooth(u_a, u_b, u)?;
                Ok(sat_gap(&b, &a))
            },
            &mut sync.touching,
            budget,
            crossings,
        )?;
        sync.last_moving_step = None;
        sync.last_target_step = None;
        while step_cursor < steps.len() && steps[step_cursor].time_u <= u_b {
            flush_step_event(pair, &mut sync, steps[step_cursor], budget, crossings)?;
            step_cursor += 1;
        }
    }

    let rel_snap = moving_in.post_snap - target_in.post_snap;
    if rel_snap != DVec3::ZERO {
        let end_moving = moving.end_pose()?;
        let end_target = target.end_pose()?;
        sync.touching = super::append_crossings(
            &end_target,
            end_moving,
            pair,
            &[rel_snap],
            sync.touching,
            budget,
            crossings,
        )?;
    }
    Ok(sync.touching)
}

fn flush_step_event(
    pair: (NodeId, NodeId),
    sync: &mut StepSyncState<'_>,
    ev: StepEvent,
    budget: &mut SupportQueryBudget,
    crossings: &mut Vec<TriggerEvent>,
) -> Result<(), PhysicsError> {
    if ev.is_moving {
        let fixed = sync
            .target
            .pose_before_step(ev.time_u, sync.last_target_step)?;
        let travel = sync.moving.span_travel(ev.span_idx)? * ev.limit;
        let slack = 128. * f64::EPSILON * (1. + travel);
        bisect_sat_interval(
            pair,
            travel,
            slack,
            |frac| {
                let a = sync.moving.pose_at_span(ev.span_idx, frac * ev.limit)?;
                Ok(sat_gap(&fixed, &a))
            },
            &mut sync.touching,
            budget,
            crossings,
        )?;
        sync.last_moving_step = Some(ev.span_idx);
    } else {
        let fixed_m = sync
            .moving
            .pose_before_step(ev.time_u, sync.last_moving_step)?;
        let travel = sync.target.span_travel(ev.span_idx)? * ev.limit;
        let slack = 128. * f64::EPSILON * (1. + travel);
        bisect_sat_interval(
            pair,
            travel,
            slack,
            |frac| {
                let b = sync.target.pose_at_span(ev.span_idx, frac * ev.limit)?;
                Ok(sat_gap(&b, &fixed_m))
            },
            &mut sync.touching,
            budget,
            crossings,
        )?;
        sync.last_target_step = Some(ev.span_idx);
    }
    Ok(())
}
