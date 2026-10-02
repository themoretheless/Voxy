//! Terrain selection is validated before window/driver initialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainBackend {
    Cpu,
    Gpu,
    Cuda { ordinal: usize },
}

impl TerrainBackend {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut gpu = false;
        let mut cuda = false;
        let mut cuda_projectiles = false;
        let mut ordinal = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--gpu-terrain" => gpu = true,
                "--cuda-terrain" => cuda = true,
                "--cuda-projectiles"
                | "--cuda-character-motion"
                | "--cuda-vehicle-motion"
                | "--cuda-collisions"
                | "--cuda-water" => cuda_projectiles = true,
                "--cuda-device" => {
                    if ordinal.is_some() {
                        return Err("--cuda-device must be specified once".into());
                    }
                    let value = args.next().ok_or("--cuda-device requires an ordinal")?;
                    let parsed = value
                        .parse::<usize>()
                        .map_err(|_| "invalid CUDA device ordinal")?;
                    if i32::try_from(parsed).is_err() {
                        return Err("CUDA device ordinal exceeds the driver range".into());
                    }
                    ordinal = Some(parsed);
                }
                _ => {}
            }
        }
        if gpu && cuda {
            return Err("choose one terrain backend: --gpu-terrain or --cuda-terrain".into());
        }
        if ordinal.is_some() && !cuda && !cuda_projectiles {
            return Err("--cuda-device requires a CUDA terrain/motion/collision flag".into());
        }
        Ok(if cuda {
            Self::Cuda {
                ordinal: ordinal.unwrap_or(0),
            }
        } else if gpu {
            Self::Gpu
        } else {
            Self::Cpu
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TerrainBackend;
    fn parse(args: &[&str]) -> Result<TerrainBackend, String> {
        TerrainBackend::parse(args.iter().map(ToString::to_string))
    }
    #[test]
    fn explicit_device_and_existing_flags() {
        assert_eq!(parse(&[]), Ok(TerrainBackend::Cpu));
        assert_eq!(parse(&["--gpu-terrain"]), Ok(TerrainBackend::Gpu));
        assert_eq!(
            parse(&["--cuda-terrain"]),
            Ok(TerrainBackend::Cuda { ordinal: 0 })
        );
        assert_eq!(
            parse(&["--autopilot", "--cuda-device", "2", "--cuda-terrain"]),
            Ok(TerrainBackend::Cuda { ordinal: 2 })
        );
    }
    #[test]
    fn cuda_projectiles_can_share_device_without_selecting_cuda_terrain() {
        let args = ["--gpu-terrain", "--cuda-projectiles", "--cuda-device", "2"];
        assert_eq!(parse(&args), Ok(TerrainBackend::Gpu));
        assert_eq!(
            super::motion_device(args.iter().map(ToString::to_string)),
            Ok(Some(2))
        );
        assert_eq!(
            super::motion_device(["--cuda-projectiles".to_owned()]),
            Ok(Some(0))
        );
    }
    #[test]
    fn character_motion_can_select_device_with_cpu_terrain() {
        let args = ["--cuda-character-motion", "--cuda-device", "3"];
        assert_eq!(parse(&args), Ok(TerrainBackend::Cpu));
        assert_eq!(
            super::motion_device(args.iter().map(ToString::to_string)),
            Ok(Some(3))
        );
    }
    #[test]
    fn vehicle_motion_uses_the_shared_device_independently_of_terrain() {
        let args = ["--cuda-vehicle-motion", "--cuda-device", "2"];
        assert_eq!(parse(&args), Ok(TerrainBackend::Cpu));
        assert_eq!(
            super::motion_device(args.iter().map(ToString::to_string)),
            Ok(Some(2))
        );
    }
    #[test]
    fn cuda_collision_selects_device_without_changing_terrain() {
        let args = ["--cuda-collisions", "--cuda-device", "2"];
        assert_eq!(parse(&args), Ok(TerrainBackend::Cpu));
        assert_eq!(
            super::motion_device(args.iter().map(ToString::to_string)),
            Ok(Some(2))
        );
    }
    #[test]
    fn cuda_water_uses_selected_device_with_cpu_terrain() {
        let args = ["--cuda-water", "--cuda-device", "3"];
        assert_eq!(parse(&args), Ok(TerrainBackend::Cpu));
        assert_eq!(
            super::motion_device(args.iter().map(ToString::to_string)),
            Ok(Some(3))
        );
        assert_eq!(
            super::motion_device(["--cuda-water".to_owned()]),
            Ok(Some(0))
        );
    }
    #[test]
    fn invalid_selection_is_rejected_before_driver_work() {
        for args in [
            vec!["--cuda-device"],
            vec!["--cuda-device", "-1", "--cuda-terrain"],
            vec!["--cuda-device", "2147483648", "--cuda-terrain"],
            vec!["--cuda-device", "0"],
            vec!["--gpu-terrain", "--cuda-terrain"],
            vec!["--cuda-terrain", "--cuda-device", "0", "--cuda-device", "1"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
    }
}

/// Shares the selected ordinal with terrain without requiring CUDA terrain.
pub fn motion_device(args: impl IntoIterator<Item = String>) -> Result<Option<usize>, String> {
    let args: Vec<_> = args.into_iter().collect();
    TerrainBackend::parse(args.clone())?;
    if !args.iter().any(|arg| {
        arg == "--cuda-projectiles"
            || arg == "--cuda-character-motion"
            || arg == "--cuda-vehicle-motion"
            || arg == "--cuda-collisions"
            || arg == "--cuda-water"
    }) {
        return Ok(None);
    }
    let ordinal = args
        .windows(2)
        .find(|pair| pair[0] == "--cuda-device")
        .map_or(Ok(0), |pair| {
            pair[1]
                .parse::<usize>()
                .map_err(|_| "invalid CUDA device ordinal".to_owned())
        })?;
    Ok(Some(ordinal))
}
