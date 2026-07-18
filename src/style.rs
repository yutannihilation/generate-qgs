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

    /// Linear interpolation in RGB space (matches QGIS color ramps).
    fn lerp(self, other: Rgb, t: f64) -> Rgb {
        let f = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
        Rgb::new(f(self.r, other.r), f(self.g, other.g), f(self.b, other.b))
    }
}

/// Formats one channel as a float in `0..=1` the way QGIS does (`%.7g`-ish).
fn channel(v: u8) -> String {
    let s = format!("{:.7}", f64::from(v) / 255.0);
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
    pub fn categorized(
        attribute: impl Into<String>,
        categories: &[(impl AsRef<str>, Rgb)],
        catch_all: Option<Rgb>,
    ) -> Self {
        assert!(
            !categories.is_empty(),
            "categorized style needs at least 1 category"
        );
        VectorStyle::Categorized(CategorizedStyle {
            attribute: attribute.into(),
            categories: categories
                .iter()
                .map(|(value, color)| (value.as_ref().to_string(), *color))
                .collect(),
            catch_all,
            outline_color: Rgb::new(35, 35, 35),
            outline_width: 0.26,
        })
    }

    /// Graduated coloring of `attribute` with `classes` equal-interval
    /// ranges between `min` and `max`. Class colors are interpolated along
    /// `color_stops`, a list of `(offset, color)` control points with
    /// offsets in `0..=1`: the first entry must be at `0.0`, the last at
    /// `1.0`. A plain two-color ramp is `&[(0.0, from), (1.0, to)]`.
    pub fn graduated(
        attribute: impl Into<String>,
        classes: usize,
        min: f64,
        max: f64,
        color_stops: &[(f64, Rgb)],
    ) -> Self {
        assert!(classes >= 2, "graduated style needs at least 2 classes");
        assert!(min < max, "graduated style needs min < max");
        assert!(
            color_stops.len() >= 2,
            "graduated style needs at least 2 color stops"
        );
        assert!(
            color_stops.first().unwrap().0 == 0.0,
            "first color stop must be at offset 0.0"
        );
        assert!(
            color_stops.last().unwrap().0 == 1.0,
            "last color stop must be at offset 1.0"
        );
        assert!(
            color_stops.windows(2).all(|w| w[0].0 < w[1].0),
            "color stops must be in ascending offset order"
        );
        VectorStyle::Graduated(GraduatedStyle {
            attribute: attribute.into(),
            classes,
            min,
            max,
            color_stops: color_stops.to_vec(),
            outline_color: Rgb::new(35, 35, 35),
            outline_width: 0.26,
        })
    }
}

/// The recurring `<(data[-_]defined[-_]properties)>` boilerplate.
fn write_data_defined_properties(w: &mut XmlWriter, tag: &str) {
    w.start(tag);
    w.start("Option").attr("type", "Map");
    w.empty("Option", &[("name", "name"), ("type", "QString"), ("value", "")]);
    w.empty("Option", &[("name", "properties")]);
    w.empty(
        "Option",
        &[("name", "type"), ("type", "QString"), ("value", "collection")],
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
        return format!("{}e{}{:02}", mantissa, if e < 0 { '-' } else { '+' }, e.abs());
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
    write_data_defined_properties(w, "data_defined_properties");
    w.end(); // layer
    w.end(); // symbol
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
            let last_color = c.catch_all.unwrap_or(c.categories[c.categories.len() - 1].1);
            write_gradient_colorramp(w, c.categories[0].1, last_color, &[]);
            w.empty("rotation", &[]);
            w.empty("sizescale", &[]);
            write_data_defined_properties(w, "data-defined-properties");
            w.end(); // renderer-v2
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
        );
        let mut w = XmlWriter::new(0);
        write_renderer(&mut w, GeometryType::Polygon, &style);
        let out = w.finish();

        assert!(out.contains("type=\"categorizedSymbol\""));
        assert!(out.contains("attr=\"NAME\""));
        // Categories reference symbols by index...
        assert!(out.contains(
            "<category label=\"Alamance\" render=\"true\" symbol=\"0\" type=\"string\""
        ));
        assert!(out.contains("value=\"Alamance\""));
        assert!(out.contains(
            "<category label=\"Alexander\" render=\"true\" symbol=\"1\" type=\"string\""
        ));
        // ...and the catch-all is a NULL category with the next index.
        assert!(out.contains(
            "<category label=\"\" render=\"true\" symbol=\"2\" type=\"NULL\""
        ));
        assert!(out.contains("value=\"NULL\""));
        // Symbols are named with the same ids, colors included.
        assert!(out.contains("name=\"0\" type=\"fill\""));
        assert!(out.contains("name=\"2\" type=\"fill\""));
        assert!(out.contains("255,255,255,255,rgb:1,1,1,1"));
        assert!(out.contains("255,252,252,255,rgb:1,0.9882353,0.9882353,1"));
        assert!(out.contains("255,0,0,255,rgb:1,0,0,1"));
    }
}
