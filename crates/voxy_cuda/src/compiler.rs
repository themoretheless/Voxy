use crate::CudaError;

fn select_architecture(device: (i32, i32), supported: &[i32]) -> Result<i32, CudaError> {
    let (major, minor) = device;
    if major <= 0 || !(0..10).contains(&minor) {
        return Err(CudaError::UnsupportedCompilerArchitecture);
    }
    let limit = major
        .checked_mul(10)
        .and_then(|value| value.checked_add(minor))
        .ok_or(CudaError::UnsupportedCompilerArchitecture)?;
    supported
        .iter()
        .copied()
        .filter(|arch| *arch > 0 && *arch <= limit)
        .max()
        .ok_or(CudaError::UnsupportedCompilerArchitecture)
}

#[cfg(feature = "cuda")]
#[allow(unsafe_code)]
impl crate::CudaCompute {
    pub(crate) fn compiler_options(
        &self,
        source_name: &'static str,
    ) -> Result<cudarc::nvrtc::CompileOptions, CudaError> {
        // SAFETY: Only queries the loaded compiler; no device pointer is passed.
        if !unsafe { cudarc::nvrtc::sys::is_culib_present() } {
            return Err(CudaError::CompilerUnavailable);
        }
        let mut count = 0;
        // SAFETY: `count` is a live writable i32 for the duration of the query.
        unsafe { cudarc::nvrtc::sys::nvrtcGetNumSupportedArchs(&raw mut count) }
            .result()
            .map_err(CudaError::CompilerQuery)?;
        if !(1..=256).contains(&count) {
            return Err(CudaError::UnsupportedCompilerArchitecture);
        }
        let mut supported = vec![
            0;
            usize::try_from(count)
                .map_err(|_| CudaError::UnsupportedCompilerArchitecture)?
        ];
        // SAFETY: The compiler-reported count bounds this writable vector.
        unsafe { cudarc::nvrtc::sys::nvrtcGetSupportedArchs(supported.as_mut_ptr()) }
            .result()
            .map_err(CudaError::CompilerQuery)?;
        let architecture = select_architecture(
            self.context
                .compute_capability()
                .map_err(CudaError::Driver)?,
            &supported,
        )?;
        Ok(cudarc::nvrtc::CompileOptions {
            // Keep every runtime kernel aligned with the verified NVRTC matrix.
            // Integer workloads also inherit this policy for future arithmetic changes.
            name: Some(source_name.to_owned()),
            fmad: Some(false),
            use_fast_math: Some(false),
            ftz: Some(false),
            prec_div: Some(true),
            prec_sqrt: Some(true),
            options: vec![format!("--gpu-architecture=compute_{architecture}")],
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::select_architecture;
    #[test]
    fn compiler_target_never_exceeds_device_or_compiler_support() {
        let supported = [89, 52, 75, 86, 80];
        assert_eq!(select_architecture((8, 9), &supported).unwrap(), 89);
        assert_eq!(select_architecture((8, 7), &supported).unwrap(), 86);
        assert_eq!(select_architecture((12, 0), &supported).unwrap(), 89);
        assert_eq!(select_architecture((5, 2), &supported).unwrap(), 52);
        for device in [(3, 0), (0, 0), (8, 10), (i32::MAX, 0)] {
            assert!(select_architecture(device, &supported).is_err());
        }
        assert!(select_architecture((8, 9), &[]).is_err());
        assert!(select_architecture((8, 9), &[90, 0, -1]).is_err());
    }
}
