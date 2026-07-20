
// clang-format sorts includes unless SortIncludes: Never. However, the ordering
// does matter here. So, we need to disable clang-format for safety.

// clang-format off
#include <stdint.h>
#include <Rinternals.h>
#include <R_ext/Parse.h>
// clang-format on

#include "rust/api.h"

static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;

SEXP handle_result(SEXP res_) {
    uintptr_t res = (uintptr_t)res_;

    // An error is indicated by tag.
    if ((res & TAGGED_POINTER_MASK) == 1) {
        // Remove tag
        SEXP res_aligned = (SEXP)(res & ~TAGGED_POINTER_MASK);

        // Currently, there are two types of error cases:
        //
        //   1. Error from Rust code
        //   2. Error from R's C API, which is caught by R_UnwindProtect()
        //
        if (TYPEOF(res_aligned) == CHARSXP) {
            // In case 1, the result is an error message that can be passed to
            // Rf_errorcall() directly.
            Rf_errorcall(R_NilValue, "%s", CHAR(res_aligned));
        } else {
            // In case 2, the result is the token to restart the
            // cleanup process on R's side.
            R_ContinueUnwind(res_aligned);
        }
    }

    return (SEXP)res;
}



SEXP savvy_QgsBuilder_add_raster_layer__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__layer_name, SEXP c_arg__srs, SEXP c_arg__style) {
    SEXP res = savvy_QgsBuilder_add_raster_layer__ffi(self__, c_arg__path, c_arg__layer_name, c_arg__srs, c_arg__style);
    return handle_result(res);
}

SEXP savvy_QgsBuilder_add_vector_layer__impl(SEXP self__, SEXP c_arg__path, SEXP c_arg__layer_name, SEXP c_arg__srs, SEXP c_arg__geometry, SEXP c_arg__style) {
    SEXP res = savvy_QgsBuilder_add_vector_layer__ffi(self__, c_arg__path, c_arg__layer_name, c_arg__srs, c_arg__geometry, c_arg__style);
    return handle_result(res);
}

SEXP savvy_QgsBuilder_add_xyz_tile_layer__impl(SEXP self__, SEXP c_arg__name, SEXP c_arg__url, SEXP c_arg__zmin, SEXP c_arg__zmax) {
    SEXP res = savvy_QgsBuilder_add_xyz_tile_layer__ffi(self__, c_arg__name, c_arg__url, c_arg__zmin, c_arg__zmax);
    return handle_result(res);
}

SEXP savvy_QgsBuilder_build__impl(SEXP self__) {
    SEXP res = savvy_QgsBuilder_build__ffi(self__);
    return handle_result(res);
}

SEXP savvy_QgsBuilder_new__impl(void) {
    SEXP res = savvy_QgsBuilder_new__ffi();
    return handle_result(res);
}

SEXP savvy_QgsBuilder_set_project_crs__impl(SEXP self__, SEXP c_arg__srs) {
    SEXP res = savvy_QgsBuilder_set_project_crs__ffi(self__, c_arg__srs);
    return handle_result(res);
}

SEXP savvy_QgsBuilder_write_to__impl(SEXP self__, SEXP c_arg__path) {
    SEXP res = savvy_QgsBuilder_write_to__ffi(self__, c_arg__path);
    return handle_result(res);
}

SEXP savvy_RasterStyle_multiband__impl(SEXP c_arg__red_band, SEXP c_arg__red_min, SEXP c_arg__red_max, SEXP c_arg__green_band, SEXP c_arg__green_min, SEXP c_arg__green_max, SEXP c_arg__blue_band, SEXP c_arg__blue_min, SEXP c_arg__blue_max) {
    SEXP res = savvy_RasterStyle_multiband__ffi(c_arg__red_band, c_arg__red_min, c_arg__red_max, c_arg__green_band, c_arg__green_min, c_arg__green_max, c_arg__blue_band, c_arg__blue_min, c_arg__blue_max);
    return handle_result(res);
}

SEXP savvy_RasterStyle_pseudocolor__impl(SEXP c_arg__classes, SEXP c_arg__min, SEXP c_arg__max, SEXP c_arg__stop_offsets, SEXP c_arg__stop_r, SEXP c_arg__stop_g, SEXP c_arg__stop_b) {
    SEXP res = savvy_RasterStyle_pseudocolor__ffi(c_arg__classes, c_arg__min, c_arg__max, c_arg__stop_offsets, c_arg__stop_r, c_arg__stop_g, c_arg__stop_b);
    return handle_result(res);
}

SEXP savvy_RasterStyle_pseudocolor_discrete__impl(SEXP c_arg__classes, SEXP c_arg__min, SEXP c_arg__max, SEXP c_arg__stop_offsets, SEXP c_arg__stop_r, SEXP c_arg__stop_g, SEXP c_arg__stop_b) {
    SEXP res = savvy_RasterStyle_pseudocolor_discrete__ffi(c_arg__classes, c_arg__min, c_arg__max, c_arg__stop_offsets, c_arg__stop_r, c_arg__stop_g, c_arg__stop_b);
    return handle_result(res);
}

SEXP savvy_Rgb_new__impl(SEXP c_arg__r, SEXP c_arg__g, SEXP c_arg__b) {
    SEXP res = savvy_Rgb_new__ffi(c_arg__r, c_arg__g, c_arg__b);
    return handle_result(res);
}

SEXP savvy_VectorStyle_categorized__impl(SEXP c_arg__attribute, SEXP c_arg__values, SEXP c_arg__colors_r, SEXP c_arg__colors_g, SEXP c_arg__colors_b, SEXP c_arg__catch_all) {
    SEXP res = savvy_VectorStyle_categorized__ffi(c_arg__attribute, c_arg__values, c_arg__colors_r, c_arg__colors_g, c_arg__colors_b, c_arg__catch_all);
    return handle_result(res);
}

SEXP savvy_VectorStyle_continuous__impl(SEXP c_arg__attribute, SEXP c_arg__min, SEXP c_arg__max, SEXP c_arg__stop_offsets, SEXP c_arg__stop_r, SEXP c_arg__stop_g, SEXP c_arg__stop_b) {
    SEXP res = savvy_VectorStyle_continuous__ffi(c_arg__attribute, c_arg__min, c_arg__max, c_arg__stop_offsets, c_arg__stop_r, c_arg__stop_g, c_arg__stop_b);
    return handle_result(res);
}

SEXP savvy_VectorStyle_graduated__impl(SEXP c_arg__attribute, SEXP c_arg__classes, SEXP c_arg__min, SEXP c_arg__max, SEXP c_arg__stop_offsets, SEXP c_arg__stop_r, SEXP c_arg__stop_g, SEXP c_arg__stop_b) {
    SEXP res = savvy_VectorStyle_graduated__ffi(c_arg__attribute, c_arg__classes, c_arg__min, c_arg__max, c_arg__stop_offsets, c_arg__stop_r, c_arg__stop_g, c_arg__stop_b);
    return handle_result(res);
}

SEXP savvy_VectorStyle_single__impl(SEXP c_arg__color) {
    SEXP res = savvy_VectorStyle_single__ffi(c_arg__color);
    return handle_result(res);
}


static const R_CallMethodDef CallEntries[] = {


    {"savvy_QgsBuilder_add_raster_layer__impl", (DL_FUNC) &savvy_QgsBuilder_add_raster_layer__impl, 5},
    {"savvy_QgsBuilder_add_vector_layer__impl", (DL_FUNC) &savvy_QgsBuilder_add_vector_layer__impl, 6},
    {"savvy_QgsBuilder_add_xyz_tile_layer__impl", (DL_FUNC) &savvy_QgsBuilder_add_xyz_tile_layer__impl, 5},
    {"savvy_QgsBuilder_build__impl", (DL_FUNC) &savvy_QgsBuilder_build__impl, 1},
    {"savvy_QgsBuilder_new__impl", (DL_FUNC) &savvy_QgsBuilder_new__impl, 0},
    {"savvy_QgsBuilder_set_project_crs__impl", (DL_FUNC) &savvy_QgsBuilder_set_project_crs__impl, 2},
    {"savvy_QgsBuilder_write_to__impl", (DL_FUNC) &savvy_QgsBuilder_write_to__impl, 2},
    {"savvy_RasterStyle_multiband__impl", (DL_FUNC) &savvy_RasterStyle_multiband__impl, 9},
    {"savvy_RasterStyle_pseudocolor__impl", (DL_FUNC) &savvy_RasterStyle_pseudocolor__impl, 7},
    {"savvy_RasterStyle_pseudocolor_discrete__impl", (DL_FUNC) &savvy_RasterStyle_pseudocolor_discrete__impl, 7},
    {"savvy_Rgb_new__impl", (DL_FUNC) &savvy_Rgb_new__impl, 3},
    {"savvy_VectorStyle_categorized__impl", (DL_FUNC) &savvy_VectorStyle_categorized__impl, 6},
    {"savvy_VectorStyle_continuous__impl", (DL_FUNC) &savvy_VectorStyle_continuous__impl, 7},
    {"savvy_VectorStyle_graduated__impl", (DL_FUNC) &savvy_VectorStyle_graduated__impl, 8},
    {"savvy_VectorStyle_single__impl", (DL_FUNC) &savvy_VectorStyle_single__impl, 1},
    {NULL, NULL, 0}
};

void R_init_ggplot2qgis(DllInfo *dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);

    // Functions for initialization, if any.

}
