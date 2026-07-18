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

// Or a color gradient driven by an attribute. The gradient is a list of
// (offset, color) stops with offsets in 0..=1; a plain two-color ramp is
// just [(0.0, from), (1.0, to)]:
b.add_vector_layer(
    "../tmp/nc.gpkg",
    "nc2",
    4267,
    GeometryType::Polygon,
    VectorStyle::graduated(
        "SID79",
        6,
        0.0,
        57.0,
        &[
            (0.0, Rgb::new(0, 0, 4)),
            (0.5, Rgb::new(180, 54, 122)),
            (1.0, Rgb::new(252, 253, 191)),
        ],
    ),
)?;

// Or a discrete color per attribute value ("categorized" in QGIS), with an
// optional catch-all color for all other values:
b.add_vector_layer(
    "../tmp/nc.gpkg",
    "nc3",
    4267,
    GeometryType::Polygon,
    VectorStyle::categorized(
        "NAME",
        &[("Alamance", Rgb::new(255, 255, 255)), ("Alexander", Rgb::new(255, 252, 252))],
        Some(Rgb::new(255, 0, 0)),
    ),
)?;

b.write_to("project.qgs")?;
```

## Demo

`cargo run` writes projects equivalent to the samples into `out/`:

- `out/load.qgs` — XYZ tile layer + single-color vector layer
- `out/red.qgs` — XYZ tile layer + graduated-color vector layer (two-color ramp)
- `out/magma.qgs` — XYZ tile layer + graduated-color vector layer (magma ramp with many color stops)
- `out/categorized.qgs` — XYZ tile layer + categorized vector layer (discrete color per value)

## Sample .qgs files

- `blank.qgs`: a QGIS project without any data sources or layers
- `load.qgs`: a QGIS project with two layers: XYZ tile and .gpkg (single color)
- `red.qgs`: a QGIS project with two layers: XYZ tile and .gpkg (gradient based on attribute value)
- `magma.qgs`: a QGIS project with two layers: XYZ tile and .gpkg (magma color ramp based on attribute value)
- `categorized.qgs`: a QGIS project with two layers: XYZ tile and .gpkg (discrete color per attribute value)
