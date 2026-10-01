//! Selected-reaction REACLIB text reader; no temperature-domain inference.
use crate::astrophysics_nuclear::Reaclib;
#[derive(Clone, Debug, PartialEq)]
pub struct Rate {
    pub reactants: Vec<String>,
    pub products: Vec<String>,
    pub labels: Vec<String>,
    pub q_mev: f64,
    pub reverse: bool,
    pub fit: Reaclib,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidFormat,
    UnsupportedChapter,
    WeakReaction,
    MixedReaction,
    BudgetExceeded,
    InvalidDomain,
    InvalidNetwork,
    UnknownSpecies,
    BindingMismatch,
}
fn number(field: &str) -> Result<f64, Error> {
    let value: f64 = field
        .trim()
        .replace(['D', 'd'], "e")
        .parse()
        .map_err(|_| Error::InvalidFormat)?;
    if !value.is_finite() {
        return Err(Error::InvalidFormat);
    }
    Ok(value)
}
fn coefficients(line: &str, count: usize) -> Result<Vec<f64>, Error> {
    if line.len() < 13 * count || !line[13 * count..].trim().is_empty() {
        return Err(Error::InvalidFormat);
    }
    (0..count)
        .map(|i| number(&line[i * 13..(i + 1) * 13]))
        .collect()
}
/// Parse one selected reaction, possibly with several additive coefficient sets.
/// Accepts repeated chapter markers and blank separator lines. The fitted
/// temperature interval in kelvin must be supplied from source documentation.
/// No weak rates, mixed reactions or unsupported reaction orders are accepted.
/// # Errors
/// Malformed fields, unsupported chapter, weak/mixed data, domain or set budget.
pub fn parse(
    text: &str,
    min_temperature: f64,
    max_temperature: f64,
    max_sets: usize,
) -> Result<Rate, Error> {
    if !min_temperature.is_finite()
        || !max_temperature.is_finite()
        || min_temperature <= 0.0
        || max_temperature < min_temperature
    {
        return Err(Error::InvalidDomain);
    }
    if !text.is_ascii() {
        return Err(Error::InvalidFormat);
    }
    let mut lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .peekable();
    let chapter: u8 = lines
        .next()
        .ok_or(Error::InvalidFormat)?
        .trim()
        .parse()
        .map_err(|_| Error::InvalidFormat)?;
    let (inputs, outputs) = reaction_order(chapter)?;
    let mut result: Option<Rate> = None;
    while let Some(mut header) = lines.next() {
        if let Ok(marker) = header.trim().parse::<u8>() {
            if marker != chapter {
                return Err(Error::MixedReaction);
            }
            header = lines.next().ok_or(Error::InvalidFormat)?;
        }
        if result.as_ref().map_or(0, |r| r.fit.sets.len()) >= max_sets {
            return Err(Error::BudgetExceeded);
        }
        if header.len() < 64
            || !header[..5].trim().is_empty()
            || !header[35..43].trim().is_empty()
            || !header[49..52].trim().is_empty()
            || !header[64..].trim().is_empty()
        {
            return Err(Error::InvalidFormat);
        }
        let names: Vec<_> = (0..6)
            .map(|i| header[5 + i * 5..10 + i * 5].trim())
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect();
        // Historical chapter 8 also permits two products.
        let outputs = if chapter == 8 && names.len() == 5 {
            2
        } else {
            outputs
        };
        if names.len() != inputs + outputs {
            return Err(Error::InvalidFormat);
        }
        let flags = &header[43..49];
        if flags.as_bytes()[4] == b'w' {
            return Err(Error::WeakReaction);
        }
        let reverse = flags.as_bytes()[5] == b'v';
        let q = number(&header[52..64])?;
        let mut values = coefficients(lines.next().ok_or(Error::InvalidFormat)?, 4)?;
        values.extend(coefficients(lines.next().ok_or(Error::InvalidFormat)?, 3)?);
        let set: [f64; 7] = values.try_into().map_err(|_| Error::InvalidFormat)?;
        let reactants = names[..inputs].to_vec();
        let products = names[inputs..].to_vec();
        if let Some(rate) = &mut result {
            if rate.reactants != reactants
                || rate.products != products
                || rate.reverse != reverse
                || (rate.q_mev - q).abs() > 1e-10 * q.abs().max(1.0)
                || rate.labels[0][..4] != flags[..4]
            {
                return Err(Error::MixedReaction);
            }
            rate.labels.push(flags.to_owned());
            rate.fit.sets.push(set);
        } else {
            result = Some(Rate {
                reactants,
                products,
                labels: vec![flags.to_owned()],
                q_mev: q,
                reverse,
                fit: Reaclib {
                    sets: vec![set],
                    min_temperature,
                    max_temperature,
                },
            });
        }
    }
    result.ok_or(Error::InvalidFormat)
}

fn reaction_order(chapter: u8) -> Result<(usize, usize), Error> {
    Ok(match chapter {
        1 => (1, 1),
        2 => (1, 2),
        3 => (1, 3),
        4 => (2, 1),
        5 => (2, 2),
        6 => (2, 3),
        8 => (3, 1),
        9 => (3, 2),
        _ => return Err(Error::UnsupportedChapter),
    })
}

/// Exact SI conversion of `MeV` to joules.
pub const MEV_JOULES: f64 = 1.602_176_634e-13;
impl Rate {
    /// Map REACLIB names to the supplied network species order, and verify Q
    /// against the network binding reservoir with an explicit absolute `MeV`
    /// tolerance. Does not mutate the network. Reverse records keep their
    /// supplied direction; no automatic detailed-balance correction is applied.
    /// # Errors
    /// Invalid/duplicate names, unknown species, invalid stoichiometry or Q.
    pub fn reaction(
        &self,
        network: &crate::astrophysics_nuclear::Network,
        names: &[&str],
        neutrino_fraction: f64,
        q_tolerance_mev: f64,
    ) -> Result<crate::astrophysics_nuclear::Reaction, Error> {
        if self
            .labels
            .iter()
            .any(|label| label.as_bytes().get(4) == Some(&b'w'))
        {
            return Err(Error::WeakReaction);
        }
        if names.len() != network.nuclei.len()
            || names.is_empty()
            || names
                .iter()
                .enumerate()
                .any(|(i, name)| name.is_empty() || names[..i].contains(name))
            || !q_tolerance_mev.is_finite()
            || q_tolerance_mev < 0.0
            || !self.q_mev.is_finite()
        {
            return Err(Error::InvalidNetwork);
        }
        let counts = |species: &[String]| -> Result<Vec<u8>, Error> {
            let mut counts = vec![0_u8; names.len()];
            for name in species {
                let i = names
                    .iter()
                    .position(|n| *n == name)
                    .ok_or(Error::UnknownSpecies)?;
                counts[i] = counts[i].checked_add(1).ok_or(Error::InvalidNetwork)?;
            }
            Ok(counts)
        };
        let reaction = crate::astrophysics_nuclear::Reaction {
            reactants: counts(&self.reactants)?,
            products: counts(&self.products)?,
            rate: self.fit.clone(),
            neutrino_fraction,
        };
        let mut candidate = network.clone();
        candidate.reactions.push(reaction.clone());
        let mut fractions = vec![0.0; names.len()];
        fractions[0] = 1.0;
        candidate
            .reservoir(&fractions)
            .map_err(|_| Error::InvalidNetwork)?;
        let q = network
            .nuclei
            .iter()
            .zip(&reaction.reactants)
            .zip(&reaction.products)
            .map(|((n, a), b)| (f64::from(*b) - f64::from(*a)) * n.binding_energy)
            .sum::<f64>()
            / MEV_JOULES;
        if !q.is_finite() || (q - self.q_mev).abs() > q_tolerance_mev {
            return Err(Error::BindingMismatch);
        }
        Ok(reaction)
    }
}
