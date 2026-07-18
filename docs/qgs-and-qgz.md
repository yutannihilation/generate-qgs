Understanding `.qgs` and `.qgz` file format
===========================================

Findings from comparing the three sample projects (`samples/blank.qgs`,
`samples/load.qgs`, `samples/red.qgs`), all saved by QGIS 4.2.0.

## `.qgs` vs `.qgz`

- `.qgs` is a plain XML file. This crate generates this format.
- `.qgz` is a ZIP archive bundling the same `.qgs` XML together with an
  auxiliary `.qgd` file (SQLite, used to store auxiliary data such as
  labels positioned manually). QGIS opens `.qgs` files directly, so we do
  not need `.qgz`.

## Overall structure of a `.qgs` file

```xml
<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis projectname="" saveDateTime="..." saveUser="..." saveUserFull="..." version="4.2.0-Belém do Pará">
  ... project-wide settings ...
  <layer-tree-group> ... layer tree entries ... </layer-tree-group>
  ...
  <projectlayers>
    <maplayer type="raster"> ... </maplayer>   <!-- XYZ tile layer -->
    <maplayer type="vector"> ... </maplayer>   <!-- .gpkg layer -->
  </projectlayers>
  <layerorder> ... </layerorder>
  ... more project-wide settings ...
</qgis>
```

Root `<qgis>` attributes (`projectname`, `saveDateTime`, `saveUser`,
`saveUserFull`, `version`) are informational; QGIS does not seem to rely on
them when loading.

## Nodes that store layer information (known)

A layer appears in **four** places, tied together by the layer id
(e.g. `nc_b1079259_f0a1_4ebf_8df3_a22a440d836b`). The id is
`<name-sanitized-to-ASCII-alphanumerics>_<uuid-with-underscores>`; when the
name has no ASCII alphanumerics (e.g. Japanese), the prefix is empty and the
id starts with `_`.

| Node | Role |
| --- | --- |
| `<layer-tree-group>/<layer-tree-layer>` | Entry in the Layers panel. `providerKey` (`ogr`/`wms`), `source` (same as `<datasource>`). Top-most layer comes first. |
| `<layer-tree-group>/<custom-order>/<item>` | Layer id list, bottom-most first. |
| `<legend>/<legendlayer>/<filegroup>/<legendlayerfile>` | Legend entries (same order as the layer tree). |
| `<projectlayers>/<maplayer>` | The actual layer definition: data source, CRS, renderer (style). |
| `<layerorder>/<layer>` | Drawing order, bottom-most first. |

### XYZ tile layer (`<maplayer type="raster" layerType="Raster">`)

- `<provider>wms</provider>` — XYZ tiles are handled by the WMS provider.
- `<datasource>` is an `&`-separated key=value URI with percent-encoded
  values, e.g.
  `crs=EPSG%3A3857&format&type=xyz&url=https%3A%2F%2F...%2F%7Bz%7D%2F%7Bx%7D%2F%7By%7D.png&zmax=18&zmin=0&http-header:referer=`.
  In XML, each `&` is written as `&amp;`.
- `<extent>` covers the whole EPSG:3857 world
  (`±20037508.342789...`); `<wgs84extent>` is `±180`, `±85.0511287798066`.
- `<pipe>/<rasterrenderer type="singlebandcolordata">` is the standard
  renderer for XYZ tiles.

### Vector layer (`<maplayer type="vector" layerType="Vector">`)

- `<provider encoding="UTF-8">ogr</provider>` — GDAL/OGR based, works for
  `.gpkg`.
- `<datasource>` is `<path-to.gpkg>|layername=<table-name>`. A relative path
  is resolved against the project file location
  (`<properties name="Paths">/Absolute = false`).
- `geometry` / `wkbType` attributes describe the geometry (e.g.
  `Polygon` / `MultiPolygon`).
- `<srs>/<spatialrefsys>` holds the layer CRS as WKT2 + authid. The `<proj4>`
  element written by QGIS is redundant — QGIS loads the project fine without
  it (it re-resolves the CRS from `<wkt>`/`<authid>`), so this crate does not
  emit it. The same applies to `srsid` (QGIS's internal database id) and to
  `projectionacronym`/`ellipsoidacronym`.
- `<renderer-v2>` holds the symbology:
  - `type="singleSymbol"`: one `<symbol type="fill">` with a `SimpleFill`
    layer; the fill color is in
    `<Option name="color" value="R,G,B,255,rgb:r,g,b,1"/>` (both integer
    0-255 and float 0-1 forms in one string).
  - `type="graduatedSymbol"` with `attr="<field>"` and
    `graduatedMethod="GraduatedColor"`: `<ranges>` list `lower`/`upper`
    bounds (15 decimal digits) referencing symbols by index; `<symbols>`
    contains one symbol per class with colors interpolated along the
    `<colorramp type="gradient">`;
    `<classificationMethod id="Pretty">` with
    `<labelFormat format="%1 - %2">` drives the legend labels.
  - The gradient `<colorramp>` holds the endpoints in `color1`/`color2`
    and, if the ramp has intermediate stops (like `samples/magma.qgs`), a
    `stops` option:
    `offset;color;rgb;ccw:offset;color;rgb;ccw:...` where `offset` is in
    `0..=1` (formatted with `%g`, 6 significant digits). Class colors are
    piecewise-linear interpolations between consecutive control points in
    RGB space, sampled at `i / (classes - 1)`.
  - `type="categorizedSymbol"` (like `samples/categorized.qgs`) is the
    discrete counterpart: `<categories>` maps attribute values to symbol
    ids — `<category label="..." symbol="0" type="string" value="..."/>`
    (the trailing "all other values" category is
    `type="NULL" value="NULL"` with an empty label) — and `<symbols>`
    holds one `<symbol name="0">`, `<symbol name="1">`, ... per category.
    It also carries a `<source-symbol>` and an informational
    `<colorramp>` (only used when re-classifying).
- Field-related nodes (`<fieldConfiguration>`, `<aliases>`, `<defaults>`,
  `<constraints>`, ...) repeat per-column boilerplate. QGIS regenerates
  them from the data source on load, so a generator can omit them.

## Nodes that are project boilerplate (known, static in this crate)

These are identical (or nearly) across the samples and are kept verbatim in
the template:

- `<projectCrs>` / `<verticalCrs>`: project coordinate reference systems.
- `<mapcanvas name="theMapCanvas">`: canvas units, `<extent>` (initial view),
  `<destinationsrs>` (display CRS).
- `<snapping-settings>`, `<relations/>`, `<polymorphicRelations/>`,
  `<projectModels/>`, `<mapViewDocks/>`.
- `<main-annotation-layer>`: the built-in annotation layer ("注記" in a
  Japanese locale).
- `<properties name="properties">`: big bag of project settings
  (GUI colors, measurement units, PAL labeling engine, WMS/WFS/WMTS server
  metadata, ...).
- `<visibility-presets/>`, `<projectMetadata>`, `<Annotations/>`,
  `<Layouts/>`, `<Bookmarks/>`, `<Sensors/>`, `<ProjectViewSettings>`,
  `<ProjectStyleSettings>`, `<ProjectTimeSettings>`,
  `<ElevationProperties>`, `<ProjectDisplaySettings>`,
  `<ProjectGpsSettings>`, ...

## Nodes that are unknown / ignored for now

- `<transformContext>`: datum transformation pairs used in the session
  (the samples store NAD27→EPSG:3857 because of the `nc` layer). We emit an
  empty `<transformContext/>`; QGIS recomputes it.
- `<elevation>` / `<temporal>` inside `<maplayer>`: 3D/elevation profile and
  temporal controller settings. Omitted; QGIS fills defaults.
- `iccProfileId` / `projectStyleId` (`attachment:///...`): references into
  the `.qgd` auxiliary database. Left empty.
- `<saveDateTime>`-style metadata, `<ProjectGpsSettings destinationLayer...>`:
  session state, regenerated by QGIS.
- Random UUIDs inside symbol layers (`<layer id="{...}">`) and graduated
  `<range uuid="{...}">`: only need to be unique within the document.

## Generation strategy (this crate)

1. Take `blank.qgs` (cleaned of user/session-specific values, `<proj4>`
   removed) as the static template.
2. For each added layer, generate the four entries listed above and splice
   them into the template at well-known anchors
   (`<custom-order enabled="0"/>`, `<legend updateDrawingOrder="true"/>`,
   `<projectlayers/>`, `<layerorder/>`).
3. Only the `<maplayer>` bodies differ between layer types and styles;
   everything else is shared boilerplate.

## SRS handling (this crate)

The SRS of a layer is given either as an EPSG code or as a WKT2 string, and
converted with the [epsg-utils](https://crates.io/crates/epsg-utils) crate:

- EPSG code → WKT2 via `epsg_utils::epsg_to_wkt2` (embedded EPSG dataset).
- WKT2 → EPSG code via `epsg_utils::parse_wkt2` + `Crs::to_epsg` (extracted
  from the trailing `ID["EPSG", ...]` node; absent codes are tolerated).

The CRS name (`<description>`) and `geographicflag` (true for
`GEOGCRS`/`GEODCRS`, false for `PROJCRS`) are also taken from the parsed
WKT2.
