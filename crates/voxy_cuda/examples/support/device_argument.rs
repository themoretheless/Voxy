//! Strict device selection for physical CUDA acceptance probes.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<usize, String> {
    let mut args = args.into_iter();
    let ordinal = match args.next() {
        None => 0,
        Some(value) => {
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("expected nonnegative CUDA device ordinal".into());
            }
            value
                .parse::<usize>()
                .map_err(|_| "CUDA device ordinal exceeds usize".to_owned())?
        }
    };
    if args.next().is_some() {
        return Err("expected at most one CUDA device ordinal".into());
    }
    Ok(ordinal)
}

#[cfg(test)]
mod tests {
    use super::parse;
    #[test]
    fn device_selection_is_explicit_and_rejects_ignored_arguments() {
        assert_eq!(parse([]).unwrap(), 0);
        assert_eq!(parse(["3".into()]).unwrap(), 3);
        for value in ["", "-1", "+1", "1.5", "cuda", " 1", "1 "] {
            assert!(parse([value.into()]).is_err(), "{value:?}");
        }
        assert!(parse(["0".into(), "1".into()]).is_err());
        assert!(parse(["0".into(), "--cuda-device".into()]).is_err());
        assert!(parse(["9".repeat(100)]).is_err());
    }
}
