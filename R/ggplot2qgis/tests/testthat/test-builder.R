test_that("an empty project keeps the template anchors", {
  out <- QgsBuilder$new()$build()
  expect_type(out, "character")
  expect_match(out, '<custom-order enabled="0"/>', fixed = TRUE)
  expect_match(out, '<legend updateDrawingOrder="true"/>', fixed = TRUE)
  expect_match(out, '<projectlayers/>', fixed = TRUE)
  expect_match(out, '<layerorder/>', fixed = TRUE)
})

test_that("a vector layer with a single style shows up in the project", {
  b <- QgsBuilder$new()
  b$add_xyz_tile_layer(
    "gsi tiles",
    "https://cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png",
    0L, 18L
  )
  b$add_vector_layer(
    "nc.gpkg", "nc", 4267L,
    GeometryType$Polygon,
    VectorStyle$single(Rgb$new(232L, 113L, 141L))
  )
  out <- b$build()
  expect_match(out, "nc.gpkg|layername=nc", fixed = TRUE)
  expect_match(out, 'type="singleSymbol"', fixed = TRUE)
  expect_match(out, 'geometry="Polygon"', fixed = TRUE)
  expect_match(out, "232,113,141,255,rgb:", fixed = TRUE)
  # The vector layer's SRS, and the XYZ tile layer's one (always EPSG:3857).
  expect_match(out, "<authid>EPSG:4267</authid>", fixed = TRUE)
  expect_match(out, "<authid>EPSG:3857</authid>", fixed = TRUE)
})

test_that("graduated and categorized styles show up in the project", {
  b <- QgsBuilder$new()
  b$add_vector_layer(
    "nc.gpkg", "graduated", 4267,
    GeometryType$Polygon,
    VectorStyle$graduated(
      "SID79", 6L, 0, 57,
      stop_offsets = c(0, 1),
      stop_r = c(255L, 255L),
      stop_g = c(255L, 0L),
      stop_b = c(255L, 0L)
    )
  )
  b$add_vector_layer(
    "nc.gpkg", "categorized", 4267,
    GeometryType$Point,
    VectorStyle$categorized(
      "NAME",
      values = c("Alamance", "Alexander"),
      colors_r = c(255L, 255L),
      colors_g = c(255L, 252L),
      colors_b = c(255L, 252L),
      catch_all = Rgb$new(255L, 0L, 0L)
    )
  )
  out <- b$build()
  expect_match(out, 'type="graduatedSymbol"', fixed = TRUE)
  expect_match(out, 'attr="SID79"', fixed = TRUE)
  expect_match(out, 'type="categorizedSymbol"', fixed = TRUE)
  expect_match(out, 'value="Alamance"', fixed = TRUE)
  expect_match(out, 'value="NULL"', fixed = TRUE)
  # Note the SRS was given as a numeric (not an integer).
  expect_match(out, "<authid>EPSG:4267</authid>", fixed = TRUE)
})

test_that("raster layers show up in the project", {
  stops <- list(
    stop_offsets = c(0, 1),
    stop_r = c(215L, 43L),
    stop_g = c(25L, 131L),
    stop_b = c(28L, 186L)
  )
  b <- QgsBuilder$new()
  b$add_raster_layer(
    "volcano2.tif", "volcano2", 2193L,
    do.call(RasterStyle$pseudocolor, c(list(5L, 80, 200), stops))
  )
  b$add_raster_layer(
    "volcano2.tif", "volcano2_discrete", 2193L,
    do.call(RasterStyle$pseudocolor_discrete, c(list(10L, 80, 200), stops))
  )
  b$add_raster_layer(
    "cyl_tile.tif", "cyl_tile", 3857L,
    RasterStyle$multiband(1L, 35, 253, 2L, 35, 251, 3L, 35, 250)
  )
  out <- b$build()
  expect_match(out, "<provider>gdal</provider>", fixed = TRUE)
  expect_match(out, 'type="singlebandpseudocolor"', fixed = TRUE)
  expect_match(out, 'colorRampType="INTERPOLATED"', fixed = TRUE)
  expect_match(out, 'colorRampType="DISCRETE"', fixed = TRUE)
  expect_match(out, 'type="multibandcolor"', fixed = TRUE)
  expect_match(out, "<authid>EPSG:2193</authid>", fixed = TRUE)
})

test_that("srs can be a WKT2 string", {
  wkt <- paste0(
    'GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",',
    'ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],',
    'CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1]],',
    'AXIS["geodetic longitude (Lon)",east,ORDER[2]],',
    'ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",4326]]'
  )
  b <- QgsBuilder$new()
  b$add_vector_layer(
    "points.gpkg", "stations", wkt,
    GeometryType$Point,
    VectorStyle$single(Rgb$new(200L, 30L, 30L))
  )
  out <- b$build()
  expect_match(out, "<authid>EPSG:4326</authid>", fixed = TRUE)
})

test_that("an invalid srs is an error", {
  b <- QgsBuilder$new()
  expect_error(
    b$add_vector_layer(
      "nc.gpkg", "nc", -1L,
      GeometryType$Polygon,
      VectorStyle$single(Rgb$new(0L, 0L, 0L))
    ),
    "unknown EPSG code"
  )
  expect_error(
    b$add_vector_layer(
      "nc.gpkg", "nc", "not a wkt",
      GeometryType$Polygon,
      VectorStyle$single(Rgb$new(0L, 0L, 0L))
    ),
    "failed to parse WKT2"
  )
  expect_error(
    b$add_vector_layer(
      "nc.gpkg", "nc", TRUE,
      GeometryType$Polygon,
      VectorStyle$single(Rgb$new(0L, 0L, 0L))
    ),
    "EPSG code"
  )
})

test_that("an invalid geometry type is an error", {
  b <- QgsBuilder$new()
  expect_error(
    b$add_vector_layer("nc.gpkg", "nc", 4267L, "polygon", VectorStyle$single(Rgb$new(0L, 0L, 0L))),
    "Expected ggplot2qgis::GeometryType"
  )
})

test_that("an invalid zoom level is an error", {
  b <- QgsBuilder$new()
  expect_error(
    b$add_xyz_tile_layer("x", "https://example.com/{z}/{x}/{y}.png", -1L, 18L),
    "between 0 and 255"
  )
})

test_that("write_to() writes the same content as build()", {
  b <- QgsBuilder$new()
  b$add_xyz_tile_layer("osm", "https://tile.openstreetmap.org/{z}/{x}/{y}.png", 0L, 19L)

  f <- tempfile(fileext = ".qgs")
  on.exit(unlink(f))
  b$write_to(f)

  expect_true(file.exists(f))
  expect_identical(
    readBin(f, "raw", file.size(f)),
    charToRaw(enc2utf8(b$build()))
  )
})

test_that("write_to() fails on a non-writable path", {
  b <- QgsBuilder$new()
  expect_error(b$write_to(file.path(tempdir(), "no-such-dir", "x.qgs")))
})
