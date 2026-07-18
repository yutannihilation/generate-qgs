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
/// substitute for QGIS's "Pretty" classification); colors are interpolated
/// from `ramp_start` to `ramp_end`.
#[derive(Debug, Clone)]
pub struct GraduatedStyle {
    /// Attribute (field) name to classify on.
    pub attribute: String,
    /// Number of classes (>= 2).
    pub classes: usize,
    pub min: f64,
    pub max: f64,
    pub ramp_start: Rgb,
    pub ramp_end: Rgb,
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

    /// Graduated coloring of `attribute` with `classes` equal-interval
    /// ranges between `min` and `max`, colors interpolated from
    /// `ramp_start` to `ramp_end`.
    pub fn graduated(
        attribute: impl Into<String>,
        classes: usize,
        min: f64,
        max: f64,
        ramp_start: Rgb,
        ramp_end: Rgb,
    ) -> Self {
        assert!(classes >= 2, "graduated style needs at least 2 classes");
        assert!(min < max, "graduated style needs min < max");
        VectorStyle::Graduated(GraduatedStyle {
            attribute: attribute.into(),
            classes,
            min,
            max,
            ramp_start,
            ramp_end,
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
                let color = g.ramp_start.lerp(g.ramp_end, t);
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
            write_symbol(w, "0", geom, g.ramp_start, g.outline_color, g.outline_width);
            w.end(); // source-symbol
            w.start("colorramp")
                .attr("name", "[source]")
                .attr("type", "gradient");
            w.start("Option").attr("type", "Map");
            for (name, value) in [
                ("color1", g.ramp_start.qgis()),
                ("color2", g.ramp_end.qgis()),
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
            w.end(); // Option
            w.end(); // colorramp
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
        let white = Rgb::new(255, 255, 255);
        let red = Rgb::new(255, 0, 0);
        let expected = [
            Rgb::new(255, 255, 255),
            Rgb::new(255, 204, 204),
            Rgb::new(255, 153, 153),
            Rgb::new(255, 102, 102),
            Rgb::new(255, 51, 51),
            Rgb::new(255, 0, 0),
        ];
        for (i, want) in expected.iter().enumerate() {
            assert_eq!(&white.lerp(red, i as f64 / 5.0), want);
        }
    }

    #[test]
    fn labels() {
        assert_eq!(range_label(0.0, 10.0), "0 - 10");
        assert_eq!(range_label(9.5, 19.0), "9.5 - 19");
    }
}
