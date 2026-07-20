//! R bindings to the `generate-qgs` crate.
//!
//! These are raw-level wrappers: the R API mirrors the Rust API almost
//! one-to-one. Higher-level helpers (e.g. converting a ggplot2 object) are
//! built on top of these on the R side.

use savvy::{IntegerSexp, NotAvailableValue, NumericScalar, RealSexp, Sexp, savvy_err};

mod builder;
mod style;

/// Convert a single color channel into `u8`, rejecting out-of-range values.
fn channel(v: i32) -> savvy::Result<u8> {
    u8::try_from(v).map_err(|_| savvy_err!("color channel must be between 0 and 255, got {v}"))
}

fn rgb(r: i32, g: i32, b: i32) -> savvy::Result<generate_qgs::Rgb> {
    Ok(generate_qgs::Rgb::new(
        channel(r)?,
        channel(g)?,
        channel(b)?,
    ))
}

/// Convert parallel channel vectors into a vector of colors.
fn rgb_vec(
    r: &IntegerSexp,
    g: &IntegerSexp,
    b: &IntegerSexp,
) -> savvy::Result<Vec<generate_qgs::Rgb>> {
    let (r, g, b) = (r.as_slice(), g.as_slice(), b.as_slice());
    if g.len() != r.len() || b.len() != r.len() {
        return Err(savvy_err!(
            "color channel vectors must have the same length"
        ));
    }
    let mut out = Vec::with_capacity(r.len());
    for i in 0..r.len() {
        if r[i].is_na() || g[i].is_na() || b[i].is_na() {
            return Err(savvy_err!("color channels must not be NA"));
        }
        out.push(rgb(r[i], g[i], b[i])?);
    }
    Ok(out)
}

/// Zip offsets and parallel color channel vectors into color stops.
fn color_stops(
    offsets: &RealSexp,
    r: &IntegerSexp,
    g: &IntegerSexp,
    b: &IntegerSexp,
) -> savvy::Result<Vec<(f64, generate_qgs::Rgb)>> {
    let colors = rgb_vec(r, g, b)?;
    let offsets = offsets.as_slice();
    if offsets.len() != colors.len() {
        return Err(savvy_err!(
            "`stop_offsets` must have the same length as the color channels"
        ));
    }
    let mut out = Vec::with_capacity(offsets.len());
    for (i, &o) in offsets.iter().enumerate() {
        if !o.is_finite() {
            return Err(savvy_err!(
                "`stop_offsets` must not contain NA, NaN, or Inf"
            ));
        }
        out.push((o, colors[i]));
    }
    Ok(out)
}

/// A class count given as an R integer; reject negative values here because
/// the conversion to `usize` would silently wrap them into huge numbers.
fn class_count(classes: i32) -> savvy::Result<usize> {
    usize::try_from(classes).map_err(|_| savvy_err!("`classes` must be a positive integer"))
}

/// A 1-based band number.
fn band_number(band: i32) -> savvy::Result<u32> {
    u32::try_from(band).map_err(|_| savvy_err!("band numbers are 1-based, got {band}"))
}

/// Interpret the `srs` argument: an EPSG code (integer or numeric) or a
/// WKT2 string.
fn parse_srs(x: Sexp) -> savvy::Result<generate_qgs::Srs> {
    if x.is_string() {
        return Ok(generate_qgs::Srs::from(<&str>::try_from(x)?));
    }
    if x.is_numeric() {
        return Ok(generate_qgs::Srs::Epsg(
            NumericScalar::try_from(x)?.as_i32()?,
        ));
    }
    Err(savvy_err!(
        "`srs` must be an EPSG code (integer or numeric) or a WKT2 string"
    ))
}
