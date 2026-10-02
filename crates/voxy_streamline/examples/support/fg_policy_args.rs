use voxy_streamline::FrameGeneration as FrameGenerationMode;

pub fn parse(value: &str) -> Result<FrameGenerationMode, String> {
    if value == "off" {
        return Ok(FrameGenerationMode::Off);
    }
    if value == "dynamic" {
        return Ok(FrameGenerationMode::Dynamic { target_fps: None });
    }
    if let Some(count) = value.strip_prefix("fixed:") {
        let generated_frames = count.parse::<u32>().map_err(|_| "invalid FG count")?;
        if generated_frames == 0 {
            return Err("FG count must be positive; use off to disable generation".into());
        }
        return Ok(FrameGenerationMode::Fixed { generated_frames });
    }
    if let Some(target) = value.strip_prefix("dynamic:") {
        let fps = target
            .parse::<f32>()
            .map_err(|_| "invalid dynamic FG target")?;
        if !fps.is_finite() || fps <= 0.0 {
            return Err("dynamic FG target must be finite and positive".into());
        }
        return Ok(FrameGenerationMode::Dynamic {
            target_fps: Some(fps),
        });
    }
    Err("FG policy must be off, fixed:N, dynamic or dynamic:FPS".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policies_preserve_user_counts_and_targets() {
        assert_eq!(parse("off"), Ok(FrameGenerationMode::Off));
        assert_eq!(
            parse("fixed:3"),
            Ok(FrameGenerationMode::Fixed {
                generated_frames: 3
            })
        );
        assert_eq!(
            parse("dynamic"),
            Ok(FrameGenerationMode::Dynamic { target_fps: None })
        );
        assert_eq!(
            parse("dynamic:144"),
            Ok(FrameGenerationMode::Dynamic {
                target_fps: Some(144.0)
            })
        );
        for value in [
            "",
            "fixed:0",
            "fixed:-1",
            "fixed:4294967296",
            "fixed:1.5",
            "dynamic:NaN",
            "dynamic:inf",
            "dynamic:0",
            "dynamic:-60",
            "auto",
        ] {
            assert!(parse(value).is_err(), "accepted invalid policy {value}");
        }
    }
}
