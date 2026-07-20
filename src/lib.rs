//! Generate QGIS project files (`.qgs`) programmatically.
//!
//! Quick-and-dirty generator: the static boilerplate of a project (taken
//! from an empty project saved by QGIS) is used as a template, and only the
//! layer-related parts are generated. See `docs/qgs-format.md` for the
//! findings about the file format.
//!
//! # Example
//!
//! ```no_run
//! use generate_qgs::{GeometryType, QgsBuilder, Rgb, VectorStyle};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut b = QgsBuilder::new();
//! b.add_xyz_tile_layer(
//!     "地理院タイル（標準地図）",
//!     "https://cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png",
//!     0,
//!     18,
//! );
//! b.add_vector_layer(
//!     "../tmp/nc.gpkg",
//!     "nc",
//!     4326,
//!     GeometryType::Polygon,
//!     VectorStyle::single(Rgb::new(232, 113, 141)),
//! )?;
//! b.write_to("project.qgs")?;
//! # Ok(())
//! # }
//! ```

mod ids;
mod layers;
mod srs;
mod style;
mod time;
mod xml;

use std::io;
use std::path::Path;

pub use srs::{Srs, SrsError};
pub use style::{
    GeometryType, GraduatedStyle, MultibandColorStyle, PseudocolorMode, PseudocolorStyle,
    RasterStyle, Rgb, SimpleStyle, VectorStyle,
};

use layers::{Layer, RasterLayer, VectorLayer, XyzLayer};
use xml::XmlWriter;

/// The static project scaffold, adapted from `samples/blank.qgs`.
const TEMPLATE: &str = include_str!("template.qgs");

/// Builds a `.qgs` project file.
///
/// Layers are added bottom-most first: typically a base XYZ tile layer,
/// then vector layers on top of it.
#[derive(Default)]
pub struct QgsBuilder {
    layers: Vec<Layer>,
}

impl QgsBuilder {
    pub fn new() -> Self {
        QgsBuilder::default()
    }

    /// Adds an XYZ tile layer (e.g. OpenStreetMap-like tiles). The `url`
    /// should contain `{z}`/`{x}`/`{y}` placeholders. XYZ tiles are always
    /// in EPSG:3857.
    pub fn add_xyz_tile_layer(&mut self, name: &str, url: &str, zmin: u8, zmax: u8) -> &mut Self {
        let srs = Srs::Epsg(3857)
            .resolve()
            .expect("EPSG:3857 is always in the EPSG dataset");
        self.layers.push(Layer::Xyz(XyzLayer {
            id: ids::layer_id(name),
            name: name.to_string(),
            url: url.to_string(),
            zmin,
            zmax,
            srs,
        }));
        self
    }

    /// Adds a GeoPackage vector layer.
    ///
    /// * `path` — path to the `.gpkg` file, embedded verbatim into the
    ///   project. A relative path is resolved by QGIS against the location
    ///   of the saved `.qgs` file.
    /// * `layer_name` — table/layer name inside the `.gpkg`
    ///   (`<path>|layername=<layer_name>`); also used as the display name.
    /// * `srs` — SRS of the data (cannot be derived without reading the
    ///   file, so it must be stated): either an EPSG code or a WKT2 string;
    ///   see [`Srs`].
    /// * `geometry` — geometry kind of the layer.
    /// * `style` — how features are rendered, see [`VectorStyle`].
    ///
    /// Returns an error if the SRS cannot be resolved.
    pub fn add_vector_layer(
        &mut self,
        path: &str,
        layer_name: &str,
        srs: impl Into<Srs>,
        geometry: GeometryType,
        style: VectorStyle,
    ) -> Result<&mut Self, SrsError> {
        self.layers.push(Layer::Vector(VectorLayer {
            id: ids::layer_id(layer_name),
            name: layer_name.to_string(),
            path: path.to_string(),
            srs: srs.into().resolve()?,
            geometry,
            style,
        }));
        Ok(self)
    }

    /// Adds a raster layer from a local file (e.g. GeoTIFF), loaded through
    /// the GDAL provider.
    ///
    /// * `path` — path to the raster file, embedded verbatim into the
    ///   project. A relative path is resolved by QGIS against the location
    ///   of the saved `.qgs` file.
    /// * `layer_name` — display name of the layer.
    /// * `srs` — SRS of the data (cannot be derived without reading the
    ///   file, so it must be stated): either an EPSG code or a WKT2 string;
    ///   see [`Srs`].
    /// * `style` — how the raster is rendered, see [`RasterStyle`].
    ///
    /// Returns an error if the SRS cannot be resolved.
    pub fn add_raster_layer(
        &mut self,
        path: &str,
        layer_name: &str,
        srs: impl Into<Srs>,
        style: RasterStyle,
    ) -> Result<&mut Self, SrsError> {
        self.layers.push(Layer::Raster(RasterLayer {
            id: ids::layer_id(layer_name),
            name: layer_name.to_string(),
            path: path.to_string(),
            srs: srs.into().resolve()?,
            style,
        }));
        Ok(self)
    }

    /// Renders the project file content.
    pub fn build(&self) -> String {
        let mut out = TEMPLATE.replace("{{SAVE_DATETIME}}", &time::now_iso8601());
        if self.layers.is_empty() {
            return out;
        }

        // Layer tree and legend: top-most layer first, i.e. the reverse of
        // the insertion order.
        let mut tree = XmlWriter::new(2);
        let mut legend = XmlWriter::new(2);
        for layer in self.layers.iter().rev() {
            layer.write_layer_tree_layer(&mut tree);
            layer.write_legend_layer(&mut legend);
        }
        // Drawing order (<custom-order>, <layerorder>): bottom-most first.
        let mut order = XmlWriter::new(3);
        let mut layerorder = XmlWriter::new(2);
        let mut projectlayers = XmlWriter::new(2);
        for layer in &self.layers {
            order.elem("item", layer.id());
            layerorder.start("layer").attr("id", layer.id()).end();
            layer.write_maplayer(&mut projectlayers);
        }

        // The replacement anchors include their leading indentation so the
        // spliced fragments stay flush with the template.
        out = out.replace(
            "\n    <custom-order enabled=\"0\"/>",
            &format!(
                "{}\n    <custom-order enabled=\"0\">{}\n    </custom-order>",
                tree.finish(),
                order.finish()
            ),
        );
        out = out.replace(
            "\n  <legend updateDrawingOrder=\"true\"/>",
            &format!(
                "\n  <legend updateDrawingOrder=\"true\">{}\n  </legend>",
                legend.finish()
            ),
        );
        out = out.replace(
            "\n  <projectlayers/>",
            &format!(
                "\n  <projectlayers>{}\n  </projectlayers>",
                projectlayers.finish()
            ),
        );
        out = out.replace(
            "\n  <layerorder/>",
            &format!("\n  <layerorder>{}\n  </layerorder>", layerorder.finish()),
        );
        out
    }

    /// Writes the project to a `.qgs` file.
    pub fn write_to(&self, path: impl AsRef<Path>) -> io::Result<()> {
        std::fs::write(path, self.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_builder() -> QgsBuilder {
        let mut b = QgsBuilder::new();
        b.add_xyz_tile_layer(
            "地理院タイル（標準地図）",
            "https://cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png",
            0,
            18,
        );
        b.add_vector_layer(
            "../tmp/nc.gpkg",
            "nc",
            4267,
            GeometryType::Polygon,
            VectorStyle::graduated(
                "SID79",
                6,
                0.0,
                57.0,
                &[(0.0, Rgb::new(255, 255, 255)), (1.0, Rgb::new(255, 0, 0))],
            ),
        )
        .unwrap();
        b
    }

    #[test]
    fn empty_project_keeps_template_anchors() {
        let out = QgsBuilder::new().build();
        assert!(out.contains("<custom-order enabled=\"0\"/>"));
        assert!(out.contains("<legend updateDrawingOrder=\"true\"/>"));
        assert!(out.contains("<projectlayers/>"));
        assert!(out.contains("<layerorder/>"));
        assert!(!out.contains("{{SAVE_DATETIME}}"));
    }

    #[test]
    fn layers_appear_in_all_four_places() {
        let out = sample_builder().build();
        assert!(out.contains("<layer-tree-layer"));
        assert!(out.contains("<legendlayer"));
        assert!(out.contains("<maplayer"));
        assert!(out.contains("<layer id="));
        assert!(out.contains("type=\"graduatedSymbol\""));
        assert!(out.contains("attr=\"SID79\""));
        assert!(out.contains("../tmp/nc.gpkg|layername=nc"));
        assert!(out.contains("type=xyz"));
    }

    #[test]
    fn special_characters_are_escaped() {
        let mut b = QgsBuilder::new();
        b.add_vector_layer(
            "a&b<.gpkg",
            "x\"y",
            4326,
            GeometryType::Point,
            VectorStyle::single(Rgb::new(0, 0, 0)),
        )
        .unwrap();
        let out = b.build();
        assert!(out.contains("a&amp;b&lt;.gpkg"));
        assert!(out.contains("x&quot;y"));
    }
}
