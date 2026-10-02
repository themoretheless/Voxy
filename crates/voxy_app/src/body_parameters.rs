//! Continuous model-space morphology. Dimensions are ratios except height and mass.
//! Mass changes an illustrative thickness estimate, not a calibrated body composition.
use voxy_render::SceneVertex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BodyModel {
    #[default]
    Female,
    Male,
}
impl BodyModel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Female => "female",
            Self::Male => "male",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NippleType {
    #[default]
    Reference,
    Projecting,
    Flat,
    Inverted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NavelShape {
    #[default]
    Reference,
    Round,
    Vertical,
    Horizontal,
}

/// Time-dependent response to a normalized stimulus. Time constants are supplied
/// by the caller; this model does not map temperature to physiological response.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColdResponse {
    response: f64,
    target: f64,
    onset_seconds: f64,
    recovery_seconds: f64,
}
impl ColdResponse {
    pub fn new(
        response: f64,
        target: f64,
        onset_seconds: f64,
        recovery_seconds: f64,
    ) -> Result<Self, &'static str> {
        if !response.is_finite()
            || !(0. ..=1.).contains(&response)
            || !target.is_finite()
            || !(0. ..=1.).contains(&target)
            || !onset_seconds.is_finite()
            || onset_seconds <= 0.
            || !recovery_seconds.is_finite()
            || recovery_seconds <= 0.
        {
            return Err("cold response/target must be 0..1 and time constants finite and positive");
        }
        Ok(Self {
            response,
            target,
            onset_seconds,
            recovery_seconds,
        })
    }
    pub fn response(&self) -> f64 {
        self.response
    }
    pub fn set_target(&mut self, target: f64) -> Result<(), &'static str> {
        if !target.is_finite() || !(0. ..=1.).contains(&target) {
            return Err("cold target must be 0..1");
        }
        self.target = target;
        Ok(())
    }
    /// Exact first-order integration for a constant target, including zero dt.
    /// Invalid steps leave state unchanged; finite large steps converge safely.
    pub fn advance(&mut self, seconds: f64) -> Result<f64, &'static str> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("cold response time step must be finite and nonnegative");
        }
        let tau = if self.target >= self.response {
            self.onset_seconds
        } else {
            self.recovery_seconds
        };
        let fraction = -(-seconds / tau).exp_m1();
        self.response += (self.target - self.response) * fraction;
        Ok(self.response)
    }
    pub fn apply(&self, parameters: BodyParameters) -> BodyParameters {
        BodyParameters {
            nipple_cold_response: self.response as f32,
            ..parameters
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyParameters {
    pub body_model: BodyModel,
    pub height_cm: f32,
    pub weight_kg: f32,
    pub breast_size: f32,
    pub buttock_size: f32,
    pub left_breast_size: f32,
    pub right_breast_size: f32,
    pub left_buttock_size: f32,
    pub right_buttock_size: f32,

    pub leg_length: f32,
    pub arm_length: f32,
    pub torso_length: f32,
    pub shoulder_width: f32,
    pub hip_width: f32,
    pub waist_width: f32,
    pub head_size: f32,
    pub hand_size: f32,
    pub foot_size: f32,
    pub thigh_length: f32,
    pub shin_length: f32,
    pub upper_arm_length: f32,
    pub forearm_length: f32,
    pub thigh_width: f32,
    pub calf_width: f32,
    pub upper_arm_width: f32,
    pub forearm_width: f32,
    pub torso_depth: f32,
    pub neck_length: f32,
    pub nipple_radius_mm: f32,
    pub nipple_projection_mm: f32,
    pub left_nipple_radius_scale: f32,
    pub right_nipple_radius_scale: f32,
    pub left_nipple_projection_scale: f32,
    pub right_nipple_projection_scale: f32,
    pub nipple_type: NippleType,
    /// Normalized visual cold stimulus; no temperature calibration.
    pub nipple_cold_response: f32,
    pub areola_radius_mm: f32,
    pub left_areola_size: f32,
    pub right_areola_size: f32,
    pub areola_pigmentation: f32,
    pub navel_width_mm: f32,
    pub navel_height_mm: f32,
    /// Positive is recessed; negative is protruding.
    pub navel_depth_mm: f32,
    pub navel_shape: NavelShape,
}
impl Default for BodyParameters {
    fn default() -> Self {
        Self {
            body_model: BodyModel::Female,
            height_cm: 164.,
            weight_kg: 60.,
            breast_size: 1.,
            buttock_size: 1.,
            left_breast_size: 1.,
            right_breast_size: 1.,
            left_buttock_size: 1.,
            right_buttock_size: 1.,

            leg_length: 1.,
            arm_length: 1.,
            torso_length: 1.,
            shoulder_width: 1.,
            hip_width: 1.,
            waist_width: 1.,
            head_size: 1.,
            hand_size: 1.,
            foot_size: 1.,
            thigh_length: 1.,
            shin_length: 1.,
            upper_arm_length: 1.,
            forearm_length: 1.,
            thigh_width: 1.,
            calf_width: 1.,
            upper_arm_width: 1.,
            forearm_width: 1.,
            torso_depth: 1.,
            neck_length: 1.,
            nipple_radius_mm: 8.,
            nipple_projection_mm: 3.,
            left_nipple_radius_scale: 1.,
            right_nipple_radius_scale: 1.,
            left_nipple_projection_scale: 1.,
            right_nipple_projection_scale: 1.,
            nipple_type: NippleType::Reference,
            nipple_cold_response: 0.,
            areola_radius_mm: 18.5,
            left_areola_size: 1.,
            right_areola_size: 1.,
            areola_pigmentation: 1.,
            navel_width_mm: 16.,
            navel_height_mm: 20.,
            navel_depth_mm: 4.,
            navel_shape: NavelShape::Reference,
        }
    }
}
impl BodyParameters {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.height_cm.is_finite()
            || !(130. ..=210.).contains(&self.height_cm)
            || !self.weight_kg.is_finite()
            || !(35. ..=180.).contains(&self.weight_kg)
        {
            return Err("height_cm must be 130..210 and weight_kg 35..180");
        }
        for v in [
            self.breast_size,
            self.buttock_size,
            self.left_breast_size,
            self.right_breast_size,
            self.left_buttock_size,
            self.right_buttock_size,
            self.leg_length,
            self.arm_length,
            self.torso_length,
            self.shoulder_width,
            self.hip_width,
            self.waist_width,
            self.head_size,
            self.hand_size,
            self.foot_size,
            self.thigh_length,
            self.shin_length,
            self.upper_arm_length,
            self.forearm_length,
            self.thigh_width,
            self.calf_width,
            self.upper_arm_width,
            self.forearm_width,
            self.torso_depth,
            self.neck_length,
        ] {
            if !v.is_finite() || !(0.6..=1.6).contains(&v) {
                return Err("body ratios must be finite and within 0.6..1.6");
            }
        }
        if [
            self.breast_size * self.left_breast_size,
            self.breast_size * self.right_breast_size,
            self.buttock_size * self.left_buttock_size,
            self.buttock_size * self.right_buttock_size,
        ]
        .iter()
        .any(|v| !(0.6..=1.6).contains(v))
        {
            return Err("combined regional size must be within 0.6..1.6");
        }
        for (name, value, min, max) in [
            (
                "left_nipple_radius_scale",
                self.left_nipple_radius_scale,
                0.6,
                1.6,
            ),
            (
                "right_nipple_radius_scale",
                self.right_nipple_radius_scale,
                0.6,
                1.6,
            ),
            (
                "left_nipple_projection_scale",
                self.left_nipple_projection_scale,
                0.6,
                1.6,
            ),
            (
                "right_nipple_projection_scale",
                self.right_nipple_projection_scale,
                0.6,
                1.6,
            ),
            ("left_areola_size", self.left_areola_size, 0.6, 1.6),
            ("right_areola_size", self.right_areola_size, 0.6, 1.6),
            ("areola_radius_mm", self.areola_radius_mm, 8., 40.),
            ("areola_pigmentation", self.areola_pigmentation, 0., 1.),
            ("nipple_cold_response", self.nipple_cold_response, 0., 1.),
            ("nipple_radius_mm", self.nipple_radius_mm, 3., 25.),
            ("nipple_projection_mm", self.nipple_projection_mm, 0., 12.),
            ("navel_width_mm", self.navel_width_mm, 8., 40.),
            ("navel_height_mm", self.navel_height_mm, 8., 40.),
            ("navel_depth_mm", self.navel_depth_mm, -8., 12.),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err(match name {
                    "left_nipple_radius_scale"
                    | "right_nipple_radius_scale"
                    | "left_nipple_projection_scale"
                    | "right_nipple_projection_scale" => "nipple side scale must be 0.6..1.6",
                    "left_areola_size" | "right_areola_size" => "areola side size must be 0.6..1.6",
                    "areola_radius_mm" => "areola radius must be 8..40 mm",
                    "areola_pigmentation" => "areola pigmentation must be 0..1",
                    "nipple_cold_response" => "cold response must be 0..1",
                    "nipple_radius_mm" => "nipple radius must be 3..25 mm",
                    "nipple_projection_mm" => "nipple projection must be 0..12 mm",
                    "navel_depth_mm" => "navel depth must be -8..12 mm",
                    _ => "navel width and height must be 8..40 mm",
                });
            }
        }
        if [self.left_areola_size, self.right_areola_size]
            .iter()
            .any(|side| !(8. ..=40.).contains(&(self.areola_radius_mm * side)))
        {
            return Err("combined areola radius must be 8..40 mm");
        }
        if [
            self.left_nipple_radius_scale,
            self.right_nipple_radius_scale,
        ]
        .iter()
        .any(|side| !(3. ..=25.).contains(&(self.nipple_radius_mm * side)))
        {
            return Err("combined nipple radius must be 3..25 mm before cold contraction");
        }
        if [
            self.left_nipple_projection_scale,
            self.right_nipple_projection_scale,
        ]
        .iter()
        .any(|side| !(0. ..=12.).contains(&(self.nipple_projection_mm * side)))
        {
            return Err("combined nipple projection must be 0..12 mm before cold response");
        }
        Ok(())
    }
    /// Reads a partial JSON object. Omitted fields retain the reference proportions.
    pub fn from_json(text: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let value: serde_json::Value = serde_json::from_str(text)?;
        let object = value.as_object().ok_or("expected body parameter object")?;
        let mut p = Self::default();
        for (key, value) in object {
            if key == "body_model" {
                p.body_model = match value.as_str() {
                    Some("female") => BodyModel::Female,
                    Some("male") => BodyModel::Male,
                    _ => return Err("invalid body_model".into()),
                };
                continue;
            }
            if key == "nipple_type" {
                p.nipple_type = match value.as_str() {
                    Some("reference") => NippleType::Reference,
                    Some("projecting") => NippleType::Projecting,
                    Some("flat") => NippleType::Flat,
                    Some("inverted") => NippleType::Inverted,
                    _ => return Err("invalid nipple_type".into()),
                };
                continue;
            }
            if key == "navel_shape" {
                p.navel_shape = match value.as_str() {
                    Some("reference") => NavelShape::Reference,
                    Some("round") => NavelShape::Round,
                    Some("vertical") => NavelShape::Vertical,
                    Some("horizontal") => NavelShape::Horizontal,
                    _ => return Err("invalid navel_shape".into()),
                };
                continue;
            }
            let v = value.as_f64().ok_or("body parameter must be a number")? as f32;
            let target = match key.as_str() {
                "height_cm" => &mut p.height_cm,
                "weight_kg" => &mut p.weight_kg,
                "breast_size" => &mut p.breast_size,
                "buttock_size" => &mut p.buttock_size,
                "left_breast_size" => &mut p.left_breast_size,
                "right_breast_size" => &mut p.right_breast_size,
                "left_buttock_size" => &mut p.left_buttock_size,
                "right_buttock_size" => &mut p.right_buttock_size,

                "leg_length" => &mut p.leg_length,
                "arm_length" => &mut p.arm_length,
                "torso_length" => &mut p.torso_length,
                "shoulder_width" => &mut p.shoulder_width,
                "hip_width" => &mut p.hip_width,
                "waist_width" => &mut p.waist_width,
                "head_size" => &mut p.head_size,
                "hand_size" => &mut p.hand_size,
                "foot_size" => &mut p.foot_size,
                "thigh_length" => &mut p.thigh_length,
                "shin_length" => &mut p.shin_length,
                "upper_arm_length" => &mut p.upper_arm_length,
                "forearm_length" => &mut p.forearm_length,
                "thigh_width" => &mut p.thigh_width,
                "calf_width" => &mut p.calf_width,
                "upper_arm_width" => &mut p.upper_arm_width,
                "forearm_width" => &mut p.forearm_width,
                "torso_depth" => &mut p.torso_depth,
                "neck_length" => &mut p.neck_length,
                "left_areola_size" => &mut p.left_areola_size,
                "right_areola_size" => &mut p.right_areola_size,
                "areola_radius_mm" => &mut p.areola_radius_mm,
                "areola_pigmentation" => &mut p.areola_pigmentation,
                "nipple_cold_response" => &mut p.nipple_cold_response,
                "nipple_radius_mm" => &mut p.nipple_radius_mm,
                "nipple_projection_mm" => &mut p.nipple_projection_mm,
                "left_nipple_radius_scale" => &mut p.left_nipple_radius_scale,
                "right_nipple_radius_scale" => &mut p.right_nipple_radius_scale,
                "left_nipple_projection_scale" => &mut p.left_nipple_projection_scale,
                "right_nipple_projection_scale" => &mut p.right_nipple_projection_scale,
                "navel_width_mm" => &mut p.navel_width_mm,
                "navel_height_mm" => &mut p.navel_height_mm,
                "navel_depth_mm" => &mut p.navel_depth_mm,

                _ => return Err(format!("unknown body parameter: {key}").into()),
            };
            *target = v;
        }
        p.validate()?;
        Ok(p)
    }
    /// Complete JSON representation used by presets and the MCP bridge.
    pub fn to_json(&self) -> serde_json::Value {
        let mut value = serde_json::json!({"body_model":self.body_model.as_str(),"height_cm":self.height_cm,"weight_kg":self.weight_kg,
            "breast_size":self.breast_size,"buttock_size":self.buttock_size,
"left_breast_size":self.left_breast_size,"right_breast_size":self.right_breast_size,"left_buttock_size":self.left_buttock_size,"right_buttock_size":self.right_buttock_size,
            "leg_length":self.leg_length,"arm_length":self.arm_length,"torso_length":self.torso_length,
            "shoulder_width":self.shoulder_width,"hip_width":self.hip_width,"waist_width":self.waist_width,
            "head_size":self.head_size,"hand_size":self.hand_size,"foot_size":self.foot_size,
"thigh_length":self.thigh_length,"shin_length":self.shin_length,"upper_arm_length":self.upper_arm_length,"forearm_length":self.forearm_length,"thigh_width":self.thigh_width,"calf_width":self.calf_width,"upper_arm_width":self.upper_arm_width,"forearm_width":self.forearm_width,"torso_depth":self.torso_depth,"neck_length":self.neck_length,
            "nipple_radius_mm":self.nipple_radius_mm,"nipple_projection_mm":self.nipple_projection_mm,
            "left_areola_size":self.left_areola_size,"right_areola_size":self.right_areola_size,
            "areola_radius_mm":self.areola_radius_mm,"areola_pigmentation":self.areola_pigmentation,
            "nipple_cold_response":self.nipple_cold_response,
            "nipple_type":match self.nipple_type { NippleType::Reference=>"reference",NippleType::Projecting=>"projecting",NippleType::Flat=>"flat",NippleType::Inverted=>"inverted" },
            "navel_width_mm":self.navel_width_mm,"navel_height_mm":self.navel_height_mm,"navel_depth_mm":self.navel_depth_mm,
            "navel_shape":match self.navel_shape {NavelShape::Reference=>"reference",NavelShape::Round=>"round",NavelShape::Vertical=>"vertical",NavelShape::Horizontal=>"horizontal"}});
        for (name, number) in [
            ("left_nipple_radius_scale", self.left_nipple_radius_scale),
            ("right_nipple_radius_scale", self.right_nipple_radius_scale),
            (
                "left_nipple_projection_scale",
                self.left_nipple_projection_scale,
            ),
            (
                "right_nipple_projection_scale",
                self.right_nipple_projection_scale,
            ),
        ] {
            value[name] = serde_json::json!(number);
        }
        value
    }
    pub(crate) fn areola_radius_at(&self, x: f32) -> f32 {
        let t = ((x + 0.04) / 0.08).clamp(0., 1.);
        let blend = t * t * (3. - 2. * t);
        self.areola_radius_mm
            * (self.left_areola_size * (1. - blend) + self.right_areola_size * blend)
    }
    /// Illustrative reference-space pigmentation, independent of projection and cold stimulus.
    #[cfg(test)]
    pub(crate) fn areola_tint(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let smooth = |a: f32, b: f32, value: f32| {
            let t = ((value - a) / (b - a)).clamp(0., 1.);
            t * t * (3. - 2. * t)
        };
        let radius = self.areola_radius_at(x) / 1000.;
        let distance = ((x.abs() - 0.08).powi(2) + (y - 0.36).powi(2)).sqrt();
        let coverage = (1. - smooth(radius - 0.0025, radius + 0.0025, distance))
            * smooth(0.08, 0.12, z)
            * self.areola_pigmentation;
        [
            1. - 0.28 * coverage,
            1. - 0.51 * coverage,
            1. - 0.54 * coverage,
        ]
    }
    /// Validates a partial update before committing any field.
    pub fn patched(&self, patch: &serde_json::Value) -> Result<Self, Box<dyn std::error::Error>> {
        let object = patch.as_object().ok_or("parameters must be an object")?;
        let mut merged = self.to_json();
        for (key, value) in object {
            merged
                .as_object_mut()
                .unwrap()
                .insert(key.clone(), value.clone());
        }
        Self::from_json(&merged.to_string())
    }
    pub(crate) fn transform(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        fn band(y: f32, center: f32, radius: f32) -> f32 {
            let t = ((y - center) / radius).abs().min(1.);
            (1. - t * t).powi(2)
        }
        let arm = ((x.abs() - 0.17) / 0.13).clamp(0., 1.);
        let thickness = ((self.weight_kg / 60.) / (self.height_cm / 164.).powi(3)).powf(0.25);
        let width = 1.
            + (self.shoulder_width - 1.) * band(y, 0.43, 0.19)
            + (self.hip_width - 1.) * band(y, -0.12, 0.22)
            + (self.waist_width - 1.) * band(y, 0.12, 0.20);
        let mut px = x * width * thickness;
        let leg_y = |y: f32| {
            let thigh = (y + 0.10).max(-0.36);
            let shin = (y + 0.46).min(0.);
            -0.10 + self.leg_length * (thigh * self.thigh_length + shin * self.shin_length)
        };
        let mut py = if y < -0.10 {
            leg_y(y)
        } else if y < 0.48 {
            -0.10 + (y + 0.10) * self.torso_length
        } else {
            -0.10
                + 0.58 * self.torso_length
                + (y - 0.48) * self.head_size
                + (self.neck_length - 1.) * 0.08 * ((y - 0.48) / 0.08).clamp(0., 1.)
        };
        let mut pz = z * thickness;
        let shoulder_y = -0.10 + 0.53 * self.torso_length;
        px += x.signum() * (x.abs() - 0.17).max(0.) * (self.arm_length - 1.) * arm;
        py += (py - shoulder_y) * (self.arm_length - 1.) * arm;
        // Segment translations accumulate below elbow; compact bands avoid hard seams.
        let upper = (0.43 - y).clamp(0., 0.23);
        let lower = (0.20 - y).clamp(0., 0.22);
        py -= arm
            * self.arm_length
            * (upper * (self.upper_arm_length - 1.) + lower * (self.forearm_length - 1.));
        px += x.signum()
            * arm
            * self.arm_length
            * (0.12 * (upper / 0.23) * (self.upper_arm_length - 1.)
                + 0.09 * (lower / 0.22) * (self.forearm_length - 1.));
        let leg_mask = (1. - arm) * ((-0.10 - y) / 0.12).clamp(0., 1.);
        let thigh = band(y, -0.29, 0.24) * leg_mask;
        let calf = band(y, -0.61, 0.18) * leg_mask;
        // Scale each leg about its own centre rather than changing leg separation.
        let leg_width = 1. + (self.thigh_width - 1.) * thigh + (self.calf_width - 1.) * calf;
        let leg_center = x.signum() * 0.09 * width * thickness;
        px = leg_center + (px - leg_center) * leg_width;
        pz *= leg_width;
        let arm_width = 1.
            + arm
                * ((self.upper_arm_width - 1.) * band(y, 0.32, 0.17)
                    + (self.forearm_width - 1.) * band(y, 0.09, 0.16));
        pz *= arm_width;
        // The rest-space arm centre follows the diagonal upper/lower arm axis.
        let arm_center =
            x.signum() * (0.17 + 0.12 * (upper / 0.23) + 0.09 * (lower / 0.22)) * width * thickness;
        px += (x * width * thickness - arm_center) * (arm_width - 1.);
        pz *= 1. + (self.torso_depth - 1.) * band(y, 0.15, 0.40) * (1. - arm);
        let t = ((x + 0.04) / 0.08).clamp(0., 1.);
        let blend = t * t * (3. - 2. * t);
        let breast_size = self.breast_size
            * (self.left_breast_size * (1. - blend) + self.right_breast_size * blend);
        let buttock_size = self.buttock_size
            * (self.left_buttock_size * (1. - blend) + self.right_buttock_size * blend);
        let chest = band(y, 0.36, 0.18) * (1. - arm) * (z / 0.10).clamp(0., 1.);
        let rear = band(y, -0.10, 0.20) * (1. - arm) * (-z / 0.08).clamp(0., 1.);
        pz += (breast_size - 1.) * 0.085 * chest - (buttock_size - 1.) * 0.075 * rear;
        px *= 1. + 0.16 * (breast_size - 1.) * chest + 0.18 * (buttock_size - 1.) * rear;
        let head = ((y - 0.48) / 0.08).clamp(0., 1.);
        px *= 1. + (self.head_size - 1.) * head;
        pz *= 1. + (self.head_size - 1.) * head;
        let hand = band(y, -0.12, 0.13) * arm;
        px += x.signum() * (x.abs() - 0.36).max(0.) * (self.hand_size - 1.) * hand;
        pz *= 1. + (self.hand_size - 1.) * hand;
        let foot = ((-y - 0.68) / 0.10).clamp(0., 1.) * (1. - arm);
        pz *= 1. + (self.foot_size - 1.) * foot;
        // Compact local relief morphs on the anterior reference surface.
        // Reference preserves the imported detail; other modes add an illustrative profile.
        let profile = |dx: f32, dy: f32, rx: f32, ry: f32| {
            let r2 = (dx / rx).powi(2) + (dy / ry).powi(2);
            (1. - r2).max(0.).powi(3)
        };
        let front = ((z - 0.08) / 0.04).clamp(0., 1.);
        if self.nipple_type != NippleType::Reference
            || self.nipple_radius_mm != 8.
            || self.nipple_projection_mm != 3.
            || self.left_nipple_radius_scale != 1.
            || self.right_nipple_radius_scale != 1.
            || self.left_nipple_projection_scale != 1.
            || self.right_nipple_projection_scale != 1.
            || self.nipple_cold_response > 0.
        {
            let cold = self.nipple_cold_response;
            let radius_scale = self.left_nipple_radius_scale * (1. - blend)
                + self.right_nipple_radius_scale * blend;
            let projection_scale = self.left_nipple_projection_scale * (1. - blend)
                + self.right_nipple_projection_scale * blend;
            let radius = self.nipple_radius_mm * radius_scale / 1000. * (1. - 0.15 * cold);
            let signed = match self.nipple_type {
                NippleType::Inverted => -1.,
                NippleType::Flat => 0.,
                _ => 1.,
            };
            // Illustrative erection response, not tissue swelling or calibrated physiology.
            let relief =
                (signed * self.nipple_projection_mm * projection_scale + 3. * cold) / 1000.;
            let local = profile(x.abs() - 0.08, y - 0.36, radius, radius);
            // Projecting relief is added to the imported surface. Subtracting an
            // assumed 3 mm source bump cancelled the default projecting morph;
            // that bump is not measured on either imported body mesh.
            // Flat/inverted/reference edits retain the existing approximation.
            let reference_relief = if self.nipple_type == NippleType::Projecting {
                0.
            } else {
                0.003 * profile(x.abs() - 0.08, y - 0.36, 0.008, 0.008)
            };
            pz += front * (relief * local - reference_relief);
        }
        if self.navel_shape != NavelShape::Reference
            || self.navel_width_mm != 16.
            || self.navel_height_mm != 20.
            || self.navel_depth_mm != 4.
        {
            let mut rx = self.navel_width_mm / 2000.;
            let mut ry = self.navel_height_mm / 2000.;
            match self.navel_shape {
                NavelShape::Round => {
                    let r = (rx * ry).sqrt();
                    rx = r;
                    ry = r;
                }
                NavelShape::Vertical => {
                    ry = ry.max(rx * 1.5);
                }
                NavelShape::Horizontal => {
                    rx = rx.max(ry * 1.5);
                }
                _ => {}
            }
            pz += front
                * (-self.navel_depth_mm / 1000. * profile(x, y - 0.10, rx, ry)
                    + 0.004 * profile(x, y - 0.10, 0.008, 0.010));
        }
        // Normalize stature after changing segment proportions; soles remain anchored.
        let bottom = leg_y(-0.820042849);
        let top = -0.10
            + 0.58 * self.torso_length
            + (0.819330931 - 0.48) * self.head_size
            + (self.neck_length - 1.) * 0.08;
        let scale = (self.height_cm / 100.) / (top - bottom);
        [px * scale, (py - bottom) * scale - 0.820042849, pz * scale]
    }
    /// Preflight the reference surface after this morph. Does not detect
    /// intersections; area checks cannot establish anatomical validity.
    pub fn surface_quality(
        &self,
        vertices: &[SceneVertex],
        indices: &[u32],
    ) -> Result<serde_json::Value, &'static str> {
        self.validate()?;
        if indices.is_empty() || indices.len() % 3 != 0 {
            return Err("invalid surface triangles");
        }
        let mut minimum_ratio = f64::INFINITY;
        let mut minimum_area = f64::INFINITY;
        let mut maximum_ratio = 0_f64;
        let mut rotated_faces = 0usize;
        for triangle in indices.chunks_exact(3) {
            let mut rest = [glam::DVec3::ZERO; 3];
            let mut posed = rest;
            for k in 0..3 {
                let vertex = vertices
                    .get(triangle[k] as usize)
                    .ok_or("surface index out of bounds")?;
                if vertex.position.iter().any(|v| !v.is_finite()) {
                    return Err("nonfinite surface vertex");
                }
                rest[k] = glam::DVec3::from_array(vertex.position.map(f64::from));
                let position = if *self == Self::default() {
                    vertex.position
                } else {
                    self.transform(vertex.position)
                };
                posed[k] = glam::DVec3::from_array(position.map(f64::from));
            }
            let before = (rest[1] - rest[0]).cross(rest[2] - rest[0]);
            let after = (posed[1] - posed[0]).cross(posed[2] - posed[0]);
            let reference_area = before.length() * 0.5;
            let area = after.length() * 0.5;
            if reference_area <= 0. || !area.is_finite() || area <= 0. {
                return Err("degenerate surface triangle");
            }
            let ratio = area / reference_area;
            if ratio < 1e-12 {
                return Err("collapsed surface triangle after morph");
            }
            minimum_ratio = minimum_ratio.min(ratio);
            maximum_ratio = maximum_ratio.max(ratio);
            minimum_area = minimum_area.min(area);
            if before.dot(after) < 0. {
                rotated_faces += 1;
            }
        }
        Ok(
            serde_json::json!({"triangles":indices.len()/3,"minimumAreaM2":minimum_area,
            "minimumAreaRatio":minimum_ratio,"maximumAreaRatio":maximum_ratio,
            "normalRotationOver90DegreesCount":rotated_faces,"intersectionCheckPerformed":false}),
        )
    }

    pub(crate) fn apply(&self, vertices: &mut [SceneVertex]) {
        if *self == Self::default() {
            return;
        }
        for vertex in vertices {
            vertex.position = self.transform(vertex.position);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn height_and_soles_follow_parameters() {
        let p = BodyParameters {
            height_cm: 190.,
            leg_length: 1.2,
            torso_length: 0.9,
            ..Default::default()
        };
        let bottom = p.transform([0., -0.820042849, 0.]);
        let top = p.transform([0., 0.819330931, 0.]);
        assert!((top[1] - bottom[1] - 1.90).abs() < 1e-5);
        assert!((bottom[1] + 0.820042849).abs() < 1e-5);
    }
    #[test]
    fn local_sizes_and_limb_lengths_change_geometry() {
        let base = BodyParameters::default();
        for (p, point, axis) in [
            (
                BodyParameters {
                    breast_size: 1.5,
                    ..base
                },
                [0.1, 0.36, 0.14],
                2,
            ),
            (
                BodyParameters {
                    buttock_size: 1.5,
                    ..base
                },
                [0.1, -0.10, -0.11],
                2,
            ),
            (
                BodyParameters {
                    arm_length: 1.4,
                    ..base
                },
                [0.4, -0.10, 0.],
                0,
            ),
            (
                BodyParameters {
                    weight_kg: 90.,
                    ..base
                },
                [0.1, 0.10, 0.08],
                0,
            ),
        ] {
            assert!((p.transform(point)[axis] - base.transform(point)[axis]).abs() > 0.01);
        }
    }
    #[test]
    fn json_rejects_bad_inputs() {
        assert!(BodyParameters::from_json("{\"height_cm\":180,\"breast_size\":1.3}").is_ok());
        for json in [
            "{\"weight_kg\":0}",
            "{\"arm_length\":8}",
            "{\"breast_szie\":1}",
            "[]",
        ] {
            assert!(BodyParameters::from_json(json).is_err());
        }
    }
}

#[cfg(test)]
mod segment_tests {
    use super::*;
    #[test]
    fn segment_proportions_preserve_stature_and_joint_continuity() {
        let p = BodyParameters {
            height_cm: 180.,
            thigh_length: 1.3,
            shin_length: 0.8,
            neck_length: 1.2,
            ..Default::default()
        };
        let bottom = p.transform([0., -0.820042849, 0.]);
        let top = p.transform([0., 0.819330931, 0.]);
        assert!((top[1] - bottom[1] - 1.8).abs() < 1e-5);
        let hip = p.transform([0.09, -0.10, 0.]);
        let knee = p.transform([0.09, -0.46, 0.]);
        let ankle = p.transform([0.09, -0.80, 0.]);
        assert!(
            ((hip[1] - knee[1]) / (knee[1] - ankle[1]) - (0.36 * 1.3) / (0.34 * 0.8)).abs() < 1e-4
        );
        for y in [-0.46, -0.10, 0.20, 0.43, 0.48, 0.56] {
            let a = p.transform([0.30, y - 1e-5, 0.02]);
            let b = p.transform([0.30, y + 1e-5, 0.02]);
            assert!((0..3).all(|i| (a[i] - b[i]).abs() < 1e-3));
        }
    }
    #[test]
    fn each_new_parameter_changes_its_region_and_roundtrips() {
        let base = BodyParameters::default();
        for (name, point) in [
            ("thigh_length", [0.09, -0.30, 0.03]),
            ("shin_length", [0.09, -0.60, 0.03]),
            ("upper_arm_length", [0.29, 0.20, 0.03]),
            ("forearm_length", [0.38, -0.02, 0.03]),
            ("thigh_width", [0.15, -0.29, 0.05]),
            ("calf_width", [0.13, -0.61, 0.04]),
            ("upper_arm_width", [0.27, 0.32, 0.04]),
            ("forearm_width", [0.35, 0.09, 0.03]),
            ("torso_depth", [0.05, 0.15, 0.08]),
            ("neck_length", [0.03, 0.58, 0.04]),
        ] {
            let patch = serde_json::json!({name:1.3});
            let p = base.patched(&patch).unwrap();
            assert_eq!(
                BodyParameters::from_json(&p.to_json().to_string()).unwrap(),
                p
            );
            let a = base.transform(point);
            let b = p.transform(point);
            assert!((0..3).any(|i| (a[i] - b[i]).abs() > 0.001), "{name}");
        }
        assert!(
            base.patched(&serde_json::json!({"shin_length":0.}))
                .is_err()
        );
    }
}

#[cfg(test)]
mod detail_tests {
    use super::*;
    #[test]
    fn nipple_sides_change_independently_and_combined_dimensions_are_bounded() {
        let base = BodyParameters::default()
            .patched(&serde_json::json!({"nipple_type":"projecting"}))
            .unwrap();
        for (field, side, offset) in [
            ("left_nipple_radius_scale", -1., 0.004),
            ("right_nipple_radius_scale", 1., 0.004),
            ("left_nipple_projection_scale", -1., 0.),
            ("right_nipple_projection_scale", 1., 0.),
        ] {
            let changed = base.patched(&serde_json::json!({field:1.5})).unwrap();
            let selected = [side * (0.08 + offset), 0.36, 0.14];
            let other = [-side * (0.08 + offset), 0.36, 0.14];
            assert!(
                (changed.transform(selected)[2] - base.transform(selected)[2]).abs() > 0.0001,
                "{field}"
            );
            assert_eq!(changed.transform(other), base.transform(other), "{field}");
            assert_eq!(
                BodyParameters::from_json(&changed.to_json().to_string()).unwrap(),
                changed
            );
            assert!(base.patched(&serde_json::json!({field:0.5})).is_err());
        }
        assert!(
            base.patched(
                &serde_json::json!({"nipple_radius_mm":25.,"left_nipple_radius_scale":1.6})
            )
            .is_err()
        );
        assert!(
            base.patched(
                &serde_json::json!({"nipple_radius_mm":3.,"right_nipple_radius_scale":0.6})
            )
            .is_err()
        );
        assert!(
            base.patched(
                &serde_json::json!({"nipple_projection_mm":12.,"right_nipple_projection_scale":1.6})
            )
            .is_err()
        );
    }
    #[test]
    fn default_projecting_relief_survives_on_both_sides_and_cold_adds_height() {
        let base = BodyParameters::default();
        let projecting = base
            .patched(&serde_json::json!({"nipple_type":"projecting"}))
            .unwrap();
        let cold = projecting
            .patched(&serde_json::json!({"nipple_cold_response":1.}))
            .unwrap();
        for x in [-0.08, 0.08] {
            let point = [x, 0.36, 0.14];
            let original = base.transform(point)[2];
            let warm = projecting.transform(point)[2];
            let chilled = cold.transform(point)[2];
            let scaled_relief = 0.003 * original / point[2];
            assert!((warm - original - scaled_relief).abs() < 1e-6);
            assert!((chilled - warm - scaled_relief).abs() < 1e-6);
        }
        for point in [[0., 0.36, 0.14], [0.08, 0.36, -0.14], [0.08, 0.5, 0.14]] {
            assert_eq!(projecting.transform(point), base.transform(point));
        }
    }
    #[test]
    fn detail_types_have_opposite_relief_and_leave_remote_surface_alone() {
        let base = BodyParameters::default();
        let projecting = base
            .patched(&serde_json::json!({"nipple_type":"projecting","nipple_projection_mm":8}))
            .unwrap();
        let inverted = base
            .patched(&serde_json::json!({"nipple_type":"inverted","nipple_projection_mm":8}))
            .unwrap();
        let flat = base
            .patched(&serde_json::json!({"nipple_type":"flat"}))
            .unwrap();
        let point = [0.08, 0.36, 0.158];
        assert!(projecting.transform(point)[2] > flat.transform(point)[2]);
        assert!(inverted.transform(point)[2] < flat.transform(point)[2]);
        assert_eq!(
            projecting.transform([0., 0.65, 0.13]),
            base.transform([0., 0.65, 0.13])
        );
        let recessed = base
            .patched(&serde_json::json!({"navel_shape":"vertical","navel_depth_mm":8}))
            .unwrap();
        let protruding = base
            .patched(&serde_json::json!({"navel_shape":"round","navel_depth_mm":-5}))
            .unwrap();
        assert!(
            recessed.transform([0., 0.10, 0.13])[2] < protruding.transform([0., 0.10, 0.13])[2]
        );
        for p in [projecting, inverted, flat, recessed, protruding] {
            assert_eq!(
                p,
                BodyParameters::from_json(&p.to_json().to_string()).unwrap()
            );
        }
        for patch in [
            serde_json::json!({"nipple_type":"bad"}),
            serde_json::json!({"navel_depth_mm":50}),
            serde_json::json!({"nipple_radius_mm":0}),
        ] {
            assert!(base.patched(&patch).is_err());
        }
    }
    #[test]
    fn imported_surface_contains_vertices_changed_by_detail_morphs() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let base = BodyParameters::default();
        for patch in [
            serde_json::json!({"nipple_type":"projecting","nipple_projection_mm":8,"nipple_radius_mm":15}),
            serde_json::json!({"navel_shape":"vertical","navel_depth_mm":8,"navel_width_mm":25,"navel_height_mm":30}),
        ] {
            let p = base.patched(&patch).unwrap();
            assert!(asset.mesh.vertices().iter().any(|v| {
                (p.transform(v.position)[2] - base.transform(v.position)[2]).abs() > 0.001
            }));
        }
    }
}

#[cfg(test)]
mod cold_response_tests {
    use super::*;
    #[test]
    fn integration_is_independent_of_time_partition_and_recovers() {
        let mut whole = ColdResponse::new(0., 1., 2., 8.).unwrap();
        let mut split = whole;
        whole.advance(3.).unwrap();
        for _ in 0..300 {
            split.advance(0.01).unwrap();
        }
        assert!((whole.response() - split.response()).abs() < 1e-13);
        assert!((whole.response() - (1. - (-1.5_f64).exp())).abs() < 1e-13);
        let previous = whole.response();
        whole.set_target(0.).unwrap();
        whole.advance(8.).unwrap();
        assert!((whole.response() - previous * (-1_f64).exp()).abs() < 1e-13);
        let body = BodyParameters::default();
        assert_eq!(
            whole.apply(body).nipple_cold_response,
            whole.response() as f32
        );
        assert_eq!(whole.apply(body).nipple_radius_mm, body.nipple_radius_mm);
    }
    #[test]
    fn invalid_input_is_atomic_and_extreme_steps_remain_bounded() {
        let mut state = ColdResponse::new(0.2, 0.8, 2., 8.).unwrap();
        for dt in [-1., f64::NAN, f64::INFINITY] {
            let previous = state;
            assert!(state.advance(dt).is_err());
            assert_eq!(state, previous);
        }
        let previous = state;
        for target in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
            assert!(state.set_target(target).is_err());
            assert_eq!(state, previous);
        }
        state.advance(0.).unwrap();
        assert_eq!(state, previous);
        state.advance(f64::MAX).unwrap();
        assert_eq!(state.response(), 0.8);
        assert!(ColdResponse::new(0., 1., 0., 8.).is_err());
        assert!(ColdResponse::new(0., 1., 2., f64::NAN).is_err());
    }
}

#[cfg(test)]
mod asymmetry_tests {
    #[test]
    fn unilateral_morph_changes_only_selected_side() {
        let base = super::BodyParameters::default();
        let p = base
            .patched(&serde_json::json!({"left_breast_size":1.3,"left_buttock_size":1.2}))
            .unwrap();
        for point in [[0.1, 0.36, 0.14], [0.1, -0.10, -0.11]] {
            assert_eq!(base.transform(point), p.transform(point));
        }
        for point in [[-0.1, 0.36, 0.14], [-0.1, -0.10, -0.11]] {
            assert!((base.transform(point)[2] - p.transform(point)[2]).abs() > 0.005);
        }
    }
    #[test]
    fn cold_response_is_local_and_validated() {
        let neutral = super::BodyParameters::default();
        let cold = neutral
            .patched(&serde_json::json!({"nipple_cold_response":1.}))
            .unwrap();
        assert!(cold.transform([0.08, 0.36, 0.14])[2] > neutral.transform([0.08, 0.36, 0.14])[2]);
        assert_eq!(
            cold.transform([0., 0., 0.14]),
            neutral.transform([0., 0., 0.14])
        );
        assert!(
            neutral
                .patched(&serde_json::json!({"nipple_cold_response":1.1}))
                .is_err()
        );
        assert_eq!(
            super::BodyParameters::from_json(&cold.to_json().to_string()).unwrap(),
            cold
        );
    }
}

#[cfg(test)]
mod areola_tests {
    use super::BodyParameters;
    #[test]
    fn pigment_radius_changes_colour_without_changing_geometry() {
        let base = BodyParameters::default();
        let small = base
            .patched(&serde_json::json!({"areola_radius_mm":8.,"areola_pigmentation":0.5}))
            .unwrap();
        let large = base
            .patched(&serde_json::json!({"areola_radius_mm":30.}))
            .unwrap();
        let point = [0.10, 0.36, 0.14];
        assert_eq!(small.areola_tint(point), [1.; 3]);
        assert!(large.areola_tint(point)[1] < 0.6);
        assert_eq!(small.transform(point), large.transform(point));
        assert_eq!(large.areola_tint([0.10, 0.36, -0.14]), [1.; 3]);
        assert_eq!(
            large.areola_tint([-0.10, 0.36, 0.14]),
            large.areola_tint(point)
        );
        assert!(
            base.patched(&serde_json::json!({"areola_radius_mm":41.}))
                .is_err()
        );
        assert!(
            base.patched(&serde_json::json!({"areola_pigmentation":-0.1}))
                .is_err()
        );
        assert_eq!(
            BodyParameters::from_json(&large.to_json().to_string()).unwrap(),
            large
        );
    }
}

#[cfg(test)]
mod areola_side_tests {
    use super::BodyParameters;
    #[test]
    fn sides_are_independent_and_combined_radius_is_bounded() {
        let base = BodyParameters::default();
        let changed = base
            .patched(&serde_json::json!({"left_areola_size":1.6}))
            .unwrap();
        assert_eq!(changed.areola_radius_at(0.08), base.areola_radius_at(0.08));
        assert!(changed.areola_radius_at(-0.08) > base.areola_radius_at(-0.08));
        assert_eq!(
            changed.areola_tint([0.10, 0.36, 0.14]),
            base.areola_tint([0.10, 0.36, 0.14])
        );
        assert_ne!(
            changed.areola_tint([-0.10, 0.36, 0.14]),
            base.areola_tint([-0.10, 0.36, 0.14])
        );
        assert!(
            base.patched(&serde_json::json!({"areola_radius_mm":40.,"left_areola_size":1.6}))
                .is_err()
        );
        assert_eq!(
            BodyParameters::from_json(&changed.to_json().to_string()).unwrap(),
            changed
        );
    }
}
