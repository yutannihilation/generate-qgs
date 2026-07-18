//! Demo: regenerates projects equivalent to `samples/load.qgs` and
//! `samples/red.qgs` into `out/`.

use generate_qgs::{GeometryType, QgsBuilder, Rgb, VectorStyle};

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
        Rgb::new(255, 255, 255),
        Rgb::new(255, 0, 0),
    ))?
    .write_to("out/red.qgs")?;

    println!("wrote out/load.qgs and out/red.qgs");
    Ok(())
}
