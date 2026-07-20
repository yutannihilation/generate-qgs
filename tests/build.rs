use generate_qgs::{GeometryType, QgsBuilder, RasterStyle, Rgb, Srs, VectorStyle};
use quick_xml::Reader;
use quick_xml::events::Event;

fn assert_well_formed(xml: &str) {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => panic!("malformed XML: {e}"),
        }
    }
}

#[test]
fn empty_project_is_well_formed() {
    assert_well_formed(&QgsBuilder::new().build());
}

#[test]
fn full_project_is_well_formed() {
    let mut b = QgsBuilder::new();
    b.add_xyz_tile_layer(
        "地理院タイル（標準地図）",
        "https://cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png",
        0,
        18,
    );
    // SRS by EPSG code:
    b.add_vector_layer(
        "../tmp/nc.gpkg",
        "nc",
        4267,
        GeometryType::Polygon,
        VectorStyle::single(Rgb::new(232, 113, 141)),
    )
    .unwrap();
    // SRS by WKT2 string:
    b.add_vector_layer(
        "points.gpkg",
        "stations & stops",
        Srs::Wkt(epsg_utils::epsg_to_wkt2(4326).unwrap().to_string()),
        GeometryType::Point,
        VectorStyle::graduated(
            "value",
            5,
            -3.5,
            100.25,
            &[
                (0.0, Rgb::new(255, 255, 255)),
                (0.3, Rgb::new(255, 255, 0)),
                (1.0, Rgb::new(0, 0, 255)),
            ],
        ),
    )
    .unwrap();
    b.add_vector_layer(
        "lines.gpkg",
        "roads",
        "GEOGCRS[\"WGS 84\",ENSEMBLE[\"World Geodetic System 1984 ensemble\",MEMBER[\"World Geodetic System 1984 (Transit)\"],MEMBER[\"World Geodetic System 1984 (G730)\"],MEMBER[\"World Geodetic System 1984 (G873)\"],MEMBER[\"World Geodetic System 1984 (G1150)\"],MEMBER[\"World Geodetic System 1984 (G1674)\"],MEMBER[\"World Geodetic System 1984 (G1762)\"],MEMBER[\"World Geodetic System 1984 (G2139)\"],MEMBER[\"World Geodetic System 1984 (G2296)\"],ELLIPSOID[\"WGS 84\",6378137,298.257223563,LENGTHUNIT[\"metre\",1]],ENSEMBLEACCURACY[2.0]],PRIMEM[\"Greenwich\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],CS[ellipsoidal,2],AXIS[\"geodetic latitude (Lat)\",north,ORDER[1],ANGLEUNIT[\"degree\",0.0174532925199433]],AXIS[\"geodetic longitude (Lon)\",east,ORDER[2],ANGLEUNIT[\"degree\",0.0174532925199433]],USAGE[SCOPE[\"Horizontal component of 3D system.\"],AREA[\"World.\"],BBOX[-90,-180,90,180]],ID[\"EPSG\",4326]]",
        GeometryType::LineString,
        VectorStyle::single(Rgb::new(200, 30, 30)),
    )
    .unwrap();
    let out = b.build();
    assert_well_formed(&out);

    // The SRS should show up as authid in each layer's spatialrefsys block.
    assert!(out.contains("<authid>EPSG:4267</authid>"));
    assert!(out.contains("<authid>EPSG:4326</authid>"));
    assert!(out.contains("<authid>EPSG:3857</authid>"));
    // proj4 is gone everywhere.
    assert!(!out.contains("<proj4>"));
}

#[test]
fn raster_project_is_well_formed() {
    let ramp = [
        (0.0, Rgb::new(215, 25, 28)),
        (0.25, Rgb::new(253, 174, 97)),
        (0.5, Rgb::new(255, 255, 191)),
        (0.75, Rgb::new(171, 221, 164)),
        (1.0, Rgb::new(43, 131, 186)),
    ];
    let mut b = QgsBuilder::new();
    b.add_raster_layer(
        "../tmp/volcano2.tif",
        "volcano2",
        2193,
        RasterStyle::pseudocolor(5, 80.0, 200.0, &ramp),
    )
    .unwrap();
    b.add_raster_layer(
        "../tmp/volcano2.tif",
        "volcano2_discrete",
        2193,
        RasterStyle::pseudocolor_discrete(10, 80.0, 200.0, &ramp),
    )
    .unwrap();
    b.add_raster_layer(
        "../tmp/cyl_tile.tif",
        "cyl_tile",
        3857,
        RasterStyle::multiband((1, 35.0, 253.0), (2, 35.0, 251.0), (3, 35.0, 250.0)),
    )
    .unwrap();
    let out = b.build();
    assert_well_formed(&out);

    assert!(out.contains("<provider>gdal</provider>"));
    assert!(out.contains("providerKey=\"gdal\""));
    assert!(out.contains("type=\"singlebandpseudocolor\""));
    assert!(out.contains("colorRampType=\"INTERPOLATED\""));
    assert!(out.contains("colorRampType=\"DISCRETE\""));
    assert!(out.contains("type=\"multibandcolor\""));
    // One noData entry for the single-band layer, three for the RGB one.
    assert!(out.contains("<noDataList bandNo=\"3\" useSrcNoData=\"1\"/>"));
    // The discrete legend labels (`<` is XML-escaped, `>` is not).
    assert!(out.contains("label=\"&lt;= 92\""));
    assert!(out.contains("label=\"> 188\" value=\"inf\""));
}

#[test]
fn layer_ids_are_consistent_across_sections() {
    let mut b = QgsBuilder::new();
    b.add_xyz_tile_layer(
        "osm",
        "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
        0,
        19,
    );
    let out = b.build();

    // The single generated layer id must appear in the tree, the legend,
    // the maplayer and the layer order.
    let id = {
        let marker = "<layer-tree-layer checked=\"Qt::Checked\" expanded=\"1\" id=\"";
        let start = out.find(marker).expect("layer-tree-layer not found") + marker.len();
        out[start..out[start..].find('"').unwrap() + start].to_string()
    };
    assert!(out.contains(&format!("layerid=\"{id}\"")));
    assert!(out.contains(&format!("<id>{id}</id>")));
    assert!(out.contains(&format!("<layer id=\"{id}\"/>")));
    assert!(out.contains(&format!("<item>{id}</item>")));
}
