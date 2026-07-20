test_that("Rgb$new creates a color", {
  color <- Rgb$new(232L, 113L, 141L)
  expect_s3_class(color, "ggplot2qgis::Rgb")
})

test_that("Rgb$new rejects out-of-range channels", {
  expect_error(Rgb$new(-1L, 0L, 0L), "between 0 and 255")
  expect_error(Rgb$new(0L, 256L, 0L), "between 0 and 255")
  expect_error(Rgb$new(0L, 0L, NA_integer_), "non-missing")
})

test_that("VectorStyle$single creates a style", {
  style <- VectorStyle$single(Rgb$new(255L, 0L, 0L))
  expect_s3_class(style, "ggplot2qgis::VectorStyle")
})

test_that("VectorStyle$single rejects a non-Rgb input", {
  expect_error(VectorStyle$single("#FF0000"), "Expected ggplot2qgis::Rgb")
})

test_that("VectorStyle$graduated creates a style", {
  style <- VectorStyle$graduated(
    "SID79", 6L, 0, 57,
    stop_offsets = c(0, 1),
    stop_r = c(255L, 255L),
    stop_g = c(255L, 0L),
    stop_b = c(255L, 0L)
  )
  expect_s3_class(style, "ggplot2qgis::VectorStyle")
})

test_that("VectorStyle$graduated validates the arguments", {
  # classes < 2
  expect_error(
    VectorStyle$graduated("x", 1L, 0, 1, c(0, 1), c(0L, 0L), c(0L, 0L), c(0L, 0L)),
    "at least 2 classes"
  )
  # negative classes (would overflow a usize)
  expect_error(
    VectorStyle$graduated("x", -1L, 0, 1, c(0, 1), c(0L, 0L), c(0L, 0L), c(0L, 0L)),
    "positive integer"
  )
  # min >= max
  expect_error(
    VectorStyle$graduated("x", 2L, 1, 1, c(0, 1), c(0L, 0L), c(0L, 0L), c(0L, 0L)),
    "smaller than max"
  )
  # only one color stop
  expect_error(
    VectorStyle$graduated("x", 2L, 0, 1, 0, 0L, 0L, 0L),
    "at least 2 color stops"
  )
  # offsets not covering 0..1
  expect_error(
    VectorStyle$graduated("x", 2L, 0, 1, c(0.1, 1), c(0L, 0L), c(0L, 0L), c(0L, 0L)),
    "offset 0.0"
  )
  # offsets not ascending
  expect_error(
    VectorStyle$graduated(
      "x", 2L, 0, 1,
      c(0, 0.5, 0.5, 1), c(0L, 0L, 0L, 0L), c(0L, 0L, 0L, 0L), c(0L, 0L, 0L, 0L)
    ),
    "ascending"
  )
  # NA offset
  expect_error(
    VectorStyle$graduated("x", 2L, 0, 1, c(0, NA_real_), c(0L, 0L), c(0L, 0L), c(0L, 0L)),
    "NA"
  )
  # channel vectors of different lengths
  expect_error(
    VectorStyle$graduated("x", 2L, 0, 1, c(0, 1), c(0L, 0L), 0L, c(0L, 0L)),
    "same length"
  )
  # offsets and channels of different lengths
  expect_error(
    VectorStyle$graduated("x", 2L, 0, 1, c(0, 0.5, 1), c(0L, 0L), c(0L, 0L), c(0L, 0L)),
    "same length"
  )
})

test_that("VectorStyle$categorized creates a style", {
  style <- VectorStyle$categorized(
    "NAME",
    values = c("Alamance", "Alexander"),
    colors_r = c(255L, 255L),
    colors_g = c(255L, 252L),
    colors_b = c(255L, 252L),
    catch_all = Rgb$new(255L, 0L, 0L)
  )
  expect_s3_class(style, "ggplot2qgis::VectorStyle")
})

test_that("VectorStyle$categorized validates the arguments", {
  # no category
  expect_error(
    VectorStyle$categorized("x", character(), integer(), integer(), integer()),
    "at least 1 category"
  )
  # NA value
  expect_error(
    VectorStyle$categorized("x", NA_character_, 0L, 0L, 0L),
    "must not contain NA"
  )
  # mismatched lengths
  expect_error(
    VectorStyle$categorized("x", c("a", "b"), 0L, 0L, 0L),
    "same length"
  )
})

test_that("RasterStyle$pseudocolor and pseudocolor_discrete create styles", {
  stops <- list(
    stop_offsets = c(0, 1),
    stop_r = c(215L, 43L),
    stop_g = c(25L, 131L),
    stop_b = c(28L, 186L)
  )
  expect_s3_class(
    do.call(RasterStyle$pseudocolor, c(list(5L, 80, 200), stops)),
    "ggplot2qgis::RasterStyle"
  )
  expect_s3_class(
    do.call(RasterStyle$pseudocolor_discrete, c(list(10L, 80, 200), stops)),
    "ggplot2qgis::RasterStyle"
  )
})

test_that("RasterStyle$multiband creates a style and validates band numbers", {
  expect_s3_class(
    RasterStyle$multiband(1L, 35, 253, 2L, 35, 251, 3L, 35, 250),
    "ggplot2qgis::RasterStyle"
  )
  expect_error(
    RasterStyle$multiband(0L, 35, 253, 2L, 35, 251, 3L, 35, 250),
    "1-based"
  )
  expect_error(
    RasterStyle$multiband(1L, 253, 35, 2L, 35, 251, 3L, 35, 250),
    "smaller than max"
  )
})

test_that("GeometryType variants exist", {
  expect_s3_class(GeometryType$Point, "ggplot2qgis::GeometryType")
  expect_s3_class(GeometryType$LineString, "ggplot2qgis::GeometryType")
  expect_s3_class(GeometryType$Polygon, "ggplot2qgis::GeometryType")
  expect_error(GeometryType$MultiPolygon, "Unknown variant")
})
