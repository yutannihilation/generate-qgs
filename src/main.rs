//! Demo: regenerates projects equivalent to the files in `samples/` into
//! `out/`.

use generate_qgs::{GeometryType, QgsBuilder, Rgb, VectorStyle};

mod demo_data;

fn builder(style: VectorStyle) -> Result<QgsBuilder, generate_qgs::SrsError> {
    let mut b = QgsBuilder::new();
    b.add_xyz_tile_layer(
        "地理院タイル（標準地図）",
        "https://cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png",
        0,
        18,
    );
    // The sample `nc.gpkg` dataset is in NAD27 (EPSG:4267).
    b.add_vector_layer(
        "../tmp/nc.gpkg",
        "nc",
        4267,
        GeometryType::Polygon,
        style,
    )?;
    Ok(b)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("out")?;

    // Like samples/load.qgs: single-color vector layer.
    builder(VectorStyle::single(Rgb::new(232, 113, 141)))?.write_to("out/load.qgs")?;

    // Like samples/red.qgs: white -> red gradient over the SID79 attribute.
    builder(VectorStyle::graduated(
        "SID79",
        6,
        0.0,
        57.0,
        &[(0.0, Rgb::new(255, 255, 255)), (1.0, Rgb::new(255, 0, 0))],
    ))?
    .write_to("out/red.qgs")?;

    // Like samples/magma.qgs: the magma color ramp over the SID79 attribute.
    builder(VectorStyle::graduated(
        "SID79",
        6,
        0.0,
        57.0,
        demo_data::MAGMA_RAMP,
    ))?
    .write_to("out/magma.qgs")?;

    // Like samples/categorized.qgs: one color per NAME value.
    builder(VectorStyle::categorized(
        "NAME",
        demo_data::NC_COUNTIES,
        Some(Rgb::new(255, 0, 0)),
    ))?
    .write_to("out/categorized.qgs")?;

    println!("wrote out/load.qgs, out/red.qgs, out/magma.qgs and out/categorized.qgs");
    Ok(())
}
