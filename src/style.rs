//! Symbology: colors, geometry types and `<renderer-v2>` generation.
//!
//! Reproduces the two renderer variants found in the samples:
//! `singleSymbol` (one style for all features) and `graduatedSymbol`
//! (color gradient driven by an attribute value).

use crate::ids::uuid;
use crate::xml::XmlWriter;

/// An opaque RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    /// Red channel, 0-255.
    pub r: u8,
    /// Green channel, 0-255.
    pub g: u8,
    /// Blue channel, 0-255.
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    /// QGIS color serialization: `R,G,B,255,rgb:r,g,b,1` where the `rgb:`
    /// part holds the channels as floats with up to 7 significant digits,
    /// e.g. `232,113,141,255,rgb:0.9098039,0.4431373,0.5529412,1`.
    pub(crate) fn qgis(&self) -> String {
        format!(
            "{},{},{},255,rgb:{},{},{},1",
            self.r,
            self.g,
            self.b,
            channel(self.r),
            channel(self.g),
            channel(self.b)
        )
    }

    /// `#rrggbb`, used by raster color ramp items.
    pub(crate) fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Linear interpolation in RGB space (matches QGIS color ramps).
    fn lerp(self, other: Rgb, t: f64) -> Rgb {
        let f = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
        Rgb::new(f(self.r, other.r), f(self.g, other.g), f(self.b, other.b))
    }
}

/// Formats one channel as a float in `0..=1` the way QGIS does: `%.7f` of
/// the 32-bit float, trailing zeros stripped (e.g. `0.9098039`, `0.682353`).
/// Verified against every channel value in the samples.
fn channel(v: u8) -> String {
    let s = format!("{:.7}", f32::from(v) / 255.0);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Geometry kind of a vector layer.
///
/// Sets the `geometry`/`wkbType` attributes of `<maplayer>` and selects the
/// symbol type (`marker`/`line`/`fill`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryType {
    Point,
    LineString,
    Polygon,
}

impl GeometryType {
    pub(crate) fn geometry_attr(self) -> &'static str {
        match self {
            GeometryType::Point => "Point",
            GeometryType::LineString => "LineString",
            GeometryType::Polygon => "Polygon",
        }
    }

    pub(crate) fn wkb_type_attr(self) -> &'static str {
        match self {
            GeometryType::Point => "MultiPoint",
            GeometryType::LineString => "MultiLineString",
            GeometryType::Polygon => "MultiPolygon",
        }
    }

    fn symbol_type(self) -> &'static str {
        match self {
            GeometryType::Point => "marker",
            GeometryType::LineString => "line",
            GeometryType::Polygon => "fill",
        }
    }
}

/// How a vector layer is rendered.
#[derive(Debug, Clone)]
pub enum VectorStyle {
    /// All features share one symbol (like `samples/load.qgs`).
    SingleSymbol(SimpleStyle),
    /// Features are colored by an attribute value along a color gradient
    /// (like `samples/red.qgs`).
    Graduated(GraduatedStyle),
    /// Features are colored by interpolating an attribute value along a
    /// color gradient, without binning.
    Continuous(ContinuousStyle),
    /// Each discrete attribute value gets its own color (like
    /// `samples/categorized.qgs`).
    Categorized(CategorizedStyle),
}

/// Style for [`VectorStyle::SingleSymbol`].
///
/// For polygons, `color` is the fill and `outline_*` the stroke. For lines,
/// `color` is the stroke color and `outline_width` the stroke width. For
/// points, `color` is the marker fill.
#[derive(Debug, Clone)]
pub struct SimpleStyle {
    pub color: Rgb,
    pub outline_color: Rgb,
    /// Stroke width in millimeters.
    pub outline_width: f64,
}

/// Style for [`VectorStyle::Graduated`].
///
/// Ranges are equal-interval between `min` and `max` (a "quick and dirty"
/// substitute for QGIS's "Pretty" classification); colors are sampled from
/// `color_stops`, a gradient of control points `(offset, color)` with
/// offsets in `0..=1` (like `samples/magma.qgs`).
#[derive(Debug, Clone)]
pub struct GraduatedStyle {
    /// Attribute (field) name to classify on.
    pub attribute: String,
    /// Number of classes (>= 2).
    pub classes: usize,
    pub min: f64,
    pub max: f64,
    /// Gradient control points `(offset, color)`, sorted by offset;
    /// the first must be at `0.0` and the last at `1.0`.
    pub color_stops: Vec<(f64, Rgb)>,
    pub outline_color: Rgb,
    /// Stroke width in millimeters.
    pub outline_width: f64,
}

/// Style for [`VectorStyle::Continuous`].
///
/// Rendered as a `singleSymbol` renderer whose symbol color is a
/// data-defined expression interpolating `color_stops` over the attribute
/// value — the vector counterpart of an `INTERPOLATED` raster ramp. The
/// legend shows a single swatch (QGIS cannot draw a continuous legend for a
/// data-defined vector color).
#[derive(Debug, Clone)]
pub struct ContinuousStyle {
    /// Attribute (field) name to interpolate on.
    pub attribute: String,
    /// Attribute value mapped to offset `0.0` of the gradient.
    pub min: f64,
    /// Attribute value mapped to offset `1.0` of the gradient.
    pub max: f64,
    /// Gradient control points `(offset, color)`, sorted by offset;
    /// the first must be at `0.0` and the last at `1.0`.
    pub color_stops: Vec<(f64, Rgb)>,
    pub outline_color: Rgb,
    /// Stroke width in millimeters.
    pub outline_width: f64,
}

/// Errors that can occur while constructing a style.
#[derive(Debug)]
pub enum StyleError {
    /// A graduated or pseudocolor style needs at least 2 classes.
    TooFewClasses(usize),
    /// `min` must be smaller than `max` (or not greater, for multiband
    /// channel limits).
    InvalidRange { min: f64, max: f64 },
    /// A color ramp needs at least 2 color stops.
    TooFewColorStops(usize),
    /// The first color stop must be at offset `0.0` and the last at `1.0`.
    BadColorStopEndpoints,
    /// Color stops must be in ascending offset order.
    NonAscendingColorStops,
    /// A categorized style needs at least 1 category.
    NoCategories,
    /// Band numbers are 1-based.
    InvalidBand(u32),
}

impl std::fmt::Display for StyleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StyleError::TooFewClasses(n) => {
                write!(f, "style needs at least 2 classes, got {n}")
            }
            StyleError::InvalidRange { min, max } => {
                write!(f, "invalid range: min ({min}) must be smaller than max ({max})")
            }
            StyleError::TooFewColorStops(n) => {
                write!(f, "style needs at least 2 color stops, got {n}")
            }
            StyleError::BadColorStopEndpoints => {
                write!(f, "first color stop must be at offset 0.0 and last at 1.0")
            }
            StyleError::NonAscendingColorStops => {
                write!(f, "color stops must be in ascending offset order")
            }
            StyleError::NoCategories => write!(f, "categorized style needs at least 1 category"),
            StyleError::InvalidBand(band) => {
                write!(f, "band numbers are 1-based, got {band}")
            }
        }
    }
}

impl std::error::Error for StyleError {}

/// Validates the shared constraints of ramp-based styles (graduated vector
/// symbols and pseudocolor rasters).
fn validate_ramp(
    classes: usize,
    min: f64,
    max: f64,
    color_stops: &[(f64, Rgb)],
) -> Result<(), StyleError> {
    if classes < 2 {
        return Err(StyleError::TooFewClasses(classes));
    }
    if min >= max {
        return Err(StyleError::InvalidRange { min, max });
    }
    validate_color_stops(color_stops)
}

fn validate_color_stops(color_stops: &[(f64, Rgb)]) -> Result<(), StyleError> {
    if color_stops.len() < 2 {
        return Err(StyleError::TooFewColorStops(color_stops.len()));
    }
    if color_stops.first().unwrap().0 != 0.0 || color_stops.last().unwrap().0 != 1.0 {
        return Err(StyleError::BadColorStopEndpoints);
    }
    if !color_stops.windows(2).all(|w| w[0].0 < w[1].0) {
        return Err(StyleError::NonAscendingColorStops);
    }
    Ok(())
}

/// Style for [`VectorStyle::Categorized`].
///
/// Only the color (and the shared outline) of each category symbol is
/// customized; all other symbol attributes use QGIS's default values.
#[derive(Debug, Clone)]
pub struct CategorizedStyle {
    /// Attribute (field) name to classify on.
    pub attribute: String,
    /// `(value, color)` pairs, in legend order. Values are matched as
    /// strings (`type="string"`).
    pub categories: Vec<(String, Rgb)>,
    /// Optional catch-all category ("all other values"), rendered as
    /// `<category type="NULL" value="NULL"/>`.
    pub catch_all: Option<Rgb>,
    pub outline_color: Rgb,
    /// Stroke width in millimeters.
    pub outline_width: f64,
}

/// How a raster layer is rendered.
#[derive(Debug, Clone)]
pub enum RasterStyle {
    /// One band colored through a color ramp (like `samples/elevation.qgs`
    /// and `samples/elevation_discrete.qgs`).
    SingleBandPseudocolor(PseudocolorStyle),
    /// Three bands mapped to the RGB channels (like
    /// `samples/true-color.qgs`).
    MultibandColor(MultibandColorStyle),
}

/// Continuous vs. discrete coloring for [`RasterStyle::SingleBandPseudocolor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudocolorMode {
    /// `colorRampType="INTERPOLATED"`: colors interpolate smoothly between
    /// `min` and `max`.
    Interpolated,
    /// `colorRampType="DISCRETE"`: one flat color per equal-interval class.
    Discrete,
}

/// Style for [`RasterStyle::SingleBandPseudocolor`].
///
/// Class values are equal-interval between `min` and `max`; class colors
/// are sampled from `color_stops`, a gradient of control points
/// `(offset, color)` with offsets in `0..=1` — the same sampling rule as
/// [`VectorStyle::Graduated`].
#[derive(Debug, Clone)]
pub struct PseudocolorStyle {
    /// Band to colorize (1-based).
    pub band: u32,
    pub mode: PseudocolorMode,
    /// Number of classes (>= 2).
    pub classes: usize,
    pub min: f64,
    pub max: f64,
    /// Gradient control points `(offset, color)`, sorted by offset;
    /// the first must be at `0.0` and the last at `1.0`.
    pub color_stops: Vec<(f64, Rgb)>,
}

/// Style for [`RasterStyle::MultibandColor`].
///
/// Each channel is a `(band, min, max)` tuple: the band number (1-based)
/// and the band's min/max statistics, which QGIS caches as the contrast
/// stretch limits. With the default `NoEnhancement` algorithm the values
/// are rendered as-is, so the limits only matter if the user switches on
/// stretching inside QGIS.
#[derive(Debug, Clone)]
pub struct MultibandColorStyle {
    pub red: (u32, f64, f64),
    pub green: (u32, f64, f64),
    pub blue: (u32, f64, f64),
}

impl RasterStyle {
    /// Continuous pseudocolor of band 1 with `classes` equally spaced ramp
    /// entries between `min` and `max` (like `samples/elevation.qgs`).
    /// `color_stops` follows the same rules as [`VectorStyle::graduated`].
    ///
    /// Returns an error if the constraints documented in
    /// [`VectorStyle::graduated`] are violated.
    pub fn pseudocolor(
        classes: usize,
        min: f64,
        max: f64,
        color_stops: &[(f64, Rgb)],
    ) -> Result<Self, StyleError> {
        Self::pseudocolor_with_mode(
            PseudocolorMode::Interpolated,
            classes,
            min,
            max,
            color_stops,
        )
    }

    /// Discrete pseudocolor of band 1 with `classes` equal-interval classes
    /// between `min` and `max` (like `samples/elevation_discrete.qgs`).
    ///
    /// Returns an error if the constraints documented in
    /// [`VectorStyle::graduated`] are violated.
    pub fn pseudocolor_discrete(
        classes: usize,
        min: f64,
        max: f64,
        color_stops: &[(f64, Rgb)],
    ) -> Result<Self, StyleError> {
        Self::pseudocolor_with_mode(PseudocolorMode::Discrete, classes, min, max, color_stops)
    }

    fn pseudocolor_with_mode(
        mode: PseudocolorMode,
        classes: usize,
        min: f64,
        max: f64,
        color_stops: &[(f64, Rgb)],
    ) -> Result<Self, StyleError> {
        validate_ramp(classes, min, max, color_stops)?;
        Ok(RasterStyle::SingleBandPseudocolor(PseudocolorStyle {
            band: 1,
            mode,
            classes,
            min,
            max,
            color_stops: color_stops.to_vec(),
        }))
    }

    /// True-color rendering of three bands (like `samples/true-color.qgs`).
    /// Each channel is `(band, min, max)`; see [`MultibandColorStyle`].
    ///
    /// Returns an error if a band number is 0 or a channel's `min` is
    /// greater than its `max`.
    pub fn multiband(
        red: (u32, f64, f64),
        green: (u32, f64, f64),
        blue: (u32, f64, f64),
    ) -> Result<Self, StyleError> {
        for (band, min, max) in [red, green, blue] {
            if band == 0 {
                return Err(StyleError::InvalidBand(band));
            }
            if min > max {
                return Err(StyleError::InvalidRange { min, max });
            }
        }
        Ok(RasterStyle::MultibandColor(MultibandColorStyle {
            red,
            green,
            blue,
        }))
    }

    /// Number of `<noDataList>` entries to emit: one per band, up to the
    /// highest band referenced by the style.
    pub(crate) fn band_count(&self) -> u32 {
        match self {
            RasterStyle::SingleBandPseudocolor(p) => p.band,
            RasterStyle::MultibandColor(m) => m.red.0.max(m.green.0).max(m.blue.0),
        }
    }
}

impl VectorStyle {
    /// Single symbol with the given fill color, dark gray 0.26 mm outline
    /// (QGIS defaults).
    pub fn single(color: Rgb) -> Self {
        VectorStyle::SingleSymbol(SimpleStyle {
            color,
            outline_color: Rgb::new(35, 35, 35),
            outline_width: 0.26,
        })
    }

    /// Discrete coloring of `attribute`: each `(value, color)` pair becomes
    /// one `<category>` linked to one `<symbol>`. `catch_all`, if given, is
    /// the color of the trailing "all other values" (`value="NULL"`)
    /// category.
    ///
    /// Returns an error if `categories` is empty.
    pub fn categorized(
        attribute: impl Into<String>,
        categories: &[(impl AsRef<str>, Rgb)],
        catch_all: Option<Rgb>,
    ) -> Result<Self, StyleError> {
        if categories.is_empty() {
            return Err(StyleError::NoCategories);
        }
        Ok(VectorStyle::Categorized(CategorizedStyle {
            attribute: attribute.into(),
            categories: categories
                .iter()
                .map(|(value, color)| (value.as_ref().to_string(), *color))
                .collect(),
            catch_all,
            outline_color: Rgb::new(35, 35, 35),
            outline_width: 0.26,
        }))
    }

    /// Continuous coloring of `attribute`: the color is interpolated along
    /// `color_stops` from the attribute value rescaled so that `min` is at
    /// offset `0.0` and `max` at `1.0` (values outside are clamped).
    /// `color_stops` follows the same rules as [`VectorStyle::graduated`].
    ///
    /// Returns an error if `min >= max` or the color stops are invalid.
    pub fn continuous(
        attribute: impl Into<String>,
        min: f64,
        max: f64,
        color_stops: &[(f64, Rgb)],
    ) -> Result<Self, StyleError> {
        if min >= max {
            return Err(StyleError::InvalidRange { min, max });
        }
        validate_color_stops(color_stops)?;
        Ok(VectorStyle::Continuous(ContinuousStyle {
            attribute: attribute.into(),
            min,
            max,
            color_stops: color_stops.to_vec(),
            outline_color: Rgb::new(35, 35, 35),
            outline_width: 0.26,
        }))
    }

    /// Graduated coloring of `attribute` with `classes` equal-interval
    /// ranges between `min` and `max`. Class colors are interpolated along
    /// `color_stops`, a list of `(offset, color)` control points with
    /// offsets in `0..=1`: the first entry must be at `0.0`, the last at
    /// `1.0`. A plain two-color ramp is `&[(0.0, from), (1.0, to)]`.
    ///
    /// Returns an error if `classes < 2`, `min >= max`, there are fewer
    /// than 2 color stops, the stop offsets don't start at `0.0` and end at
    /// `1.0`, or they are not in ascending order.
    pub fn graduated(
        attribute: impl Into<String>,
        classes: usize,
        min: f64,
        max: f64,
        color_stops: &[(f64, Rgb)],
    ) -> Result<Self, StyleError> {
        validate_ramp(classes, min, max, color_stops)?;
        Ok(VectorStyle::Graduated(GraduatedStyle {
            attribute: attribute.into(),
            classes,
            min,
            max,
            color_stops: color_stops.to_vec(),
            outline_color: Rgb::new(35, 35, 35),
            outline_width: 0.26,
        }))
    }
}

/// The recurring `<(data[-_]defined[-_]properties)>` boilerplate.
fn write_data_defined_properties(w: &mut XmlWriter, tag: &str) {
    write_data_defined_properties_with(w, tag, None);
}

/// Like [`write_data_defined_properties`], but with an optional
/// `(property_name, expression)` override in the `properties` map.
fn write_data_defined_properties_with(
    w: &mut XmlWriter,
    tag: &str,
    property: Option<(&str, &str)>,
) {
    w.start(tag);
    w.start("Option").attr("type", "Map");
    w.empty(
        "Option",
        &[("name", "name"), ("type", "QString"), ("value", "")],
    );
    match property {
        None => {
            w.empty("Option", &[("name", "properties")]);
        }
        Some((name, expression)) => {
            w.start("Option").attr("name", "properties").attr("type", "Map");
            w.start("Option").attr("name", name).attr("type", "Map");
            w.empty(
                "Option",
                &[("name", "active"), ("type", "bool"), ("value", "true")],
            );
            w.empty(
                "Option",
                &[
                    ("name", "expression"),
                    ("type", "QString"),
                    ("value", expression),
                ],
            );
            // 3 = expression-based property (QgsProperty::ExpressionBasedProperty).
            w.empty("Option", &[("name", "type"), ("type", "int"), ("value", "3")]);
            w.end(); // Option (property)
            w.end(); // Option (properties)
        }
    }
    w.empty(
        "Option",
        &[
            ("name", "type"),
            ("type", "QString"),
            ("value", "collection"),
        ],
    );
    w.end(); // Option
    w.end(); // tag
}

/// Formats a number the way QGIS writes widths (`0.26`, `1`, ...).
fn num(v: f64) -> String {
    v.to_string()
}

/// Samples the color ramp at `t` (in `0..=1`) by piecewise linear
/// interpolation in RGB space between consecutive control points.
fn sample_ramp(stops: &[(f64, Rgb)], t: f64) -> Rgb {
    if t <= stops[0].0 {
        return stops[0].1;
    }
    if t >= stops[stops.len() - 1].0 {
        return stops[stops.len() - 1].1;
    }
    let i = stops
        .iter()
        .position(|&(offset, _)| offset > t)
        .expect("t is within the ramp");
    let (o0, c0) = stops[i - 1];
    let (o1, c1) = stops[i];
    let f = (t - o0) / (o1 - o0);
    c0.lerp(c1, f)
}

/// Formats a gradient stop offset like QGIS does (`%g` with 6 significant
/// digits, e.g. `0.0196078`, `0.509804`, `0.5`, `1`).
fn g6(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let exp = v.abs().log10().floor() as i32;
    if !(-4..6).contains(&exp) {
        // Scientific notation, as C's `%g` does (e.g. `1.23457e-05`).
        let s = format!("{v:.5e}");
        let (mantissa, exponent) = s.split_once('e').unwrap();
        let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
        let e: i32 = exponent.parse().unwrap();
        return format!(
            "{}e{}{:02}",
            mantissa,
            if e < 0 { '-' } else { '+' },
            e.abs()
        );
    }
    let decimals = (5 - exp).max(0) as usize;
    format!("{v:.decimals$}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Options of the symbol layer (`<Option type="Map">` children), sorted by
/// name as QGIS writes them.
fn symbol_options(
    geom: GeometryType,
    color: Rgb,
    outline_color: Rgb,
    outline_width: f64,
) -> Vec<(&'static str, String)> {
    const SCALE: &str = "3x:0,0,0,0,0,0";
    match geom {
        GeometryType::Polygon => vec![
            ("border_width_map_unit_scale", SCALE.into()),
            ("color", color.qgis()),
            ("joinstyle", "bevel".into()),
            ("offset", "0,0".into()),
            ("offset_map_unit_scale", SCALE.into()),
            ("offset_unit", "MM".into()),
            ("outline_color", outline_color.qgis()),
            ("outline_style", "solid".into()),
            ("outline_width", num(outline_width)),
            ("outline_width_unit", "MM".into()),
            ("style", "solid".into()),
        ],
        GeometryType::LineString => vec![
            ("align_dash_pattern", "0".into()),
            ("capstyle", "square".into()),
            ("customdash", "5;2".into()),
            ("customdash_map_unit_scale", SCALE.into()),
            ("customdash_unit", "MM".into()),
            ("dash_pattern_offset", "0".into()),
            ("dash_pattern_offset_map_unit_scale", SCALE.into()),
            ("dash_pattern_offset_unit", "MM".into()),
            ("draw_inside_polygon", "0".into()),
            ("joinstyle", "bevel".into()),
            ("line_color", color.qgis()),
            ("line_style", "solid".into()),
            ("line_width", num(outline_width)),
            ("line_width_unit", "MM".into()),
            ("offset", "0".into()),
            ("offset_map_unit_scale", SCALE.into()),
            ("offset_unit", "MM".into()),
            ("ring_filter", "0".into()),
            ("trim_distance_end", "0".into()),
            ("trim_distance_end_map_unit_scale", SCALE.into()),
            ("trim_distance_end_unit", "MM".into()),
            ("trim_distance_start", "0".into()),
            ("trim_distance_start_map_unit_scale", SCALE.into()),
            ("trim_distance_start_unit", "MM".into()),
            ("tweak_dash_pattern_on_corners", "0".into()),
            ("use_custom_dash", "0".into()),
            ("width_map_unit_scale", SCALE.into()),
        ],
        GeometryType::Point => vec![
            ("angle", "0".into()),
            ("cap_style", "square".into()),
            ("color", color.qgis()),
            ("horizontal_anchor_point", "1".into()),
            ("joinstyle", "bevel".into()),
            ("name", "circle".into()),
            ("offset", "0,0".into()),
            ("offset_map_unit_scale", SCALE.into()),
            ("offset_unit", "MM".into()),
            ("outline_color", outline_color.qgis()),
            ("outline_style", "solid".into()),
            ("outline_width", num(outline_width)),
            ("outline_width_map_unit_scale", SCALE.into()),
            ("outline_width_unit", "MM".into()),
            ("scale_method", "diameter".into()),
            ("size", "2".into()),
            ("size_map_unit_scale", SCALE.into()),
            ("size_unit", "MM".into()),
            ("vertical_anchor_point", "1".into()),
        ],
    }
}

/// Writes a `<symbol>` element with a single symbol layer.
fn write_symbol(
    w: &mut XmlWriter,
    name: &str,
    geom: GeometryType,
    color: Rgb,
    outline_color: Rgb,
    outline_width: f64,
) {
    write_symbol_with_dd_color(w, name, geom, color, outline_color, outline_width, None);
}

/// The data-defined property that drives the main color of a symbol layer,
/// as QGIS serializes it (`QgsSymbolLayer::propertyDefinitions()`).
fn color_property_name(geom: GeometryType) -> &'static str {
    match geom {
        // SimpleMarker and SimpleFill color both map to PropertyFillColor.
        GeometryType::Point | GeometryType::Polygon => "fillColor",
        // SimpleLine color maps to PropertyStrokeColor.
        GeometryType::LineString => "outlineColor",
    }
}

/// Like [`write_symbol`], but the symbol layer's main color can carry a
/// data-defined expression override.
#[allow(clippy::too_many_arguments)]
fn write_symbol_with_dd_color(
    w: &mut XmlWriter,
    name: &str,
    geom: GeometryType,
    color: Rgb,
    outline_color: Rgb,
    outline_width: f64,
    color_expression: Option<&str>,
) {
    let class = match geom {
        GeometryType::Point => "SimpleMarker",
        GeometryType::LineString => "SimpleLine",
        GeometryType::Polygon => "SimpleFill",
    };
    w.start("symbol")
        .attr("alpha", "1")
        .attr("clip_to_extent", "1")
        .attr("force_rhr", "0")
        .attr("frame_rate", "10")
        .attr("is_animated", "0")
        .attr("name", name)
        .attr("type", geom.symbol_type());
    write_data_defined_properties(w, "data_defined_properties");
    w.start("layer")
        .attr("class", class)
        .attr("enabled", "1")
        .attr("id", format!("{{{}}}", uuid()))
        .attr("locked", "0")
        .attr("pass", "0");
    w.start("Option").attr("type", "Map");
    for (name, value) in symbol_options(geom, color, outline_color, outline_width) {
        w.empty(
            "Option",
            &[("name", name), ("type", "QString"), ("value", &value)],
        );
    }
    w.end(); // Option
    write_data_defined_properties_with(
        w,
        "data_defined_properties",
        color_expression.map(|expr| (color_property_name(geom), expr)),
    );
    w.end(); // layer
    w.end(); // symbol
}

/// Escapes a field name as a double-quoted QGIS expression identifier.
fn quote_field(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// The `ramp_color(create_ramp(...), ...)` expression interpolating
/// `color_stops` over the attribute value rescaled from `min..=max` to
/// `0..=1`. `ramp_color()` clamps values outside the ramp.
fn continuous_color_expression(
    attribute: &str,
    min: f64,
    max: f64,
    color_stops: &[(f64, Rgb)],
) -> String {
    let map_args = color_stops
        .iter()
        .map(|(offset, color)| format!("{},'{}'", g6(*offset), color.hex()))
        .collect::<Vec<_>>()
        .join(",");
    // The span is spelled out as `max - min` so both numbers keep their
    // exact user-facing form (subtracting in f64 first would leak noise
    // like `0.19899999999999998` into the expression).
    format!(
        "ramp_color(create_ramp(map({map_args})),({field} - {min}) / ({max} - {min}))",
        field = quote_field(attribute),
        min = num(min),
        max = num(max)
    )
}

/// Writes the `<colorramp name="[source]" type="gradient">` element for the
/// `[start, end]` endpoints plus optional intermediate `stops`.
fn write_gradient_colorramp(w: &mut XmlWriter, start: Rgb, end: Rgb, stops: &[(f64, Rgb)]) {
    w.start("colorramp")
        .attr("name", "[source]")
        .attr("type", "gradient");
    w.start("Option").attr("type", "Map");
    for (name, value) in [
        ("color1", start.qgis()),
        ("color2", end.qgis()),
        ("direction", "ccw".to_string()),
        ("discrete", "0".to_string()),
        ("rampType", "gradient".to_string()),
        ("spec", "rgb".to_string()),
    ] {
        w.empty(
            "Option",
            &[("name", name), ("type", "QString"), ("value", &value)],
        );
    }
    // Intermediate control points, if any:
    // `offset;color;rgb;ccw:offset;color;rgb;ccw:...`
    if !stops.is_empty() {
        let stops = stops
            .iter()
            .map(|(offset, color)| format!("{};{};rgb;ccw", g6(*offset), color.qgis()))
            .collect::<Vec<_>>()
            .join(":");
        w.empty(
            "Option",
            &[("name", "stops"), ("type", "QString"), ("value", &stops)],
        );
    }
    w.end(); // Option
    w.end(); // colorramp
}

/// Label for a graduated range, following the
/// `<labelFormat format="%1 - %2" labelprecision="1" trimtrailingzeroes="1"/>`
/// convention (e.g. `0 - 10`, `9.5 - 19`).
fn range_label(lower: f64, upper: f64) -> String {
    fn bound(v: f64) -> String {
        let s = format!("{v:.1}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
    format!("{} - {}", bound(lower), bound(upper))
}

/// Writes the `<renderer-v2>` element for a vector layer.
pub(crate) fn write_renderer(w: &mut XmlWriter, geom: GeometryType, style: &VectorStyle) {
    match style {
        VectorStyle::SingleSymbol(s) => {
            w.start("renderer-v2")
                .attr("enableorderby", "0")
                .attr("forceraster", "0")
                .attr("referencescale", "-1")
                .attr("symbollevels", "0")
                .attr("type", "singleSymbol");
            w.start("symbols");
            write_symbol(w, "0", geom, s.color, s.outline_color, s.outline_width);
            w.end(); // symbols
            w.empty("rotation", &[]);
            w.empty("sizescale", &[]);
            write_data_defined_properties(w, "data-defined-properties");
            w.end(); // renderer-v2
        }
        VectorStyle::Continuous(c) => {
            let expression =
                continuous_color_expression(&c.attribute, c.min, c.max, &c.color_stops);
            w.start("renderer-v2")
                .attr("enableorderby", "0")
                .attr("forceraster", "0")
                .attr("referencescale", "-1")
                .attr("symbollevels", "0")
                .attr("type", "singleSymbol");
            w.start("symbols");
            // The static color (also the legend swatch) is the middle of
            // the ramp; per feature it is overridden by the expression.
            write_symbol_with_dd_color(
                w,
                "0",
                geom,
                sample_ramp(&c.color_stops, 0.5),
                c.outline_color,
                c.outline_width,
                Some(&expression),
            );
            w.end(); // symbols
            w.empty("rotation", &[]);
            w.empty("sizescale", &[]);
            write_data_defined_properties(w, "data-defined-properties");
            w.end(); // renderer-v2
        }
        VectorStyle::Graduated(g) => {
            let step = (g.max - g.min) / g.classes as f64;
            w.start("renderer-v2")
                .attr("attr", &g.attribute)
                .attr("enableorderby", "0")
                .attr("forceraster", "0")
                .attr("graduatedMethod", "GraduatedColor")
                .attr("referencescale", "-1")
                .attr("symbollevels", "0")
                .attr("type", "graduatedSymbol");
            w.start("ranges");
            for i in 0..g.classes {
                let lower = g.min + step * i as f64;
                let upper = lower + step;
                w.start("range")
                    .attr("label", range_label(lower, upper))
                    .attr("lower", format!("{lower:.15}"))
                    .attr("render", "true")
                    .attr("symbol", i.to_string())
                    .attr("upper", format!("{upper:.15}"))
                    .attr("uuid", format!("{{{}}}", uuid()));
                w.end();
            }
            w.end(); // ranges
            w.start("symbols");
            for i in 0..g.classes {
                let t = i as f64 / (g.classes - 1) as f64;
                let color = sample_ramp(&g.color_stops, t);
                write_symbol(
                    w,
                    &i.to_string(),
                    geom,
                    color,
                    g.outline_color,
                    g.outline_width,
                );
            }
            w.end(); // symbols
            w.start("source-symbol");
            write_symbol(
                w,
                "0",
                geom,
                g.color_stops[0].1,
                g.outline_color,
                g.outline_width,
            );
            w.end(); // source-symbol
            write_gradient_colorramp(
                w,
                g.color_stops[0].1,
                g.color_stops[g.color_stops.len() - 1].1,
                &g.color_stops[1..g.color_stops.len() - 1],
            );
            w.start("classificationMethod").attr("id", "Pretty");
            w.empty(
                "symmetricMode",
                &[("astride", "0"), ("enabled", "0"), ("symmetrypoint", "0")],
            );
            w.empty(
                "labelFormat",
                &[
                    ("format", "%1 - %2"),
                    ("labelprecision", "1"),
                    ("trimtrailingzeroes", "1"),
                ],
            );
            w.start("parameters");
            w.empty("Option", &[]);
            w.end(); // parameters
            w.empty("extraInformation", &[]);
            w.end(); // classificationMethod
            w.empty("rotation", &[]);
            w.empty("sizescale", &[]);
            write_data_defined_properties(w, "data-defined-properties");
            w.end(); // renderer-v2
        }
        VectorStyle::Categorized(c) => {
            w.start("renderer-v2")
                .attr("attr", &c.attribute)
                .attr("enableorderby", "0")
                .attr("forceraster", "0")
                .attr("referencescale", "-1")
                .attr("symbollevels", "0")
                .attr("type", "categorizedSymbol");
            w.start("categories");
            for (i, (value, _)) in c.categories.iter().enumerate() {
                w.start("category")
                    .attr("label", value)
                    .attr("render", "true")
                    .attr("symbol", i.to_string())
                    .attr("type", "string")
                    .attr("uuid", format!("{{{}}}", uuid()))
                    .attr("value", value);
                w.end();
            }
            if c.catch_all.is_some() {
                w.start("category")
                    .attr("label", "")
                    .attr("render", "true")
                    .attr("symbol", c.categories.len().to_string())
                    .attr("type", "NULL")
                    .attr("uuid", format!("{{{}}}", uuid()))
                    .attr("value", "NULL");
                w.end();
            }
            w.end(); // categories
            w.start("symbols");
            for (i, (_, color)) in c.categories.iter().enumerate() {
                write_symbol(
                    w,
                    &i.to_string(),
                    geom,
                    *color,
                    c.outline_color,
                    c.outline_width,
                );
            }
            if let Some(color) = c.catch_all {
                write_symbol(
                    w,
                    &c.categories.len().to_string(),
                    geom,
                    color,
                    c.outline_color,
                    c.outline_width,
                );
            }
            w.end(); // symbols
            w.start("source-symbol");
            write_symbol(
                w,
                "0",
                geom,
                c.categories[0].1,
                c.outline_color,
                c.outline_width,
            );
            w.end(); // source-symbol
            // The colorramp is only informational for a categorized
            // renderer (used when re-classifying); derive it from the
            // first/last category color.
            let last_color = c
                .catch_all
                .unwrap_or(c.categories[c.categories.len() - 1].1);
            write_gradient_colorramp(w, c.categories[0].1, last_color, &[]);
            w.empty("rotation", &[]);
            w.empty("sizescale", &[]);
            write_data_defined_properties(w, "data-defined-properties");
            w.end(); // renderer-v2
        }
    }
}

/// The `<minMaxOrigin>` block inside `<rasterrenderer>` (raster layers).
/// `limits` is `None` for user-defined bounds, `MinMax` when they come
/// from band statistics.
pub(crate) fn write_min_max_origin(w: &mut XmlWriter, limits: &str) {
    w.start("minMaxOrigin");
    w.elem("limits", limits);
    w.elem("extent", "WholeRaster");
    w.elem("statAccuracy", "Estimated");
    w.elem("cumulativeCutLower", "0.02");
    w.elem("cumulativeCutUpper", "0.98");
    w.elem("stdDevFactor", "2");
    w.end(); // minMaxOrigin
}

/// The static `<rampLegendSettings>` block of a pseudocolor shader.
fn write_ramp_legend_settings(w: &mut XmlWriter) {
    w.start("rampLegendSettings")
        .attr("direction", "0")
        .attr("maximumLabel", "")
        .attr("minimumLabel", "")
        .attr("orientation", "2")
        .attr("prefix", "")
        .attr("suffix", "")
        .attr("useContinuousLegend", "1");
    w.start("numericFormat").attr("id", "basic");
    w.start("Option").attr("type", "Map");
    w.empty(
        "Option",
        &[("name", "decimal_separator"), ("type", "invalid")],
    );
    w.empty(
        "Option",
        &[("name", "decimals"), ("type", "int"), ("value", "6")],
    );
    w.empty(
        "Option",
        &[("name", "rounding_type"), ("type", "int"), ("value", "0")],
    );
    w.empty(
        "Option",
        &[("name", "show_plus"), ("type", "bool"), ("value", "false")],
    );
    w.empty(
        "Option",
        &[
            ("name", "show_thousand_separator"),
            ("type", "bool"),
            ("value", "true"),
        ],
    );
    w.empty(
        "Option",
        &[
            ("name", "show_trailing_zeros"),
            ("type", "bool"),
            ("value", "false"),
        ],
    );
    w.empty(
        "Option",
        &[("name", "thousand_separator"), ("type", "invalid")],
    );
    w.end(); // Option
    w.end(); // numericFormat
    w.end(); // rampLegendSettings
}

/// One `<item>` of a pseudocolor shader.
fn write_shader_item(w: &mut XmlWriter, color: Rgb, label: &str, value: &str) {
    w.empty(
        "item",
        &[
            ("alpha", "255"),
            ("color", &color.hex()),
            ("label", label),
            ("value", value),
        ],
    );
}

/// Writes the `<rasterrenderer>` element for a raster layer.
pub(crate) fn write_raster_renderer(w: &mut XmlWriter, style: &RasterStyle) {
    match style {
        RasterStyle::SingleBandPseudocolor(p) => {
            let n = p.classes;
            let (ramp_type, classification_mode) = match p.mode {
                PseudocolorMode::Interpolated => ("INTERPOLATED", "1"),
                PseudocolorMode::Discrete => ("DISCRETE", "2"),
            };
            w.start("rasterrenderer")
                .attr("alphaBand", "-1")
                .attr("band", p.band)
                .attr("classificationMax", num(p.max))
                .attr("classificationMin", num(p.min))
                .attr("nodataColor", "")
                .attr("opacity", "1")
                .attr("type", "singlebandpseudocolor");
            w.empty("rasterTransparency", &[]);
            write_min_max_origin(w, "None");
            w.start("rastershader");
            w.start("colorrampshader")
                .attr("classificationMode", classification_mode)
                .attr("clip", "0")
                .attr("colorRampType", ramp_type)
                .attr("labelPrecision", "0")
                .attr("maximumValue", num(p.max))
                .attr("minimumValue", num(p.min));
            write_gradient_colorramp(
                w,
                p.color_stops[0].1,
                p.color_stops[p.color_stops.len() - 1].1,
                &p.color_stops[1..p.color_stops.len() - 1],
            );
            // Class colors are sampled along the ramp at i/(n-1), the same
            // rule as the vector graduated renderer.
            match p.mode {
                PseudocolorMode::Interpolated => {
                    for i in 0..n {
                        let t = i as f64 / (n - 1) as f64;
                        let value = num(p.min + (p.max - p.min) * t);
                        write_shader_item(w, sample_ramp(&p.color_stops, t), &value, &value);
                    }
                }
                PseudocolorMode::Discrete => {
                    let step = (p.max - p.min) / n as f64;
                    for i in 0..n {
                        let t = i as f64 / (n - 1) as f64;
                        let color = sample_ramp(&p.color_stops, t);
                        let lower = p.min + i as f64 * step;
                        if i == n - 1 {
                            // Open-ended top class.
                            write_shader_item(w, color, &format!("> {}", num(lower)), "inf");
                        } else {
                            let upper = num(lower + step);
                            let label = if i == 0 {
                                format!("<= {upper}")
                            } else {
                                format!("{} - {}", num(lower), upper)
                            };
                            write_shader_item(w, color, &label, &upper);
                        }
                    }
                }
            }
            write_ramp_legend_settings(w);
            w.end(); // colorrampshader
            w.end(); // rastershader
            w.end(); // rasterrenderer
        }
        RasterStyle::MultibandColor(m) => {
            w.start("rasterrenderer")
                .attr("alphaBand", "-1")
                .attr("blueBand", m.blue.0)
                .attr("greenBand", m.green.0)
                .attr("nodataColor", "")
                .attr("opacity", "1")
                .attr("redBand", m.red.0)
                .attr("type", "multibandcolor");
            w.empty("rasterTransparency", &[]);
            write_min_max_origin(w, "MinMax");
            for (tag, (_, min, max)) in [
                ("redContrastEnhancement", m.red),
                ("greenContrastEnhancement", m.green),
                ("blueContrastEnhancement", m.blue),
            ] {
                w.start(tag);
                w.elem("minValue", &num(min));
                w.elem("maxValue", &num(max));
                w.elem("algorithm", "NoEnhancement");
                w.end(); // tag
            }
            w.end(); // rasterrenderer
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qgis_color_format() {
        assert_eq!(
            Rgb::new(232, 113, 141).qgis(),
            "232,113,141,255,rgb:0.9098039,0.4431373,0.5529412,1"
        );
        assert_eq!(
            Rgb::new(35, 35, 35).qgis(),
            "35,35,35,255,rgb:0.1372549,0.1372549,0.1372549,1"
        );
        assert_eq!(Rgb::new(255, 0, 0).qgis(), "255,0,0,255,rgb:1,0,0,1");
        assert_eq!(
            Rgb::new(255, 204, 204).qgis(),
            "255,204,204,255,rgb:1,0.8,0.8,1"
        );
    }

    #[test]
    fn ramp_interpolation_matches_sample() {
        // samples/red.qgs: white -> red in 6 classes
        let ramp = [(0.0, Rgb::new(255, 255, 255)), (1.0, Rgb::new(255, 0, 0))];
        let expected = [
            Rgb::new(255, 255, 255),
            Rgb::new(255, 204, 204),
            Rgb::new(255, 153, 153),
            Rgb::new(255, 102, 102),
            Rgb::new(255, 51, 51),
            Rgb::new(255, 0, 0),
        ];
        for (i, want) in expected.iter().enumerate() {
            assert_eq!(&sample_ramp(&ramp, i as f64 / 5.0), want);
        }
    }

    #[test]
    fn multi_stop_interpolation_matches_sample() {
        // Class colors in samples/magma.qgs (6 classes), and the magma
        // control points adjacent to the class positions.
        let ramp = [
            (0.0, Rgb::new(0, 0, 4)),
            (0.196078, Rgb::new(57, 15, 110)),
            (0.215686, Rgb::new(66, 15, 117)),
            (0.392157, Rgb::new(137, 40, 129)),
            (0.411765, Rgb::new(145, 43, 129)),
            (0.588235, Rgb::new(217, 70, 107)),
            (0.607843, Rgb::new(224, 76, 103)),
            (0.784314, Rgb::new(253, 152, 105)),
            (0.803922, Rgb::new(254, 161, 110)),
            (1.0, Rgb::new(252, 253, 191)),
        ];
        let expected = [
            Rgb::new(0, 0, 4),
            Rgb::new(59, 15, 111),
            Rgb::new(140, 41, 129),
            Rgb::new(221, 74, 105),
            Rgb::new(254, 159, 109),
            Rgb::new(252, 253, 191),
        ];
        for (i, want) in expected.iter().enumerate() {
            assert_eq!(&sample_ramp(&ramp, i as f64 / 5.0), want);
        }
    }

    #[test]
    fn offset_formatting() {
        assert_eq!(g6(0.0), "0");
        assert_eq!(g6(0.5), "0.5");
        assert_eq!(g6(1.0), "1");
        assert_eq!(g6(1.0 / 51.0), "0.0196078");
        assert_eq!(g6(0.509804), "0.509804");
        assert_eq!(g6(0.980392), "0.980392");
    }

    #[test]
    fn labels() {
        assert_eq!(range_label(0.0, 10.0), "0 - 10");
        assert_eq!(range_label(9.5, 19.0), "9.5 - 19");
    }

    #[test]
    fn categorized_renderer_structure() {
        let style = VectorStyle::categorized(
            "NAME",
            &[
                ("Alamance", Rgb::new(255, 255, 255)),
                ("Alexander", Rgb::new(255, 252, 252)),
            ],
            Some(Rgb::new(255, 0, 0)),
        )
        .unwrap();
        let mut w = XmlWriter::new(0);
        write_renderer(&mut w, GeometryType::Polygon, &style);
        let out = w.finish();

        assert!(out.contains("type=\"categorizedSymbol\""));
        assert!(out.contains("attr=\"NAME\""));
        // Categories reference symbols by index...
        assert!(
            out.contains(
                "<category label=\"Alamance\" render=\"true\" symbol=\"0\" type=\"string\""
            )
        );
        assert!(out.contains("value=\"Alamance\""));
        assert!(out.contains(
            "<category label=\"Alexander\" render=\"true\" symbol=\"1\" type=\"string\""
        ));
        // ...and the catch-all is a NULL category with the next index.
        assert!(out.contains("<category label=\"\" render=\"true\" symbol=\"2\" type=\"NULL\""));
        assert!(out.contains("value=\"NULL\""));
        // Symbols are named with the same ids, colors included.
        assert!(out.contains("name=\"0\" type=\"fill\""));
        assert!(out.contains("name=\"2\" type=\"fill\""));
        assert!(out.contains("255,255,255,255,rgb:1,1,1,1"));
        assert!(out.contains("255,252,252,255,rgb:1,0.9882353,0.9882353,1"));
        assert!(out.contains("255,0,0,255,rgb:1,0,0,1"));
    }

    #[test]
    fn continuous_renderer_structure() {
        let style = VectorStyle::continuous(
            "AREA",
            0.042,
            0.241,
            &[
                (0.0, Rgb::new(19, 43, 67)),
                (0.5, Rgb::new(45, 96, 141)),
                (1.0, Rgb::new(86, 177, 247)),
            ],
        )
        .unwrap();
        let mut w = XmlWriter::new(0);
        write_renderer(&mut w, GeometryType::Polygon, &style);
        let out = w.finish();

        assert!(out.contains("type=\"singleSymbol\""));
        // The fill color is driven by a data-defined expression...
        assert!(out.contains("<Option name=\"fillColor\" type=\"Map\">"));
        assert!(out.contains("name=\"active\" type=\"bool\" value=\"true\""));
        assert!(out.contains("name=\"type\" type=\"int\" value=\"3\""));
        // ...that interpolates the inline ramp over the rescaled attribute.
        let expr = "ramp_color(create_ramp(map(0,'#132b43',0.5,'#2d608d',1,'#56b1f7')),\
                    (&quot;AREA&quot; - 0.042) / (0.241 - 0.042))";
        assert!(out.contains(expr), "expression not found in:\n{out}");
        // The static color is the middle of the ramp.
        assert!(out.contains("45,96,141,255,rgb:"));
    }

    #[test]
    fn continuous_line_color_is_data_defined_stroke() {
        let style = VectorStyle::continuous(
            "x",
            0.0,
            1.0,
            &[(0.0, Rgb::new(0, 0, 0)), (1.0, Rgb::new(255, 255, 255))],
        )
        .unwrap();
        let mut w = XmlWriter::new(0);
        write_renderer(&mut w, GeometryType::LineString, &style);
        let out = w.finish();
        // SimpleLine's color is its stroke, so the override targets
        // outlineColor rather than fillColor.
        assert!(out.contains("<Option name=\"outlineColor\" type=\"Map\">"));
        assert!(!out.contains("<Option name=\"fillColor\""));
    }

    #[test]
    fn continuous_field_names_are_escaped() {
        assert_eq!(quote_field("AREA"), "\"AREA\"");
        assert_eq!(quote_field("odd\"name"), "\"odd\"\"name\"");
    }

    #[test]
    fn invalid_continuous_styles_are_errors() {
        let ramp = [(0.0, Rgb::new(0, 0, 0)), (1.0, Rgb::new(255, 255, 255))];
        assert!(matches!(
            VectorStyle::continuous("x", 1.0, 1.0, &ramp),
            Err(StyleError::InvalidRange { .. })
        ));
        assert!(matches!(
            VectorStyle::continuous("x", 0.0, 1.0, &ramp[..1]),
            Err(StyleError::TooFewColorStops(1))
        ));
    }

    #[test]
    fn invalid_styles_are_errors() {
        let ramp = [(0.0, Rgb::new(0, 0, 0)), (1.0, Rgb::new(255, 255, 255))];
        assert!(matches!(
            VectorStyle::graduated("x", 1, 0.0, 1.0, &ramp),
            Err(StyleError::TooFewClasses(1))
        ));
        assert!(matches!(
            VectorStyle::graduated("x", 2, 1.0, 1.0, &ramp),
            Err(StyleError::InvalidRange { .. })
        ));
        assert!(matches!(
            VectorStyle::graduated("x", 2, 0.0, 1.0, &ramp[..1]),
            Err(StyleError::TooFewColorStops(1))
        ));
        assert!(matches!(
            VectorStyle::graduated(
                "x",
                2,
                0.0,
                1.0,
                &[(0.1, Rgb::new(0, 0, 0)), (1.0, Rgb::new(255, 255, 255))]
            ),
            Err(StyleError::BadColorStopEndpoints)
        ));
        assert!(matches!(
            VectorStyle::graduated(
                "x",
                2,
                0.0,
                1.0,
                &[
                    (0.0, Rgb::new(0, 0, 0)),
                    (0.5, Rgb::new(255, 255, 255)),
                    (0.5, Rgb::new(0, 0, 0)),
                    (1.0, Rgb::new(255, 255, 255))
                ]
            ),
            Err(StyleError::NonAscendingColorStops)
        ));
        assert!(matches!(
            VectorStyle::categorized("x", &[] as &[(&str, Rgb)], None),
            Err(StyleError::NoCategories)
        ));
        assert!(matches!(
            RasterStyle::multiband((0, 0.0, 1.0), (1, 0.0, 1.0), (1, 0.0, 1.0)),
            Err(StyleError::InvalidBand(0))
        ));
        assert!(matches!(
            RasterStyle::multiband((1, 2.0, 1.0), (1, 0.0, 1.0), (1, 0.0, 1.0)),
            Err(StyleError::InvalidRange { .. })
        ));
    }

    /// The ramp of the elevation samples (`samples/elevation*.qgs`).
    const SPECTRAL: &[(f64, Rgb)] = &[
        (0.0, Rgb::new(215, 25, 28)),
        (0.25, Rgb::new(253, 174, 97)),
        (0.5, Rgb::new(255, 255, 191)),
        (0.75, Rgb::new(171, 221, 164)),
        (1.0, Rgb::new(43, 131, 186)),
    ];

    #[test]
    fn pseudocolor_interpolated_matches_sample() {
        // samples/elevation.qgs: 5 items at 80..200.
        let style = RasterStyle::pseudocolor(5, 80.0, 200.0, SPECTRAL).unwrap();
        let mut w = XmlWriter::new(0);
        write_raster_renderer(&mut w, &style);
        let out = w.finish();

        assert!(out.contains("type=\"singlebandpseudocolor\""));
        assert!(out.contains("classificationMax=\"200\" classificationMin=\"80\""));
        assert!(out.contains("colorRampType=\"INTERPOLATED\""));
        assert!(out.contains("classificationMode=\"1\""));
        assert!(out.contains("maximumValue=\"200\" minimumValue=\"80\""));
        for (color, value) in [
            ("#d7191c", "80"),
            ("#fdae61", "110"),
            ("#ffffbf", "140"),
            ("#abdda4", "170"),
            ("#2b83ba", "200"),
        ] {
            assert!(
                out.contains(&format!(
                    "<item alpha=\"255\" color=\"{color}\" label=\"{value}\" value=\"{value}\"/>"
                )),
                "missing item {value}"
            );
        }
    }

    #[test]
    fn pseudocolor_discrete_matches_sample() {
        // samples/elevation_discrete.qgs: 10 classes over 80..200.
        let style = RasterStyle::pseudocolor_discrete(10, 80.0, 200.0, SPECTRAL).unwrap();
        let mut w = XmlWriter::new(0);
        write_raster_renderer(&mut w, &style);
        let out = w.finish();

        assert!(out.contains("colorRampType=\"DISCRETE\""));
        assert!(out.contains("classificationMode=\"2\""));
        for (color, label, value) in [
            ("#d7191c", "&lt;= 92", "92"),
            ("#e85b3b", "92 - 104", "104"),
            ("#f99d59", "104 - 116", "116"),
            ("#fec980", "116 - 128", "128"),
            ("#ffedaa", "128 - 140", "140"),
            ("#ecf7b9", "140 - 152", "152"),
            ("#c7e8ad", "152 - 164", "164"),
            ("#9dd3a6", "164 - 176", "176"),
            ("#64abb0", "176 - 188", "188"),
            ("#2b83ba", "> 188", "inf"),
        ] {
            assert!(
                out.contains(&format!(
                    "<item alpha=\"255\" color=\"{color}\" label=\"{label}\" value=\"{value}\"/>"
                )),
                "missing item {label}"
            );
        }
    }

    #[test]
    fn multiband_matches_sample() {
        // samples/true-color.qgs.
        let style =
            RasterStyle::multiband((1, 35.0, 253.0), (2, 35.0, 251.0), (3, 35.0, 250.0)).unwrap();
        let mut w = XmlWriter::new(0);
        write_raster_renderer(&mut w, &style);
        let out = w.finish();

        assert!(out.contains("type=\"multibandcolor\""));
        assert!(out.contains("redBand=\"1\""));
        assert!(out.contains("greenBand=\"2\""));
        assert!(out.contains("blueBand=\"3\""));
        for (tag, max) in [
            ("redContrastEnhancement", "253"),
            ("greenContrastEnhancement", "251"),
            ("blueContrastEnhancement", "250"),
        ] {
            assert!(out.contains(&format!(
                "<{tag}>\n    <minValue>35</minValue>\n    <maxValue>{max}</maxValue>\n    <algorithm>NoEnhancement</algorithm>"
            )));
        }
    }
}
