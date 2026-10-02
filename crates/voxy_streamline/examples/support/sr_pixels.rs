pub(super) fn validate_output(
    bytes: &[u8],
    row_bytes: u32,
    width: u32,
    height: u32,
    hdr: bool,
) -> Result<(), String> {
    let pixel_bytes = if hdr { 8_u32 } else { 4 };
    if width == 0
        || height == 0
        || row_bytes
            < width
                .checked_mul(pixel_bytes)
                .ok_or("readback width overflow")?
    {
        return Err("invalid DLSS readback dimensions/stride".into());
    }
    let required = usize::try_from(u64::from(row_bytes) * u64::from(height))
        .map_err(|_| "DLSS readback size overflow")?;
    if bytes.len() < required {
        return Err("truncated DLSS output readback".into());
    }
    // Ignore alpha: DLSS does not promise preservation unless configured for it.
    // Interior points avoid border reconstruction differences on the reset frame.
    for (x, y) in [
        (width / 2, height / 2),
        (width / 4, height / 4),
        (width * 3 / 4, height / 4),
        (width / 4, height * 3 / 4),
        (width * 3 / 4, height * 3 / 4),
    ] {
        let pixel_bytes = if hdr { 8 } else { 4 };
        let offset =
            usize::try_from(u64::from(y) * u64::from(row_bytes) + u64::from(x) * pixel_bytes)
                .map_err(|_| "DLSS readback offset overflow")?;
        let pixel = bytes
            .get(offset..offset + usize::try_from(pixel_bytes).map_err(|_| "invalid pixel size")?)
            .ok_or("truncated DLSS output readback")?;
        if hdr {
            for (channel, expected) in [4.0_f32, 1.0, 0.5].into_iter().enumerate() {
                let start = channel * 2;
                let value = half::f16::from_le_bytes([pixel[start], pixel[start + 1]]).to_f32();
                if !value.is_finite() || (value - expected).abs() > expected * 0.08 + 0.02 {
                    return Err(format!(
                        "DLSS HDR mismatch at ({x},{y}), channel {channel}: {value}"
                    ));
                }
            }
        } else if pixel[..3]
            .iter()
            .zip([51_u8, 102, 153])
            .any(|(actual, expected)| actual.abs_diff(expected) > 16)
        {
            return Err(format!("DLSS uniform RGB mismatch at ({x},{y}): {pixel:?}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_output;
    const POINTS: [(usize, usize); 5] = [(4, 4), (2, 2), (6, 2), (2, 6), (6, 6)];
    fn image(hdr: bool) -> Vec<u8> {
        let mut bytes = vec![0xcc; 256 * 8];
        for y in 0..8 {
            for x in 0..8 {
                if hdr {
                    for (channel, value) in [4.0_f32, 1.0, 0.5, f32::NAN].into_iter().enumerate() {
                        let offset = y * 256 + x * 8 + channel * 2;
                        bytes[offset..offset + 2]
                            .copy_from_slice(&half::f16::from_f32(value).to_le_bytes());
                    }
                } else {
                    let offset = y * 256 + x * 4;
                    bytes[offset..offset + 4].copy_from_slice(&[51, 102, 153, 0]);
                }
            }
        }
        bytes
    }
    #[test]
    fn padded_rows_and_unconstrained_alpha_pass() {
        for hdr in [false, true] {
            assert!(validate_output(&image(hdr), 256, 8, 8, hdr).is_ok());
        }
    }
    #[test]
    fn every_sample_and_rgb_channel_is_checked() {
        for (x, y) in POINTS {
            for channel in 0..3 {
                let mut bytes = image(false);
                bytes[y * 256 + x * 4 + channel] = 0;
                assert!(validate_output(&bytes, 256, 8, 8, false).is_err());
            }
        }
    }
    #[test]
    fn hdr_rejects_nonfinite_clipped_and_negative_rgb() {
        for (x, y) in POINTS {
            for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.0, -4.0] {
                let mut bytes = image(true);
                let offset = y * 256 + x * 8;
                bytes[offset..offset + 2]
                    .copy_from_slice(&half::f16::from_f32(value).to_le_bytes());
                assert!(validate_output(&bytes, 256, 8, 8, true).is_err());
            }
        }
    }
    #[test]
    fn empty_output_and_invalid_layouts_fail() {
        assert!(validate_output(&vec![0; 2048], 256, 8, 8, false).is_err());
        assert!(validate_output(&image(false)[..2047], 256, 8, 8, false).is_err());
        assert!(validate_output(&image(false), 31, 8, 8, false).is_err());
        assert!(validate_output(&[], 0, 0, 0, false).is_err());
    }
}
