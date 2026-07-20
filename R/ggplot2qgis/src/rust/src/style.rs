//! Bindings for the style types: `Rgb`, `GeometryType`, `VectorStyle`, and
//! `RasterStyle`.

use savvy::{IntegerSexp, NotAvailableValue, RealSexp, StringSexp, savvy, savvy_err};

use crate::{band_number, channel, class_count, color_stops, rgb_vec};

/// An opaque RGB color.
///
/// Create an instance with `Rgb$new(r, g, b)`, where each channel is an
/// integer between 0 and 255.
///
/// @export
#[savvy]
struct Rgb {
    pub(crate) inner: generate_qgs::Rgb,
}

#[savvy]
impl Rgb {
    /// Create a color from red, green, and blue channels (each 0-255).
    fn new(r: i32, g: i32, b: i32) -> savvy::Result<Self> {
        Ok(Self {
            inner: generate_qgs::Rgb::new(channel(r)?, channel(g)?, channel(b)?),
        })
    }
}

/// Geometry kind of a vector layer: `GeometryType$Point`,
/// `GeometryType$LineString`, or `GeometryType$Polygon`.
///
/// @export
#[savvy]
enum GeometryType {
    Point,
    LineString,
    Polygon,
}

// An impl block is needed for the R-side code of the enum to be generated.
#[savvy]
impl GeometryType {}

impl From<&GeometryType> for generate_qgs::GeometryType {
    fn from(g: &GeometryType) -> Self {
        match g {
            GeometryType::Point => generate_qgs::GeometryType::Point,
            GeometryType::LineString => generate_qgs::GeometryType::LineString,
            GeometryType::Polygon => generate_qgs::GeometryType::Polygon,
        }
    }
}

/// How a vector layer is rendered.
///
/// Create an instance with one of the associated functions:
///
/// - `VectorStyle$single(color)`: all features share one symbol.
/// - `VectorStyle$graduated(attribute, classes, min, max, stop_offsets,
///   stop_r, stop_g, stop_b)`: features are colored by an attribute value
///   along a color gradient. The gradient is described by control points:
///   `stop_offsets` are offsets in `0..=1` (the first must be `0` and the
///   last `1`), and `stop_r`/`stop_g`/`stop_b` are the corresponding color
///   channels.
/// - `VectorStyle$continuous(attribute, min, max, stop_offsets, stop_r,
///   stop_g, stop_b)`: like graduated, but the color is interpolated
///   continuously (no binning) via a data-defined color expression. The
///   legend shows a single swatch.
/// - `VectorStyle$categorized(attribute, values, colors_r, colors_g,
///   colors_b, catch_all)`: each discrete attribute value gets its own
///   color. `catch_all` is an optional `Rgb` for all other values.
///
/// A style can be adjusted in place after construction:
///
/// - `$set_outline(color, width)`: the constant outline (stroke) color
///   and width in millimeters (defaults: dark gray, 0.26).
/// - `$set_stroke_target(fill_color)`: moves the varying color of a
///   graduated/continuous/categorized style to the outline; all features
///   share the constant `fill_color`. Errors on a single style.
///
/// @export
#[savvy]
struct VectorStyle {
    pub(crate) inner: generate_qgs::VectorStyle,
}

#[savvy]
impl VectorStyle {
    /// Single symbol with the given fill color, dark gray 0.26 mm outline
    /// (QGIS defaults).
    fn single(color: &Rgb) -> Self {
        Self {
            inner: generate_qgs::VectorStyle::single(color.inner),
        }
    }

    /// Sets the constant outline (stroke) color and width in millimeters.
    /// For a style whose varying color targets the stroke, the width
    /// still applies but the color is ignored.
    fn set_outline(&mut self, color: &Rgb, width: f64) -> savvy::Result<()> {
        self.inner.set_outline(color.inner, width);
        Ok(())
    }

    /// Moves the varying color of a graduated, continuous, or categorized
    /// style to the outline (stroke); every feature shares the constant
    /// `fill_color`. Errors on a single-symbol style.
    fn set_stroke_target(&mut self, fill_color: &Rgb) -> savvy::Result<()> {
        self.inner.set_stroke_target(fill_color.inner)?;
        Ok(())
    }

    /// Graduated coloring of `attribute` with `classes` equal-interval
    /// ranges between `min` and `max`. Class colors are interpolated along
    /// the color stops.
    fn graduated(
        attribute: &str,
        classes: i32,
        min: f64,
        max: f64,
        stop_offsets: RealSexp,
        stop_r: IntegerSexp,
        stop_g: IntegerSexp,
        stop_b: IntegerSexp,
    ) -> savvy::Result<Self> {
        let stops = color_stops(&stop_offsets, &stop_r, &stop_g, &stop_b)?;
        let inner = generate_qgs::VectorStyle::graduated(
            attribute,
            class_count(classes)?,
            min,
            max,
            &stops,
        )?;
        Ok(Self { inner })
    }

    /// Continuous coloring of `attribute`: the color is interpolated along
    /// the color stops from the attribute value rescaled so that `min` is
    /// at offset `0` and `max` at `1` (values outside are clamped). Unlike
    /// `graduated`, there is no binning; the legend shows a single swatch.
    fn continuous(
        attribute: &str,
        min: f64,
        max: f64,
        stop_offsets: RealSexp,
        stop_r: IntegerSexp,
        stop_g: IntegerSexp,
        stop_b: IntegerSexp,
    ) -> savvy::Result<Self> {
        let stops = color_stops(&stop_offsets, &stop_r, &stop_g, &stop_b)?;
        let inner = generate_qgs::VectorStyle::continuous(attribute, min, max, &stops)?;
        Ok(Self { inner })
    }

    /// Discrete coloring of `attribute`: each value gets its own color.
    /// `catch_all`, if given, is the color of the "all other values"
    /// category.
    fn categorized(
        attribute: &str,
        values: StringSexp,
        colors_r: IntegerSexp,
        colors_g: IntegerSexp,
        colors_b: IntegerSexp,
        catch_all: Option<&Rgb>,
    ) -> savvy::Result<Self> {
        let colors = rgb_vec(&colors_r, &colors_g, &colors_b)?;
        let values: Vec<&str> = values.iter().collect();
        if values.iter().any(|v| v.is_na()) {
            return Err(savvy_err!("`values` must not contain NA"));
        }
        if values.len() != colors.len() {
            return Err(savvy_err!(
                "`values` and the color channels must have the same length"
            ));
        }
        let categories: Vec<(&str, generate_qgs::Rgb)> =
            values.into_iter().zip(colors).collect();
        let inner = generate_qgs::VectorStyle::categorized(
            attribute,
            &categories,
            catch_all.map(|c| c.inner),
        )?;
        Ok(Self { inner })
    }
}

/// How a raster layer is rendered.
///
/// Create an instance with one of the associated functions:
///
/// - `RasterStyle$pseudocolor(classes, min, max, stop_offsets, stop_r,
///   stop_g, stop_b)`: one band colored through a continuous color ramp.
/// - `RasterStyle$pseudocolor_discrete(...)`: the same, but with one flat
///   color per equal-interval class.
/// - `RasterStyle$multiband(red_band, red_min, red_max, green_band,
///   green_min, green_max, blue_band, blue_min, blue_max)`: three bands
///   mapped to the RGB channels. Bands are 1-based; the min/max are the
///   band statistics QGIS caches as the contrast stretch limits.
///
/// @export
#[savvy]
struct RasterStyle {
    pub(crate) inner: generate_qgs::RasterStyle,
}

#[savvy]
impl RasterStyle {
    /// Continuous pseudocolor of band 1 with `classes` equally spaced ramp
    /// entries between `min` and `max`.
    fn pseudocolor(
        classes: i32,
        min: f64,
        max: f64,
        stop_offsets: RealSexp,
        stop_r: IntegerSexp,
        stop_g: IntegerSexp,
        stop_b: IntegerSexp,
    ) -> savvy::Result<Self> {
        let stops = color_stops(&stop_offsets, &stop_r, &stop_g, &stop_b)?;
        let inner =
            generate_qgs::RasterStyle::pseudocolor(class_count(classes)?, min, max, &stops)?;
        Ok(Self { inner })
    }

    /// Discrete pseudocolor of band 1 with `classes` equal-interval classes
    /// between `min` and `max`.
    fn pseudocolor_discrete(
        classes: i32,
        min: f64,
        max: f64,
        stop_offsets: RealSexp,
        stop_r: IntegerSexp,
        stop_g: IntegerSexp,
        stop_b: IntegerSexp,
    ) -> savvy::Result<Self> {
        let stops = color_stops(&stop_offsets, &stop_r, &stop_g, &stop_b)?;
        let inner = generate_qgs::RasterStyle::pseudocolor_discrete(
            class_count(classes)?,
            min,
            max,
            &stops,
        )?;
        Ok(Self { inner })
    }

    /// True-color rendering of three bands. Each channel is specified as a
    /// band number (1-based) and the band's min/max statistics.
    fn multiband(
        red_band: i32,
        red_min: f64,
        red_max: f64,
        green_band: i32,
        green_min: f64,
        green_max: f64,
        blue_band: i32,
        blue_min: f64,
        blue_max: f64,
    ) -> savvy::Result<Self> {
        let inner = generate_qgs::RasterStyle::multiband(
            (band_number(red_band)?, red_min, red_max),
            (band_number(green_band)?, green_min, green_max),
            (band_number(blue_band)?, blue_min, blue_max),
        )?;
        Ok(Self { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_rejects_out_of_range() {
        assert!(channel(-1).is_err());
        assert!(channel(256).is_err());
        assert_eq!(channel(0).unwrap(), 0);
        assert_eq!(channel(255).unwrap(), 255);
    }

    #[test]
    fn class_count_rejects_negative() {
        assert!(class_count(-1).is_err());
        assert_eq!(class_count(6).unwrap(), 6);
    }

    #[test]
    fn band_number_rejects_zero_and_negative() {
        assert!(band_number(0).is_err());
        assert!(band_number(-1).is_err());
        assert_eq!(band_number(3).unwrap(), 3);
    }
}
