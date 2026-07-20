//! Binding for `QgsBuilder`, the entry point of project generation.

use savvy::{Sexp, savvy, savvy_err};

use crate::parse_srs;
use crate::style::{GeometryType, RasterStyle, VectorStyle};

/// Builds a `.qgs` project file.
///
/// Layers are added bottom-most first: typically a base XYZ tile layer,
/// then vector layers on top of it. Finally, call `$build()` to get the
/// project content as a string, or `$write_to(path)` to write it to a file.
///
/// @export
#[savvy]
struct QgsBuilder {
    inner: generate_qgs::QgsBuilder,
}

#[savvy]
impl QgsBuilder {
    fn new() -> Self {
        Self {
            inner: generate_qgs::QgsBuilder::new(),
        }
    }

    /// Adds an XYZ tile layer (e.g. OpenStreetMap-like tiles). The `url`
    /// should contain `{z}`/`{x}`/`{y}` placeholders. XYZ tiles are always
    /// in EPSG:3857.
    fn add_xyz_tile_layer(
        &mut self,
        name: &str,
        url: &str,
        zmin: i32,
        zmax: i32,
    ) -> savvy::Result<()> {
        let to_zoom = |v: i32| {
            u8::try_from(v).map_err(|_| savvy_err!("zoom level must be between 0 and 255, got {v}"))
        };
        self.inner
            .add_xyz_tile_layer(name, url, to_zoom(zmin)?, to_zoom(zmax)?);
        Ok(())
    }

    /// Adds a GeoPackage vector layer. `srs` is either an EPSG code or a
    /// WKT2 string.
    fn add_vector_layer(
        &mut self,
        path: &str,
        layer_name: &str,
        srs: Sexp,
        geometry: &GeometryType,
        style: &VectorStyle,
    ) -> savvy::Result<()> {
        self.inner.add_vector_layer(
            path,
            layer_name,
            parse_srs(srs)?,
            geometry.into(),
            style.inner.clone(),
        )?;
        Ok(())
    }

    /// Adds a raster layer from a local file (e.g. GeoTIFF), loaded through
    /// the GDAL provider. `srs` is either an EPSG code or a WKT2 string.
    fn add_raster_layer(
        &mut self,
        path: &str,
        layer_name: &str,
        srs: Sexp,
        style: &RasterStyle,
    ) -> savvy::Result<()> {
        self.inner
            .add_raster_layer(path, layer_name, parse_srs(srs)?, style.inner.clone())?;
        Ok(())
    }

    /// Renders the project file content.
    fn build(&self) -> savvy::Result<Sexp> {
        self.inner.build().try_into()
    }

    /// Writes the project to a `.qgs` file.
    fn write_to(&self, path: &str) -> savvy::Result<()> {
        self.inner.write_to(path)?;
        Ok(())
    }
}
