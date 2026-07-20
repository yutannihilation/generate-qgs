// methods and associated functions for QgsBuilder
SEXP savvy_QgsBuilder_add_raster_layer__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__layer_name, SEXP c_arg__srs, SEXP c_arg__style);
SEXP savvy_QgsBuilder_add_vector_layer__ffi(SEXP self__, SEXP c_arg__path, SEXP c_arg__layer_name, SEXP c_arg__srs, SEXP c_arg__geometry, SEXP c_arg__style);
SEXP savvy_QgsBuilder_add_xyz_tile_layer__ffi(SEXP self__, SEXP c_arg__name, SEXP c_arg__url, SEXP c_arg__zmin, SEXP c_arg__zmax);
SEXP savvy_QgsBuilder_build__ffi(SEXP self__);
SEXP savvy_QgsBuilder_new__ffi(void);
SEXP savvy_QgsBuilder_write_to__ffi(SEXP self__, SEXP c_arg__path);

// methods and associated functions for RasterStyle
SEXP savvy_RasterStyle_multiband__ffi(SEXP c_arg__red_band, SEXP c_arg__red_min, SEXP c_arg__red_max, SEXP c_arg__green_band, SEXP c_arg__green_min, SEXP c_arg__green_max, SEXP c_arg__blue_band, SEXP c_arg__blue_min, SEXP c_arg__blue_max);
SEXP savvy_RasterStyle_pseudocolor__ffi(SEXP c_arg__classes, SEXP c_arg__min, SEXP c_arg__max, SEXP c_arg__stop_offsets, SEXP c_arg__stop_r, SEXP c_arg__stop_g, SEXP c_arg__stop_b);
SEXP savvy_RasterStyle_pseudocolor_discrete__ffi(SEXP c_arg__classes, SEXP c_arg__min, SEXP c_arg__max, SEXP c_arg__stop_offsets, SEXP c_arg__stop_r, SEXP c_arg__stop_g, SEXP c_arg__stop_b);

// methods and associated functions for Rgb
SEXP savvy_Rgb_new__ffi(SEXP c_arg__r, SEXP c_arg__g, SEXP c_arg__b);

// methods and associated functions for VectorStyle
SEXP savvy_VectorStyle_categorized__ffi(SEXP c_arg__attribute, SEXP c_arg__values, SEXP c_arg__colors_r, SEXP c_arg__colors_g, SEXP c_arg__colors_b, SEXP c_arg__catch_all);
SEXP savvy_VectorStyle_graduated__ffi(SEXP c_arg__attribute, SEXP c_arg__classes, SEXP c_arg__min, SEXP c_arg__max, SEXP c_arg__stop_offsets, SEXP c_arg__stop_r, SEXP c_arg__stop_g, SEXP c_arg__stop_b);
SEXP savvy_VectorStyle_single__ffi(SEXP c_arg__color);
