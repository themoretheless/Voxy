//! Static grey LTE absorption/emission along a ray. Intensity is bolometric
//! W m^-2 sr^-1; opacity is absorption per length (m^-1). No scattering.
use crate::{astrophysics::Error, astrophysics_thermal::STEFAN_BOLTZMANN};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    pub length: f64,
    pub absorption: f64,
    pub temperature: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Transfer {
    pub intensity: f64,
    /// Net intensity deposited in each layer (negative for net emission).
    /// Multiply by ray solid angle, projected area and duration to obtain J.
    pub deposited: Vec<f64>,
    /// Incident intensity minus the local start source, retained without losing
    /// a small diffusion flux to subtraction of nearly equal absolute intensities.
    pub incident_source_offsets: Vec<f64>,
    /// Outgoing intensity minus the final layer's end source.
    pub end_source_offset: f64,
}
/// Integrated Planck intensity in SI units.
/// # Errors
/// Invalid temperature or numerical overflow.
pub fn blackbody(temperature: f64) -> Result<f64, Error> {
    if !temperature.is_finite() || temperature < 0.0 {
        return Err(Error::InvalidInput);
    }
    let value = STEFAN_BOLTZMANN / std::f64::consts::PI * temperature.powi(4);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::NumericalOverflow)
    }
}
/// Exact constant-source formal solution, evaluated in ray traversal order.
/// # Errors
/// Invalid input/layers, exhausted layer budget or floating-point overflow.
/// Failure returns no partial transfer or external state mutations.
pub fn trace(incident: f64, layers: &[Layer], max_layers: usize) -> Result<Transfer, Error> {
    let sources = layers
        .iter()
        .map(|layer| {
            Ok(SourceLayer {
                length: layer.length,
                absorption: layer.absorption,
                source: blackbody(layer.temperature)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    trace_sources(incident, &sources, max_layers)
}
/// A constant source intensity in the same units as the incident intensity.
/// Can represent one frequency bin without a bolometric temperature conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceLayer {
    pub length: f64,
    pub absorption: f64,
    pub source: f64,
}
/// Constant-source formal solution for arbitrary nonnegative source intensities.
/// # Errors
/// Invalid layer/incident intensity, work budget or numerical overflow.
pub fn trace_sources(
    incident: f64,
    layers: &[SourceLayer],
    max_layers: usize,
) -> Result<Transfer, Error> {
    let linear = layers
        .iter()
        .map(|layer| LinearSourceLayer {
            length: layer.length,
            absorption: layer.absorption,
            source_start: layer.source,
            source_end: layer.source,
        })
        .collect::<Vec<_>>();
    trace_linear_sources(incident, &linear, max_layers)
}

/// Source intensity varying linearly along a ray segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearSourceLayer {
    pub length: f64,
    pub absorption: f64,
    pub source_start: f64,
    pub source_end: f64,
}
/// Exact formal solution for a source linear in distance (constant absorption).
/// Stable thin-limit series and positive source weights avoid cancellation.
/// # Errors
/// Invalid input/layers, work budget or overflow.
pub fn trace_linear_sources(
    incident: f64,
    layers: &[LinearSourceLayer],
    max_layers: usize,
) -> Result<Transfer, Error> {
    if !incident.is_finite() || incident < 0.0 || layers.len() > max_layers {
        return Err(Error::InvalidInput);
    }
    let mut result = Transfer {
        intensity: incident,
        deposited: Vec::with_capacity(layers.len()),
        incident_source_offsets: Vec::with_capacity(layers.len()),
        end_source_offset: 0.0,
    };
    let mut previous_source = incident;
    for layer in layers {
        if ![
            layer.length,
            layer.absorption,
            layer.source_start,
            layer.source_end,
        ]
        .into_iter()
        .all(|v| v.is_finite() && v >= 0.0)
        {
            return Err(Error::InvalidInput);
        }
        let depth = layer.length * layer.absorption;
        if !depth.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        let attenuation = (-depth).exp();
        let fraction = -(-depth).exp_m1();
        let end_weight = if depth < 1e-3 {
            depth
                * (0.5
                    + depth
                        * (-1.0 / 6.0
                            + depth * (1.0 / 24.0 + depth * (-1.0 / 120.0 + depth / 720.0))))
        } else {
            1.0 - fraction / depth
        };
        let start_weight = if depth < 1e-3 {
            fraction - end_weight
        } else {
            fraction / depth - attenuation
        };
        let incident_offset = result.end_source_offset + (previous_source - layer.source_start);
        let deposited =
            incident_offset * fraction - (layer.source_end - layer.source_start) * end_weight;
        let source_response = if depth == 0.0 { 1.0 } else { fraction / depth };
        let end_offset = incident_offset * attenuation
            - (layer.source_end - layer.source_start) * source_response;
        let outgoing = result.intensity * attenuation
            + layer.source_start * start_weight
            + layer.source_end * end_weight;
        if !outgoing.is_finite() || !deposited.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        result.intensity = outgoing;
        result.end_source_offset = end_offset;
        result.incident_source_offsets.push(incident_offset);
        previous_source = layer.source_end;
        result.deposited.push(deposited);
    }
    Ok(result)
}
