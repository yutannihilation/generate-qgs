//! Demo: regenerates projects equivalent to `samples/load.qgs`,
//! `samples/red.qgs` and `samples/magma.qgs` into `out/`.

use generate_qgs::{GeometryType, QgsBuilder, Rgb, VectorStyle};

/// The magma color ramp: 52 control points extracted from samples/magma.qgs.
const MAGMA_RAMP: &[(f64, Rgb)] = &[
    (0.0, Rgb::new(0, 0, 4)),
    (0.0196078, Rgb::new(2, 2, 11)),
    (0.0392157, Rgb::new(5, 4, 22)),
    (0.0588235, Rgb::new(9, 7, 32)),
    (0.0784314, Rgb::new(14, 11, 43)),
    (0.0980392, Rgb::new(20, 14, 54)),
    (0.117647, Rgb::new(26, 16, 66)),
    (0.137255, Rgb::new(33, 17, 78)),
    (0.156863, Rgb::new(41, 17, 90)),
    (0.176471, Rgb::new(49, 17, 101)),
    (0.196078, Rgb::new(57, 15, 110)),
    (0.215686, Rgb::new(66, 15, 117)),
    (0.235294, Rgb::new(74, 16, 121)),
    (0.254902, Rgb::new(82, 19, 124)),
    (0.27451, Rgb::new(90, 22, 126)),
    (0.294118, Rgb::new(98, 25, 128)),
    (0.313725, Rgb::new(106, 28, 129)),
    (0.333333, Rgb::new(114, 31, 129)),
    (0.352941, Rgb::new(121, 34, 130)),
    (0.372549, Rgb::new(129, 37, 129)),
    (0.392157, Rgb::new(137, 40, 129)),
    (0.411765, Rgb::new(145, 43, 129)),
    (0.431373, Rgb::new(153, 45, 128)),
    (0.45098, Rgb::new(161, 48, 126)),
    (0.470588, Rgb::new(170, 51, 125)),
    (0.490196, Rgb::new(178, 53, 123)),
    (0.509804, Rgb::new(186, 56, 120)),
    (0.529412, Rgb::new(194, 59, 117)),
    (0.54902, Rgb::new(202, 62, 114)),
    (0.568627, Rgb::new(210, 66, 111)),
    (0.588235, Rgb::new(217, 70, 107)),
    (0.607843, Rgb::new(224, 76, 103)),
    (0.627451, Rgb::new(231, 82, 99)),
    (0.647059, Rgb::new(236, 88, 96)),
    (0.666667, Rgb::new(241, 96, 93)),
    (0.686275, Rgb::new(244, 105, 92)),
    (0.705882, Rgb::new(247, 114, 92)),
    (0.72549, Rgb::new(249, 123, 93)),
    (0.745098, Rgb::new(251, 133, 96)),
    (0.764706, Rgb::new(252, 142, 100)),
    (0.784314, Rgb::new(253, 152, 105)),
    (0.803922, Rgb::new(254, 161, 110)),
    (0.823529, Rgb::new(254, 170, 116)),
    (0.843137, Rgb::new(254, 180, 123)),
    (0.862745, Rgb::new(254, 189, 130)),
    (0.882353, Rgb::new(254, 198, 138)),
    (0.901961, Rgb::new(254, 207, 146)),
    (0.921569, Rgb::new(254, 216, 154)),
    (0.941176, Rgb::new(253, 226, 163)),
    (0.960784, Rgb::new(253, 235, 172)),
    (0.980392, Rgb::new(252, 244, 182)),
    (1.0, Rgb::new(252, 253, 191)),
];

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
    builder(VectorStyle::graduated("SID79", 6, 0.0, 57.0, MAGMA_RAMP))?
        .write_to("out/magma.qgs")?;

    println!("wrote out/load.qgs, out/red.qgs and out/magma.qgs");
    Ok(())
}
