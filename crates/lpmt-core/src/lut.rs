//! Per-output 1D calibration LUTs.
//!
//! A LUT is stored in the project as three channel curves sampled at
//! [`LUT_SAMPLES`] points. Keeping the values in the project makes a show
//! self-contained: the original `.cube` file is only needed when loading it.
//! The renderer applies the curves in display space after colour correction.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Number of samples stored for each channel. It must be a multiple of four so
/// the GPU uniform layout stays simple.
pub const LUT_SAMPLES: usize = 32;

/// A display-space RGB calibration curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationLut {
    /// Human-readable source name, normally the `.cube` title.
    pub name: String,
    pub red: [f32; LUT_SAMPLES],
    pub green: [f32; LUT_SAMPLES],
    pub blue: [f32; LUT_SAMPLES],
}

#[derive(Debug, Error, PartialEq)]
pub enum LutError {
    #[error("the .cube file has no LUT_1D_SIZE declaration")]
    Missing1dSize,
    #[error("3D .cube LUTs are not supported; load a 1D LUT instead")]
    Unsupported3d,
    #[error("invalid .cube line {line}: {message}")]
    InvalidLine { line: usize, message: String },
    #[error("LUT domain must be 0 0 0 to 1 1 1 (found {found})")]
    UnsupportedDomain { found: String },
    #[error("LUT_1D_SIZE must be at least 2 (found {size})")]
    TooFewSamples { size: usize },
    #[error("LUT declares {declared} entries but contains {actual}")]
    EntryCount { declared: usize, actual: usize },
}

impl CalibrationLut {
    /// Build a LUT by resampling RGB triples uniformly spanning `0..1`.
    pub fn from_triples(name: impl Into<String>, samples: &[[f32; 3]]) -> Result<Self, LutError> {
        if samples.len() < 2 {
            return Err(LutError::TooFewSamples {
                size: samples.len(),
            });
        }

        Ok(Self {
            name: name.into(),
            red: resample(samples, 0),
            green: resample(samples, 1),
            blue: resample(samples, 2),
        })
    }

    /// A neutral curve, useful as a starting point and in tests.
    pub fn identity(name: impl Into<String>) -> Self {
        let mut channel = [0.0; LUT_SAMPLES];
        for (index, value) in channel.iter_mut().enumerate() {
            *value = index as f32 / (LUT_SAMPLES - 1) as f32;
        }
        Self {
            name: name.into(),
            red: channel,
            green: channel,
            blue: channel,
        }
    }

    /// True when the curves leave every sample unchanged. The renderer skips
    /// the LUT stage in that case.
    pub fn is_identity(&self) -> bool {
        (0..LUT_SAMPLES).all(|index| {
            let expected = index as f32 / (LUT_SAMPLES - 1) as f32;
            (self.red[index] - expected).abs() < 1e-4
                && (self.green[index] - expected).abs() < 1e-4
                && (self.blue[index] - expected).abs() < 1e-4
        })
    }

    /// Interpolate one channel. This mirrors the shader so the parser and
    /// renderer can be checked against the same expectation.
    pub fn sample(&self, channel: usize, input: f32) -> f32 {
        let table = match channel {
            0 => &self.red,
            1 => &self.green,
            _ => &self.blue,
        };
        let position = input.clamp(0.0, 1.0) * (LUT_SAMPLES - 1) as f32;
        let low = position.floor() as usize;
        let high = (low + 1).min(LUT_SAMPLES - 1);
        let t = position - low as f32;
        table[low] * (1.0 - t) + table[high] * t
    }
}

/// Parse a Resolve-style 1D `.cube` file. Only the standard `0..1` domain is
/// accepted, because the renderer feeds normalized display values into it.
pub fn parse_cube_1d(source: &str, fallback_name: &str) -> Result<CalibrationLut, LutError> {
    let mut title = None;
    let mut declared_size = None;
    let mut domain_min = [0.0_f32; 3];
    let mut domain_max = [1.0_f32; 3];
    let mut samples = Vec::new();

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let keyword = parts.next().expect("a non-empty line has a token");
        match keyword {
            "TITLE" => {
                title = Some(line[keyword.len()..].trim().trim_matches('"').to_owned());
            }
            "LUT_1D_SIZE" => {
                let value = parts.next().ok_or_else(|| LutError::InvalidLine {
                    line: line_number,
                    message: "LUT_1D_SIZE needs a value".to_owned(),
                })?;
                let size = value.parse::<usize>().map_err(|_| LutError::InvalidLine {
                    line: line_number,
                    message: format!("invalid LUT_1D_SIZE '{value}'"),
                })?;
                declared_size = Some(size);
            }
            "LUT_3D_SIZE" => return Err(LutError::Unsupported3d),
            "DOMAIN_MIN" | "DOMAIN_MAX" => {
                let values = parse_triple(parts, line_number)?;
                if keyword == "DOMAIN_MIN" {
                    domain_min = values;
                } else {
                    domain_max = values;
                }
            }
            _ => {
                let red = keyword.parse::<f32>().map_err(|_| LutError::InvalidLine {
                    line: line_number,
                    message: format!("expected an RGB triple, found '{keyword}'"),
                })?;
                let mut triple = [red, 0.0, 0.0];
                for value in &mut triple[1..] {
                    let token = parts.next().ok_or_else(|| LutError::InvalidLine {
                        line: line_number,
                        message: "expected three channel values".to_owned(),
                    })?;
                    *value = token.parse::<f32>().map_err(|_| LutError::InvalidLine {
                        line: line_number,
                        message: format!("invalid channel value '{token}'"),
                    })?;
                }
                if parts.next().is_some() {
                    return Err(LutError::InvalidLine {
                        line: line_number,
                        message: "expected exactly three channel values".to_owned(),
                    });
                }
                if triple.iter().any(|value| !value.is_finite()) {
                    return Err(LutError::InvalidLine {
                        line: line_number,
                        message: "channel values must be finite".to_owned(),
                    });
                }
                samples.push(triple.map(|value| value.clamp(0.0, 1.0)));
            }
        }
    }

    if domain_min != [0.0, 0.0, 0.0] || domain_max != [1.0, 1.0, 1.0] {
        return Err(LutError::UnsupportedDomain {
            found: format!("{domain_min:?} to {domain_max:?}"),
        });
    }

    let declared = declared_size.ok_or(LutError::Missing1dSize)?;
    if declared < 2 {
        return Err(LutError::TooFewSamples { size: declared });
    }
    if samples.len() != declared {
        return Err(LutError::EntryCount {
            declared,
            actual: samples.len(),
        });
    }

    let name = title
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| fallback_name.to_owned());
    CalibrationLut::from_triples(name, &samples)
}

fn parse_triple<'a>(
    mut values: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<[f32; 3], LutError> {
    let mut triple = [0.0_f32; 3];
    for value in &mut triple {
        let token = values.next().ok_or_else(|| LutError::InvalidLine {
            line,
            message: "expected three values".to_owned(),
        })?;
        *value = token.parse::<f32>().map_err(|_| LutError::InvalidLine {
            line,
            message: format!("invalid value '{token}'"),
        })?;
    }
    Ok(triple)
}

fn resample(samples: &[[f32; 3]], channel: usize) -> [f32; LUT_SAMPLES] {
    let last = samples.len() - 1;
    let mut table = [0.0; LUT_SAMPLES];
    for (index, value) in table.iter_mut().enumerate() {
        let position = index as f32 / (LUT_SAMPLES - 1) as f32 * last as f32;
        let low = position.floor() as usize;
        let high = (low + 1).min(last);
        let t = position - low as f32;
        *value = samples[low][channel] * (1.0 - t) + samples[high][channel] * t;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUBE: &str = r#"
        # Calibration example
        TITLE "Projector left"
        LUT_1D_SIZE 3
        DOMAIN_MIN 0 0 0
        DOMAIN_MAX 1 1 1
        0.0 0.0 0.0
        0.5 0.25 1.0
        1.0 0.5 0.2
    "#;

    #[test]
    fn parses_and_resamples_a_1d_cube() {
        let lut = parse_cube_1d(CUBE, "fallback").unwrap();
        assert_eq!(lut.name, "Projector left");
        assert!((lut.sample(0, 0.5) - 0.5).abs() < 1e-5);
        assert!((lut.sample(1, 0.5) - 0.25).abs() < 1e-5);
        assert!((lut.sample(2, 0.5) - 0.970_968).abs() < 1e-4);
        assert!((lut.sample(2, 0.75) - 0.6).abs() < 1e-5);
    }

    #[test]
    fn falls_back_to_the_file_name() {
        let source = "LUT_1D_SIZE 2\n0 0 0\n1 1 1\n";
        let lut = parse_cube_1d(source, "display.cube").unwrap();
        assert_eq!(lut.name, "display.cube");
        assert!(lut.is_identity());
    }

    #[test]
    fn identity_samples_are_neutral() {
        let lut = CalibrationLut::identity("neutral");
        for hole in [0.0, 0.25, 0.5, 0.75, 1.0] {
            for channel in 0..3 {
                assert!((lut.sample(channel, hole) - hole).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn rejects_3d_luts_and_wrong_entry_counts() {
        assert_eq!(
            parse_cube_1d("LUT_3D_SIZE 17\n", "x"),
            Err(LutError::Unsupported3d)
        );
        assert!(matches!(
            parse_cube_1d("LUT_1D_SIZE 2\n0 0 0\n", "x"),
            Err(LutError::EntryCount {
                declared: 2,
                actual: 1
            })
        ));
    }

    #[test]
    fn rejects_a_non_standard_domain() {
        let source = "LUT_1D_SIZE 2\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 2 2 2\n0 0 0\n1 1 1\n";
        assert!(matches!(
            parse_cube_1d(source, "x"),
            Err(LutError::UnsupportedDomain { .. })
        ));
    }

    #[test]
    fn clamps_out_of_range_values() {
        let low = parse_cube_1d("LUT_1D_SIZE 2\n-2 -2 -2\n-1 -1 -1\n", "x").unwrap();
        assert_eq!(low.red, [0.0; LUT_SAMPLES]);
        assert_eq!(low.blue, [0.0; LUT_SAMPLES]);

        let high = parse_cube_1d("LUT_1D_SIZE 2\n2 2 2\n3 3 3\n", "x").unwrap();
        assert!(high.red.iter().all(|value| *value == 1.0));
        assert!(high.blue.iter().all(|value| *value == 1.0));
    }
}
