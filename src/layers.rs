//! Layer definitions and their XML representation.
//!
//! Each layer is referenced from four places in the project file
//! (see `docs/qgs-format.md`); this module renders all of them.

use crate::srs::{ResolvedSrs, write_spatialrefsys};
use crate::style::{
    GeometryType, RasterStyle, VectorStyle, write_min_max_origin, write_raster_renderer,
    write_renderer,
};
use crate::xml::XmlWriter;

pub(crate) struct XyzLayer {
    pub id: String,
    pub name: String,
    pub url: String,
    pub zmin: u8,
    pub zmax: u8,
    pub srs: ResolvedSrs,
}

pub(crate) struct VectorLayer {
    pub id: String,
    pub name: String,
    pub path: String,
    pub srs: ResolvedSrs,
    pub geometry: GeometryType,
    pub style: VectorStyle,
}

pub(crate) struct RasterLayer {
    pub id: String,
    pub name: String,
    pub path: String,
    pub srs: ResolvedSrs,
    pub style: RasterStyle,
}

pub(crate) enum Layer {
    Xyz(XyzLayer),
    Vector(VectorLayer),
    Raster(RasterLayer),
}

/// Percent-encodes everything except unreserved characters (RFC 3986),
/// matching how QGIS writes the `url` parameter of an XYZ datasource
/// (`{z}` -> `%7Bz%7D`, ...).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl XyzLayer {
    /// The WMS-provider datasource URI for XYZ tiles, e.g.
    /// `crs=EPSG%3A3857&format&type=xyz&url=...&zmax=18&zmin=0&http-header:referer=`.
    fn datasource(&self) -> String {
        let crs = match self.srs.epsg {
            Some(code) => format!("crs=EPSG%3A{code}&"),
            None => String::new(),
        };
        format!(
            "{crs}format&type=xyz&url={}&zmax={}&zmin={}&http-header:referer=",
            percent_encode(&self.url),
            self.zmax,
            self.zmin
        )
    }
}

impl VectorLayer {
    /// The ogr-provider datasource: `<path>|layername=<table>`.
    fn datasource(&self) -> String {
        format!("{}|layername={}", self.path, self.name)
    }
}

impl Layer {
    pub(crate) fn id(&self) -> &str {
        match self {
            Layer::Xyz(l) => &l.id,
            Layer::Vector(l) => &l.id,
            Layer::Raster(l) => &l.id,
        }
    }

    fn name(&self) -> &str {
        match self {
            Layer::Xyz(l) => &l.name,
            Layer::Vector(l) => &l.name,
            Layer::Raster(l) => &l.name,
        }
    }

    fn provider_key(&self) -> &'static str {
        match self {
            Layer::Xyz(_) => "wms",
            Layer::Vector(_) => "ogr",
            Layer::Raster(_) => "gdal",
        }
    }

    fn datasource(&self) -> String {
        match self {
            Layer::Xyz(l) => l.datasource(),
            Layer::Vector(l) => l.datasource(),
            // The gdal-provider datasource is the plain file path.
            Layer::Raster(l) => l.path.clone(),
        }
    }

    /// `<layer-tree-group>` entry.
    pub(crate) fn write_layer_tree_layer(&self, w: &mut XmlWriter) {
        w.start("layer-tree-layer")
            .attr("checked", "Qt::Checked")
            .attr("expanded", "1")
            .attr("id", self.id())
            .attr("legend_exp", "")
            .attr("legend_split_behavior", "0")
            .attr("name", self.name())
            .attr("patch_size", "-1,-1")
            .attr("providerKey", self.provider_key())
            .attr("source", self.datasource());
        w.start("customproperties");
        w.empty("Option", &[]);
        w.end(); // customproperties
        w.end(); // layer-tree-layer
    }

    /// `<legend>` entry.
    pub(crate) fn write_legend_layer(&self, w: &mut XmlWriter) {
        w.start("legendlayer")
            .attr("checked", "Qt::Checked")
            .attr("drawingOrder", "-1")
            .attr("name", self.name())
            .attr("open", "true")
            .attr("showFeatureCount", "0");
        w.start("filegroup")
            .attr("hidden", "false")
            .attr("open", "true");
        w.empty(
            "legendlayerfile",
            &[
                ("isInOverview", "0"),
                ("layerid", self.id()),
                ("visible", "1"),
            ],
        );
        w.end(); // filegroup
        w.end(); // legendlayer
    }

    /// `<projectlayers>` entry.
    pub(crate) fn write_maplayer(&self, w: &mut XmlWriter) {
        match self {
            Layer::Xyz(l) => write_xyz_maplayer(w, l),
            Layer::Vector(l) => write_vector_maplayer(w, l),
            Layer::Raster(l) => write_raster_maplayer(w, l),
        }
    }
}

/// The `<flags>` block shared by all layers.
fn write_flags(w: &mut XmlWriter) {
    w.start("flags");
    w.elem("Identifiable", "1");
    w.elem("Removable", "1");
    w.elem("Searchable", "1");
    w.elem("Private", "0");
    w.end();
}

/// `<extent>` with the four corners as child elements.
fn write_extent(w: &mut XmlWriter, tag: &str, corners: [&str; 4]) {
    w.start(tag);
    w.elem("xmin", corners[0]);
    w.elem("ymin", corners[1]);
    w.elem("xmax", corners[2]);
    w.elem("ymax", corners[3]);
    w.end();
}

/// The `<pipe><provider><resampling .../></provider>` part of a raster
/// maplayer — identical for XYZ tiles and GDAL layers.
fn write_pipe_provider(w: &mut XmlWriter) {
    w.start("provider");
    w.empty(
        "resampling",
        &[
            ("enabled", "false"),
            ("maxOversampling", "2"),
            ("zoomedInResamplingMethod", "nearestNeighbour"),
            ("zoomedOutResamplingMethod", "nearestNeighbour"),
        ],
    );
    w.end(); // provider
}

/// The `<pipe>` tail after the renderer — identical for XYZ tiles and
/// GDAL layers.
fn write_pipe_tail(w: &mut XmlWriter) {
    w.empty(
        "brightnesscontrast",
        &[("brightness", "0"), ("contrast", "0"), ("gamma", "1")],
    );
    w.empty(
        "huesaturation",
        &[
            ("colorizeBlue", "128"),
            ("colorizeGreen", "128"),
            ("colorizeOn", "0"),
            ("colorizeRed", "255"),
            ("colorizeStrength", "100"),
            ("grayscaleMode", "0"),
            ("invertColors", "0"),
            ("saturation", "0"),
        ],
    );
    w.empty("rasterresampler", &[("maxOversampling", "2")]);
    w.elem("resamplingStage", "resamplingFilter");
}

fn write_xyz_maplayer(w: &mut XmlWriter, layer: &XyzLayer) {
    w.start("maplayer")
        .attr("autoRefreshMode", "Disabled")
        .attr("autoRefreshTime", "0")
        .attr("hasScaleBasedVisibilityFlag", "0")
        .attr("layerType", "Raster")
        .attr("legendPlaceholderImage", "")
        .attr("maxScale", "0")
        .attr("minScale", "1e+08")
        .attr("refreshOnNotifyEnabled", "0")
        .attr("refreshOnNotifyMessage", "")
        .attr("styleCategories", "AllStyleCategories")
        .attr("type", "raster");
    // XYZ tiles cover the whole EPSG:3857 world.
    write_extent(
        w,
        "extent",
        [
            "-20037508.34278924390673637",
            "-20037508.34278924763202667",
            "20037508.34278924390673637",
            "20037508.34278924763202667",
        ],
    );
    write_extent(
        w,
        "wgs84extent",
        [
            "-180",
            "-85.05112877980660357",
            "180",
            "85.05112877980660357",
        ],
    );
    w.elem("id", &layer.id);
    w.elem("datasource", &layer.datasource());
    w.elem("layername", &layer.name);
    w.start("srs");
    write_spatialrefsys(w, &layer.srs);
    w.end(); // srs
    w.elem("provider", "wms");
    w.start("noData");
    w.empty("noDataList", &[("bandNo", "1"), ("useSrcNoData", "0")]);
    w.end(); // noData
    write_flags(w);
    w.start("customproperties");
    w.start("Option").attr("type", "Map");
    w.empty(
        "Option",
        &[
            ("name", "identify/format"),
            ("type", "QString"),
            ("value", "Undefined"),
        ],
    );
    w.end(); // Option
    w.end(); // customproperties
    w.start("pipe");
    write_pipe_provider(w);
    w.start("rasterrenderer")
        .attr("alphaBand", "-1")
        .attr("band", "1")
        .attr("nodataColor", "")
        .attr("opacity", "1")
        .attr("type", "singlebandcolordata");
    w.empty("rasterTransparency", &[]);
    write_min_max_origin(w, "None");
    w.end(); // rasterrenderer
    write_pipe_tail(w);
    w.end(); // pipe
    w.elem("blendMode", "0");
    w.empty("legend", &[]);
    w.end(); // maplayer
}

/// GDAL raster layer (`samples/elevation.qgs`, `samples/true-color.qgs`).
/// `<extent>`/`<wgs84extent>` and the metadata boilerplate
/// (`<resourceMetadata>`, `<temporal>`, `<elevation>`, ...) are omitted:
/// QGIS recomputes them from the data source on load, like it does for
/// vector layers.
fn write_raster_maplayer(w: &mut XmlWriter, layer: &RasterLayer) {
    w.start("maplayer")
        .attr("autoRefreshMode", "Disabled")
        .attr("autoRefreshTime", "0")
        .attr("hasScaleBasedVisibilityFlag", "0")
        .attr("layerType", "Raster")
        .attr("legendPlaceholderImage", "")
        .attr("maxScale", "0")
        .attr("minScale", "1e+08")
        .attr("refreshOnNotifyEnabled", "0")
        .attr("refreshOnNotifyMessage", "")
        .attr("styleCategories", "AllStyleCategories")
        .attr("type", "raster");
    w.elem("id", &layer.id);
    w.elem("datasource", &layer.path);
    w.elem("layername", &layer.name);
    w.start("srs");
    write_spatialrefsys(w, &layer.srs);
    w.end(); // srs
    w.elem("provider", "gdal");
    w.start("noData");
    for band in 1..=layer.style.band_count() {
        w.empty(
            "noDataList",
            &[("bandNo", &band.to_string()), ("useSrcNoData", "1")],
        );
    }
    w.end(); // noData
    write_flags(w);
    w.start("pipe");
    write_pipe_provider(w);
    write_raster_renderer(w, &layer.style);
    write_pipe_tail(w);
    w.end(); // pipe
    w.elem("blendMode", "0");
    w.empty("legend", &[]);
    w.end(); // maplayer
}

fn write_vector_maplayer(w: &mut XmlWriter, layer: &VectorLayer) {
    w.start("maplayer")
        .attr("autoRefreshMode", "Disabled")
        .attr("autoRefreshTime", "0")
        .attr("geometry", layer.geometry.geometry_attr())
        .attr("hasScaleBasedVisibilityFlag", "0")
        .attr("labelsEnabled", "0")
        .attr("layerType", "Vector")
        .attr("legendPlaceholderImage", "")
        .attr("maxScale", "0")
        .attr("minScale", "100000000")
        .attr("readOnly", "0")
        .attr("refreshOnNotifyEnabled", "0")
        .attr("refreshOnNotifyMessage", "")
        .attr("simplifyAlgorithm", "0")
        .attr("simplifyDrawingHints", "1")
        .attr("simplifyDrawingTol", "1")
        .attr("simplifyLocal", "1")
        .attr("simplifyMaxScale", "1")
        .attr("styleCategories", "AllStyleCategories")
        .attr("symbologyReferenceScale", "-1")
        .attr("type", "vector")
        .attr("wkbType", layer.geometry.wkb_type_attr());
    w.elem("id", &layer.id);
    w.elem("datasource", &layer.datasource());
    w.elem("layername", &layer.name);
    w.start("srs");
    write_spatialrefsys(w, &layer.srs);
    w.end(); // srs
    w.start("provider")
        .attr("encoding", "UTF-8")
        .text("ogr")
        .end();
    w.empty("vectorjoins", &[]);
    w.empty("layerDependencies", &[]);
    w.empty("dataDependencies", &[]);
    w.empty("expressionfields", &[]);
    write_flags(w);
    write_renderer(w, layer.geometry, &layer.style);
    w.start("selection").attr("mode", "Default");
    w.empty("selectionColor", &[("invalid", "1")]);
    w.end(); // selection
    w.start("customproperties");
    w.empty("Option", &[]);
    w.end(); // customproperties
    w.elem("blendMode", "0");
    w.elem("featureBlendMode", "0");
    w.elem("layerOpacity", "1");
    w.start("geometryOptions")
        .attr("geometryPrecision", "0")
        .attr("removeDuplicateNodes", "0");
    w.start("activeChecks").attr("type", "StringList");
    w.empty("Option", &[("type", "QString"), ("value", "")]);
    w.end(); // activeChecks
    w.empty("checkConfiguration", &[]);
    w.end(); // geometryOptions
    w.empty(
        "legend",
        &[("showLabelLegend", "0"), ("type", "default-vector")],
    );
    w.empty("referencedLayers", &[]);
    w.empty("referencingLayers", &[]);
    w.end(); // maplayer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encoding() {
        assert_eq!(
            percent_encode("https://example.com/{z}/{x}/{y}.png"),
            "https%3A%2F%2Fexample.com%2F%7Bz%7D%2F%7Bx%7D%2F%7By%7D.png"
        );
    }

    #[test]
    fn xyz_datasource_matches_sample() {
        let layer = XyzLayer {
            id: "_x".into(),
            name: "地理院タイル（標準地図）".into(),
            url: "https://cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png".into(),
            zmin: 0,
            zmax: 18,
            srs: crate::Srs::Epsg(3857).resolve().unwrap(),
        };
        assert_eq!(
            layer.datasource(),
            "crs=EPSG%3A3857&format&type=xyz&url=https%3A%2F%2Fcyberjapandata.gsi.go.jp%2Fxyz%2Fstd%2F%7Bz%7D%2F%7Bx%7D%2F%7By%7D.png&zmax=18&zmin=0&http-header:referer="
        );
    }
}
