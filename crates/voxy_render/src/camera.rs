//! Right-handed cameras with WebGPU/DirectX depth range 0..1.
use glam::camera::rh::{proj::directx, view::look_at_mat4};
use glam::{Mat4, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SceneProjection {
    Perspective {
        vertical_fov: f32,
        aspect: f32,
        near: f32,
        far: f32,
    },
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneCamera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub projection: SceneProjection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidSceneCamera;
impl std::fmt::Display for InvalidSceneCamera {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid camera: expected a finite view and non-degenerate projection")
    }
}
impl std::error::Error for InvalidSceneCamera {}

impl SceneCamera {
    /// World-space origin and unit direction through a pixel center. Pixels use
    /// top-left image coordinates. Perspective origins are at the eye;
    /// orthographic origins are on the camera plane, before near clipping.
    /// This preserves participating medium between the eye and the near plane.
    /// # Errors
    /// Rejects invalid cameras, empty viewports and out-of-range pixels.
    pub fn pixel_ray(
        self,
        viewport: [u32; 2],
        pixel: [u32; 2],
    ) -> Result<(Vec3, Vec3), InvalidSceneCamera> {
        self.view_projection()?;
        if viewport.contains(&0) || (0..2).any(|i| pixel[i] >= viewport[i]) {
            return Err(InvalidSceneCamera);
        }
        let forward = (self.target - self.eye)
            .try_normalize()
            .ok_or(InvalidSceneCamera)?;
        let right = forward
            .cross(self.up)
            .try_normalize()
            .ok_or(InvalidSceneCamera)?;
        let up = right.cross(forward);
        // f64 avoids losing the center offset for large integer viewports.
        let u = (f64::from(pixel[0]) + 0.5) / f64::from(viewport[0]);
        let v = (f64::from(pixel[1]) + 0.5) / f64::from(viewport[1]);
        let (origin, direction) = match self.projection {
            SceneProjection::Perspective {
                vertical_fov,
                aspect,
                ..
            } => {
                let scale = f64::from(vertical_fov).mul_add(0.5, 0.).tan();
                let x = ((2. * u - 1.) * scale * f64::from(aspect)) as f32;
                let y = ((1. - 2. * v) * scale) as f32;
                (self.eye, forward + right * x + up * y)
            }
            SceneProjection::Orthographic {
                left,
                right: r,
                bottom,
                top,
                ..
            } => {
                let x = (f64::from(left) + u * (f64::from(r) - f64::from(left))) as f32;
                let y = (f64::from(top) + v * (f64::from(bottom) - f64::from(top))) as f32;
                (self.eye + right * x + up * y, forward)
            }
        };
        if !origin.is_finite() {
            return Err(InvalidSceneCamera);
        }
        Ok((origin, direction.try_normalize().ok_or(InvalidSceneCamera)?))
    }

    /// Reflect eye, target and up across a world-space plane for a capture pass.
    /// The plane normal must be finite and normalizable; unit length is not required.
    /// Projection is preserved.
    /// Use the returned view-projection to project the reflective surface into capture UVs;
    /// it remains right-handed rather than applying an implicit screen-axis flip.
    /// This does not clip geometry behind the reflector or allocate render targets.
    /// # Errors
    /// Rejects invalid source cameras, planes, or nonfinite reflected cameras.
    pub fn reflected(
        self,
        plane_point: Vec3,
        plane_normal: Vec3,
    ) -> Result<Self, InvalidSceneCamera> {
        self.view_projection()?;
        if !plane_point.is_finite() || !plane_normal.is_finite() {
            return Err(InvalidSceneCamera);
        }
        let normal = plane_normal.try_normalize().ok_or(InvalidSceneCamera)?;
        let reflect_point = |point: Vec3| point - 2.0 * (point - plane_point).dot(normal) * normal;
        let reflected = Self {
            eye: reflect_point(self.eye),
            target: reflect_point(self.target),
            up: self.up - 2.0 * self.up.dot(normal) * normal,
            projection: self.projection,
        };
        reflected.view_projection()?;
        Ok(reflected)
    }
    /// Conservative screen-error scale for an axis-aligned view-space domain.
    /// Bounds must enclose source and approximate geometry in this camera's view.
    /// Handles viewport/projection aspect mismatch by using the larger pixel scale.
    /// None requests base geometry for perspective near-plane crossing.
    /// # Errors
    /// Rejects invalid cameras, viewport dimensions and unordered/nonfinite bounds.
    pub fn lod_pixel_scale(
        self,
        view_min: Vec3,
        view_max: Vec3,
        viewport: [u32; 2],
    ) -> Result<Option<f64>, crate::LodError> {
        self.view_projection()
            .map_err(|_| crate::LodError::InvalidProjection)?;
        if !view_min.is_finite()
            || !view_max.is_finite()
            || view_min.cmpgt(view_max).any()
            || viewport.contains(&0)
        {
            return Err(crate::LodError::InvalidProjection);
        }
        let width = f64::from(viewport[0]);
        let height = f64::from(viewport[1]);
        match self.projection {
            SceneProjection::Perspective {
                vertical_fov,
                aspect,
                near,
                ..
            } => {
                let radial = f64::from(view_min.x.abs().max(view_max.x.abs()))
                    .hypot(f64::from(view_min.y.abs().max(view_max.y.abs())));
                crate::LodPolicy::perspective_pixel_scale(
                    height.max(width / f64::from(aspect)),
                    f64::from(vertical_fov),
                    -f64::from(view_max.z),
                    radial,
                    f64::from(near),
                )
            }
            SceneProjection::Orthographic {
                left,
                right,
                bottom,
                top,
                ..
            } => {
                let horizontal = crate::LodPolicy::orthographic_pixel_scale(
                    width,
                    f64::from(right) - f64::from(left),
                )?;
                let vertical = crate::LodPolicy::orthographic_pixel_scale(
                    height,
                    f64::from(top) - f64::from(bottom),
                )?;
                Ok(Some(horizontal.max(vertical)))
            }
        }
    }

    /// # Errors
    /// Rejects non-finite or degenerate views and invalid frusta.
    pub fn view_projection(self) -> Result<Mat4, InvalidSceneCamera> {
        let direction = self.target - self.eye;
        if !self.eye.is_finite()
            || !self.target.is_finite()
            || !self.up.is_finite()
            || direction.length_squared() <= f32::EPSILON
            || direction.cross(self.up).length_squared() <= f32::EPSILON
        {
            return Err(InvalidSceneCamera);
        }
        let projection = match self.projection {
            SceneProjection::Perspective {
                vertical_fov,
                aspect,
                near,
                far,
            } => {
                if ![vertical_fov, aspect, near, far]
                    .into_iter()
                    .all(f32::is_finite)
                    || !(0.0..std::f32::consts::PI).contains(&vertical_fov)
                    || vertical_fov == 0.0
                    || aspect <= 0.0
                    || near <= 0.0
                    || far <= near
                {
                    return Err(InvalidSceneCamera);
                }
                directx::perspective(vertical_fov, aspect, near, far)
            }
            SceneProjection::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => {
                if ![left, right, bottom, top, near, far]
                    .into_iter()
                    .all(f32::is_finite)
                    || right <= left
                    || top <= bottom
                    || near < 0.0
                    || far <= near
                {
                    return Err(InvalidSceneCamera);
                }
                directx::orthographic(left, right, bottom, top, near, far)
            }
        };
        let result = projection * look_at_mat4(self.eye, self.target, self.up);
        if result.is_finite() {
            Ok(result)
        } else {
            Err(InvalidSceneCamera)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera(projection: SceneProjection) -> SceneCamera {
        SceneCamera {
            eye: Vec3::ZERO,
            target: Vec3::NEG_Z,
            up: Vec3::Y,
            projection,
        }
    }

    #[test]
    fn reflection_preserves_plane_distances_and_projection_parity() {
        let source = SceneCamera {
            eye: Vec3::new(1.0, 2.0, 4.0),
            target: Vec3::new(0.0, 0.5, 0.0),
            up: Vec3::Y,
            projection: SceneProjection::Perspective {
                vertical_fov: 1.0,
                aspect: 1.5,
                near: 0.1,
                far: 100.0,
            },
        };
        for (point, normal) in [
            (Vec3::ZERO, Vec3::Y),
            (
                Vec3::new(0.5, -1.0, 2.0),
                Vec3::new(1.0, 2.0, 3.0).normalize(),
            ),
        ] {
            let mirrored = source.reflected(point, normal * 3.0).unwrap();
            assert_eq!(mirrored.projection, source.projection);
            for (original, result) in [(source.eye, mirrored.eye), (source.target, mirrored.target)]
            {
                assert!(
                    ((original - point).dot(normal) + (result - point).dot(normal)).abs() < 1e-5
                );
                let tangent = (original - result).cross(normal);
                assert!(tangent.length() < 1e-5);
            }
            let twice = mirrored.reflected(point, normal).unwrap();
            assert!(twice.eye.distance(source.eye) < 1e-5);
            assert!(twice.target.distance(source.target) < 1e-5);
            assert!(twice.up.distance(source.up) < 1e-5);
            let probe = Vec3::new(0.3, 0.4, -1.0);
            let reflected_probe = probe - 2.0 * (probe - point).dot(normal) * normal;
            let original_clip = source.view_projection().unwrap().project_point3(probe);
            let capture_clip = mirrored
                .view_projection()
                .unwrap()
                .project_point3(reflected_probe);
            assert!(
                (capture_clip - Vec3::new(-original_clip.x, original_clip.y, original_clip.z))
                    .length()
                    < 1e-5
            );
            let reversed = source.reflected(point, -normal).unwrap();
            assert!(reversed.eye.distance(mirrored.eye) < 1e-5);
        }
    }

    #[test]
    fn reflected_orthographic_camera_and_invalid_planes() {
        let source = camera(SceneProjection::Orthographic {
            left: -2.0,
            right: 2.0,
            bottom: -1.0,
            top: 1.0,
            near: 0.0,
            far: 10.0,
        });
        let reflected = source.reflected(Vec3::ZERO, Vec3::Z).unwrap();
        assert!(reflected.target.distance(Vec3::Z) < 1e-6);
        assert_eq!(reflected.projection, source.projection);
        for normal in [
            Vec3::ZERO,
            Vec3::splat(f32::NAN),
            Vec3::splat(f32::INFINITY),
        ] {
            assert!(source.reflected(Vec3::ZERO, normal).is_err());
        }
        assert!(source.reflected(Vec3::splat(f32::NAN), Vec3::Y).is_err());
    }

    #[test]
    fn perspective_depth_and_foreshortening() {
        let vp = camera(SceneProjection::Perspective {
            vertical_fov: std::f32::consts::FRAC_PI_2,
            aspect: 1.0,
            near: 1.0,
            far: 10.0,
        })
        .view_projection()
        .unwrap();
        assert!(vp.project_point3(Vec3::new(0.0, 0.0, -1.0)).z.abs() < 1e-5);
        assert!((vp.project_point3(Vec3::new(0.0, 0.0, -10.0)).z - 1.0).abs() < 1e-5);
        let close = vp.project_point3(Vec3::new(1.0, 0.0, -2.0));
        let far = vp.project_point3(Vec3::new(1.0, 0.0, -4.0));
        assert!((close.x - far.x * 2.0).abs() < 1e-5);
    }

    #[test]
    fn orthographic_scale_is_independent_of_distance() {
        let vp = camera(SceneProjection::Orthographic {
            left: -2.0,
            right: 2.0,
            bottom: -2.0,
            top: 2.0,
            near: 0.0,
            far: 10.0,
        })
        .view_projection()
        .unwrap();
        assert!(
            (vp.project_point3(Vec3::new(1.0, 0.0, -2.0)).x
                - vp.project_point3(Vec3::new(1.0, 0.0, -8.0)).x)
                .abs()
                < 1e-5
        );
    }

    #[test]
    fn rejects_invalid_frusta_and_parallel_up() {
        let projection = SceneProjection::Perspective {
            vertical_fov: 0.0,
            aspect: 1.0,
            near: 1.0,
            far: 10.0,
        };
        assert!(camera(projection).view_projection().is_err());
        let mut view = camera(SceneProjection::Orthographic {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: 0.0,
            far: 10.0,
        });
        view.up = Vec3::NEG_Z;
        assert!(view.view_projection().is_err());
    }
    #[test]
    fn lod_camera_scale_accounts_for_aspect_and_near_plane() {
        let view = camera(SceneProjection::Perspective {
            vertical_fov: std::f32::consts::FRAC_PI_2,
            aspect: 1.0,
            near: 0.1,
            far: 100.0,
        });
        let point = Vec3::new(0.0, 0.0, -10.0);
        let square = view
            .lod_pixel_scale(point, point, [1000, 1000])
            .unwrap()
            .unwrap();
        let wide = view
            .lod_pixel_scale(point, point, [2000, 1000])
            .unwrap()
            .unwrap();
        assert!((square - 50.0).abs() < 0.000_01);
        assert!((wide - square * 2.0).abs() < 0.000_01);
        let near = Vec3::new(0.0, 0.0, -0.1);
        assert!(
            view.lod_pixel_scale(near, near, [1000, 1000])
                .unwrap()
                .is_none()
        );
        assert!(view.lod_pixel_scale(point, point, [0, 1000]).is_err());
        assert!(
            view.lod_pixel_scale(Vec3::ONE, Vec3::ZERO, [1000, 1000])
                .is_err()
        );
    }
    #[test]
    fn pixel_rays_reproject_to_centers_for_both_lenses() {
        let projections = [
            SceneProjection::Perspective {
                vertical_fov: 0.8,
                aspect: 1.7,
                near: 0.3,
                far: 50.,
            },
            SceneProjection::Orthographic {
                left: -2.,
                right: 4.,
                bottom: -3.,
                top: 1.,
                near: 0.,
                far: 50.,
            },
            SceneProjection::Orthographic {
                left: -2.,
                right: 4.,
                bottom: -3.,
                top: 1.,
                near: 0.3,
                far: 50.,
            },
        ];
        for projection in projections {
            let view = SceneCamera {
                eye: Vec3::new(3., 2., 4.),
                target: Vec3::new(1., -1., 0.),
                up: Vec3::new(0.3, 1., 0.2),
                projection,
            };
            let matrix = view.view_projection().unwrap();
            let forward = (view.target - view.eye).normalize();
            for viewport in [[1, 1], [17, 9], [8, 14]] {
                for pixel in [
                    [0, 0],
                    [viewport[0] / 2, viewport[1] / 2],
                    [viewport[0] - 1, viewport[1] - 1],
                ] {
                    let (origin, direction) = view.pixel_ray(viewport, pixel).unwrap();
                    assert!((direction.length() - 1.).abs() < 1e-6);
                    match projection {
                        SceneProjection::Perspective { .. } => assert_eq!(origin, view.eye),
                        _ => {
                            assert!((origin - view.eye).dot(forward).abs() < 1e-6);
                            assert!((direction - forward).length() < 1e-6);
                        }
                    }
                    for distance in [1., 5., 20.] {
                        let clip = matrix * (origin + direction * distance).extend(1.);
                        let ndc = clip.truncate() / clip.w;
                        let expected_x = 2. * (pixel[0] as f32 + 0.5) / viewport[0] as f32 - 1.;
                        let expected_y = 1. - 2. * (pixel[1] as f32 + 0.5) / viewport[1] as f32;
                        assert!((ndc.x - expected_x).abs() < 3e-6);
                        assert!((ndc.y - expected_y).abs() < 3e-6);
                    }
                }
            }
            assert!(view.pixel_ray([0, 9], [0, 0]).is_err());
            assert!(view.pixel_ray([8, 9], [8, 0]).is_err());
        }
    }
}
