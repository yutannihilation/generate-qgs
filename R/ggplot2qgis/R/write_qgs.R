# Number of equal-interval classes of a graduated renderer. QGIS classifies
# the attribute into this many ranges; the colors are interpolated from the
# gradient stops. Same default as the Rust demo.
QGS_GRADUATED_CLASSES <- 6L

#' Write a ggplot2 map plot as a QGIS project
#'
#' Converts a ggplot2 plot whose layers are drawn from sf objects into a
#' QGIS project (`.qgs`) file. The data of each layer is saved as a
#' GeoPackage under `<path minus extension>_data/`, and the layer is styled
#' after the plot's trained color scale:
#'
#' - a continuous `fill`/`colour` scale becomes a graduated renderer,
#' - a discrete one becomes a categorized renderer,
#' - a layer with no `fill`/`colour` mapping becomes a single symbol with
#'   the color ggplot2 would have used.
#'
#' Only a bare column name is supported for the `fill`/`colour` aesthetics;
#' a constant or a computed expression (e.g. `aes(fill = AREA * 2)`) is an
#' error.
#'
#' @param plot A ggplot object. All layers must be backed by sf data.
#' @param path Path of the `.qgs` file to write. Tilde paths (e.g. `~/x.qgs`)
#'   are expanded.
#' @param use_plot_crs If `TRUE`, the project (map canvas) CRS is the plot's
#'   CRS, resolved the way [ggplot2::coord_sf()] does: its `crs` argument if
#'   specified, otherwise the CRS of the first layer that defines one. If
#'   `FALSE` (the default), the project CRS is EPSG:3857 (Web Mercator).
#'   Either way the layers keep the CRS of their data; QGIS reprojects them
#'   on the fly.
#' @returns `path`, invisibly.
#' @examples
#' library(ggplot2)
#'
#' nc <- sf::st_read(system.file("shape/nc.shp", package = "sf"), quiet = TRUE)
#' p <- ggplot(nc) +
#'   geom_sf(aes(fill = AREA))
#'
#' write_qgs(p, tempfile(fileext = ".qgs"))
#' @importFrom rlang %||%
#' @export
write_qgs <- function(plot, path, use_plot_crs = FALSE) {
  if (!inherits(plot, "ggplot")) {
    stop("`plot` must be a ggplot object, got ", class(plot)[1], call. = FALSE)
  }
  layers <- plot@layers
  if (length(layers) == 0L) {
    stop("`plot` must have at least one layer", call. = FALSE)
  }
  if (!isTRUE(use_plot_crs) && !isFALSE(use_plot_crs)) {
    stop("`use_plot_crs` must be TRUE or FALSE", call. = FALSE)
  }

  path <- path.expand(path)

  # Build the plot first so that the scales are trained by the data.
  built <- ggplot2::ggplot_build(plot)

  data_dir_name <- paste0(tools::file_path_sans_ext(basename(path)), "_data")
  data_dir <- file.path(dirname(path), data_dir_name)
  dir.create(data_dir, recursive = TRUE, showWarnings = FALSE)

  builder <- QgsBuilder$new()

  # coord_sf() uses its crs argument if specified, otherwise the CRS of
  # the first layer that defines one; the built plot carries the result
  # (as given, so e.g. a bare EPSG code needs normalization).
  plot_crs <- NULL
  if (use_plot_crs) {
    plot_crs <- built@layout$panel_params[[1]]$crs
    if (!is.null(plot_crs)) {
      plot_crs <- sf::st_crs(plot_crs)
    }
  }

  # ggplot2's first layer is the bottom-most one, which is also the order
  # QgsBuilder expects.
  for (i in seq_along(layers)) {
    layer <- layers[[i]]

    # The raw data, not the computed data of ggplot_build(), which no
    # longer has the original values.
    d <- layer$data
    if (is.null(d) || inherits(d, "waiver")) {
      d <- plot@data
    }
    if (is.null(d) || inherits(d, "waiver")) {
      stop("layer ", i, " has no data", call. = FALSE)
    }
    if (!inherits(d, "sf")) {
      stop(
        "layer ", i, ": only sf data is supported at the moment, got ",
        class(d)[1],
        call. = FALSE
      )
    }

    crs <- sf::st_crs(d)
    if (is.na(crs)) {
      stop("layer ", i, ": the data has no CRS", call. = FALSE)
    }
    if (use_plot_crs && (is.null(plot_crs) || is.na(plot_crs))) {
      plot_crs <- crs
    }

    layer_name <- paste0("layer", i)
    gpkg_file <- paste0(layer_name, ".gpkg")
    gpkg_path <- file.path(data_dir, gpkg_file)
    if (file.exists(gpkg_path)) {
      unlink(gpkg_path)
    }
    sf::st_write(d, gpkg_path, layer = layer_name, quiet = TRUE)

    builder$add_vector_layer(
      # relative to the project file
      paste0(data_dir_name, "/", gpkg_file),
      layer_name,
      qgs_srs(crs),
      qgs_geometry_type(d, i),
      qgs_vector_style(plot, built, layer, i, d)
    )
  }

  if (use_plot_crs) {
    builder$set_project_crs(qgs_srs(plot_crs))
  }
  builder$write_to(path)

  invisible(path)
}

qgs_srs <- function(crs) {
  if (!is.na(crs$epsg)) {
    crs$epsg
  } else {
    crs$wkt
  }
}

qgs_geometry_type <- function(d, i) {
  type <- as.character(sf::st_geometry_type(d, by_geometry = FALSE))
  switch(type,
    POINT = ,
    MULTIPOINT = GeometryType$Point,
    LINESTRING = ,
    MULTILINESTRING = GeometryType$LineString,
    POLYGON = ,
    MULTIPOLYGON = GeometryType$Polygon,
    stop("layer ", i, ": unsupported geometry type ", type, call. = FALSE)
  )
}

# Resolves which aesthetic drives the color of the layer and returns the
# matching VectorStyle. The layer's mapping takes precedence over the
# plot's, following how ggplot2 itself resolves aesthetics.
qgs_vector_style <- function(plot, built, layer, i, d) {
  # aes() normalizes `color` to `colour`, so only these two keys exist.
  fill <- layer$mapping[["fill"]] %||% plot@mapping[["fill"]]
  colour <- layer$mapping[["colour"]] %||% plot@mapping[["colour"]]

  if (!is.null(fill)) {
    aes_name <- "fill"
    quo <- fill
  } else if (!is.null(colour)) {
    aes_name <- "colour"
    quo <- colour
  } else {
    return(qgs_single_style(built@data[[i]], i))
  }

  if (!(rlang::is_quosure(quo) && rlang::quo_is_symbol(quo))) {
    stop(
      "layer ", i, ": only a bare column name is supported for `", aes_name,
      "`, got `", rlang::as_label(quo), "`",
      call. = FALSE
    )
  }
  attribute <- rlang::as_string(rlang::quo_get_expr(quo))
  if (!attribute %in% names(d)) {
    stop(
      "layer ", i, ": column `", attribute, "` not found in the layer data",
      call. = FALSE
    )
  }

  scale <- built@plot@scales$get_scales(aes_name)
  if (scale$is_discrete()) {
    qgs_categorized_style(scale, attribute, i)
  } else {
    qgs_graduated_style(scale, attribute, i)
  }
}

# For a layer without a fill/colour mapping, use the constant color ggplot2
# computed for it.
qgs_single_style <- function(computed, i) {
  for (aes_name in c("fill", "colour")) {
    colors <- computed[[aes_name]]
    colors <- colors[!is.na(colors)]
    if (length(colors) > 0L) {
      return(VectorStyle$single(qgs_rgb(colors[[1L]])))
    }
  }
  stop("layer ", i, ": cannot determine the color of the layer", call. = FALSE)
}

qgs_graduated_style <- function(scale, attribute, i) {
  limits <- scale$get_limits()
  if (anyNA(limits) || limits[2L] <= limits[1L]) {
    stop(
      "layer ", i, ": cannot map `", attribute,
      "` to a color gradient (the scale's limits are degenerate)",
      call. = FALSE
    )
  }

  # Breaks outside the limits would be mapped to the na.value color, so
  # keep only the inner ones; the limits themselves become the terminal
  # gradient stops (offsets 0 and 1).
  breaks <- scale$get_breaks()
  breaks <- breaks[!is.na(breaks) & breaks > limits[1L] & breaks < limits[2L]]
  stops <- c(limits[1L], sort(breaks), limits[2L])
  colors <- grDevices::col2rgb(scale$map(stops))

  VectorStyle$graduated(
    attribute,
    classes = QGS_GRADUATED_CLASSES,
    min = limits[1L],
    max = limits[2L],
    stop_offsets = (stops - limits[1L]) / (limits[2L] - limits[1L]),
    stop_r = colors["red", ],
    stop_g = colors["green", ],
    stop_b = colors["blue", ]
  )
}

qgs_categorized_style <- function(scale, attribute, i) {
  values <- scale$get_breaks()
  values <- values[!is.na(values)]
  if (length(values) == 0L) {
    stop(
      "layer ", i, ": the scale of `", attribute, "` has no values",
      call. = FALSE
    )
  }
  colors <- grDevices::col2rgb(scale$map(values))

  VectorStyle$categorized(
    attribute,
    values = as.character(values),
    colors_r = colors["red", ],
    colors_g = colors["green", ],
    colors_b = colors["blue", ]
  )
}

qgs_rgb <- function(color) {
  rgb <- grDevices::col2rgb(color)
  Rgb$new(rgb[1L], rgb[2L], rgb[3L])
}
