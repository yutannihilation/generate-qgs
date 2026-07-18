Generate .qgs XML file
======================

A quick-and-dirty Rust crate for generating QGIS project (`.qgs`) files
programmatically. See `docs/qgs-and-qgz.md` for notes on the file format.

## Usage

```rust
use generate_qgs::{GeometryType, QgsBuilder, Rgb, VectorStyle};

let mut b = QgsBuilder::new();

// Layers are added bottom-most first. XYZ tiles are always EPSG:3857.
b.add_xyz_tile_layer(
    "地理院タイル（標準地図）",
    "https://cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png",
    0,
    18,
);

// The SRS of a vector layer is specified by an EPSG code or a WKT2 string
// (converted with the epsg-utils crate). Single color:
b.add_vector_layer(
    "../tmp/nc.gpkg",   // path (relative paths resolve against the .qgs location)
    "nc",               // layer (table) name inside the .gpkg
    4267,               // SRS: NAD27 as EPSG code; Srs::Wkt("GEOGCRS[...]") works too
    GeometryType::Polygon,
    VectorStyle::single(Rgb::new(232, 113, 141)),
)?;

// Or a color gradient driven by an attribute:
b.add_vector_layer(
    "../tmp/nc.gpkg",
    "nc2",
    4267,
    GeometryType::Polygon,
    VectorStyle::graduated("SID79", 6, 0.0, 57.0, Rgb::new(255, 255, 255), Rgb::new(255, 0, 0)),
)?;

b.write_to("project.qgs")?;
```

## Demo

`cargo run` writes two projects equivalent to the samples into `out/`:

- `out/load.qgs` — XYZ tile layer + single-color vector layer
- `out/red.qgs` — XYZ tile layer + graduated-color vector layer

## Sample .qgs files

- `blank.qgs`: a QGIS project without any data sources or layers
- `load.qgs`: a QGIS project with two layers: XYZ tile and .gpkg (single color)
- `red.qgs`: a QGIS project with two layers: XYZ tile and .gpkg (gradient based on attribute value)
