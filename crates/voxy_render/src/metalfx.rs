//! Narrow native bridge. wgpu owns resources, submission and command-buffer lifetime.
#![allow(unsafe_code)]
use crate::ComputeError;
use objc2::{
    rc::{Retained, autoreleasepool},
    runtime::ProtocolObject,
};
use objc2_metal::MTLPixelFormat;
use objc2_metal_fx::{MTLFXTemporalScaler, MTLFXTemporalScalerBase, MTLFXTemporalScalerDescriptor};

#[derive(Debug)]
pub(crate) struct MetalFxTemporal {
    scaler: Retained<ProtocolObject<dyn MTLFXTemporalScaler>>,
}
impl MetalFxTemporal {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self, ComputeError> {
        // SAFETY: borrow the matching backend; never destroy or submit native handles.
        // All descriptor formats/sizes match our retained, single-sample textures.
        autoreleasepool(|_| unsafe {
            let native = device
                .as_hal::<wgpu_hal::api::Metal>()
                .ok_or(ComputeError::Unsupported)?;
            if !MTLFXTemporalScalerDescriptor::supportsDevice(native.raw_device()) {
                return Err(ComputeError::Unsupported);
            }
            let descriptor = MTLFXTemporalScalerDescriptor::new();
            descriptor.setInputWidth(width as usize);
            descriptor.setInputHeight(height as usize);
            descriptor.setOutputWidth(width as usize);
            descriptor.setOutputHeight(height as usize);
            descriptor.setColorTextureFormat(MTLPixelFormat::RGBA16Float);
            descriptor.setDepthTextureFormat(MTLPixelFormat::Depth32Float);
            descriptor.setMotionTextureFormat(MTLPixelFormat::RG16Float);
            descriptor.setOutputTextureFormat(MTLPixelFormat::RGBA16Float);
            descriptor.setAutoExposureEnabled(false);
            let scaler = descriptor
                .newTemporalScalerWithDevice(native.raw_device())
                .ok_or(ComputeError::Unsupported)?;
            scaler.setInputContentWidth(width as usize);
            scaler.setInputContentHeight(height as usize);
            // Motion is previousUV-currentUV in top-left normalized coordinates.
            scaler.setMotionVectorScaleX(width as f32);
            scaler.setMotionVectorScaleY(height as f32);
            scaler.setDepthReversed(false);
            scaler.setPreExposure(1.);
            Ok(Self { scaler })
        })
    }
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::Texture,
        depth: &wgpu::Texture,
        motion: &wgpu::Texture,
        output: &wgpu::Texture,
        jitter: glam::Vec2,
        reset: bool,
    ) {
        // SAFETY: all four textures were registered in the preceding wgpu command
        // buffer; native and display buffers are submitted together in order.
        // This encoder records native commands only. All textures remain retained
        // by the renderer. The scaler records only; wgpu finishes/submits. No wgpu
        // API is called while the HAL encoder is borrowed. No active pass exists.
        autoreleasepool(|_| unsafe {
            // Each as_hal guard takes the same device snatch lock. Retain one
            // native handle and release its guard before borrowing the next.
            let retain_texture = |texture: &wgpu::Texture| {
                let native = texture
                    .as_hal::<wgpu_hal::api::Metal>()
                    .expect("validated Metal texture");
                Retained::retain(std::ptr::from_ref(native.raw_handle()).cast_mut())
                    .expect("live Metal texture")
            };
            let color = retain_texture(color);
            let depth = retain_texture(depth);
            let motion = retain_texture(motion);
            let output = retain_texture(output);
            self.scaler.setColorTexture(Some(&color));
            self.scaler.setDepthTexture(Some(&depth));
            self.scaler.setMotionTexture(Some(&motion));
            self.scaler.setOutputTexture(Some(&output));
            self.scaler.setJitterOffsetX(jitter.x);
            self.scaler.setJitterOffsetY(jitter.y);
            self.scaler.setReset(reset);
            encoder.as_hal_mut::<wgpu_hal::api::Metal, _, _>(|native| {
                self.scaler.encodeToCommandBuffer(
                    native
                        .expect("validated Metal encoder")
                        .raw_command_buffer()
                        .expect("open command buffer"),
                );
            });
        });
    }
}
