//! Spatial reference systems, written into `<srs>`/`<spatialrefsys>` nodes.
//!
//! A layer's SRS is written as a `<spatialrefsys>` block whose `<wkt>`
//! element holds the WKT2 definition. QGIS re-resolves the CRS from it (and
//! from `<authid>`) on load, so the other fields only need to be plausible;
//! `<proj4>` is not needed and is not emitted.
//!
//! The SRS can be specified either as an EPSG code or as a WKT2 string;
//! conversion between the two is done with the [`epsg_utils`] crate.

use crate::xml::XmlWriter;

/// The spatial reference system of a layer.
///
/// Construct it from an EPSG code or from a WKT2 string. The `add_*_layer`
/// methods take `impl Into<Srs>`, so an integer literal or a string can be
/// passed directly:
///
/// ```
/// # use generate_qgs::Srs;
/// let a: Srs = 3857.into(); // EPSG code
/// let b: Srs = "GEOGCRS[\"WGS 84\",...]".into(); // WKT2
/// ```
#[derive(Debug, Clone)]
pub enum Srs {
    /// An EPSG code, e.g. `3857`. The WKT2 definition is looked up from the
    /// EPSG dataset embedded in `epsg-utils`.
    Epsg(i32),
    /// A WKT2 definition. The EPSG code is extracted from its
    /// `ID["EPSG", ...]` node if present.
    Wkt(String),
}

impl From<i32> for Srs {
    fn from(code: i32) -> Self {
        Srs::Epsg(code)
    }
}

impl From<u32> for Srs {
    fn from(code: u32) -> Self {
        Srs::Epsg(code as i32)
    }
}

impl From<&str> for Srs {
    fn from(wkt: &str) -> Self {
        Srs::Wkt(wkt.to_string())
    }
}

impl From<String> for Srs {
    fn from(wkt: String) -> Self {
        Srs::Wkt(wkt)
    }
}

/// Errors that can occur while resolving an [`Srs`].
#[derive(Debug)]
pub enum SrsError {
    /// The EPSG code is not contained in the `epsg-utils` dataset.
    UnknownEpsgCode(i32),
    /// The WKT2 string could not be parsed.
    WktParse(epsg_utils::ParseError),
}

impl std::fmt::Display for SrsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SrsError::UnknownEpsgCode(code) => {
                write!(f, "unknown EPSG code: {code}")
            }
            SrsError::WktParse(e) => write!(f, "failed to parse WKT2: {e}"),
        }
    }
}

impl std::error::Error for SrsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SrsError::UnknownEpsgCode(_) => None,
            SrsError::WktParse(e) => Some(e),
        }
    }
}

/// An [`Srs`] resolved into everything the `<spatialrefsys>` block needs.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedSrs {
    /// WKT2 definition.
    pub wkt: String,
    /// EPSG code, if known.
    pub epsg: Option<i32>,
    /// CRS name, e.g. `"WGS 84"`.
    pub name: String,
    /// Whether this is a geographic (lon/lat) CRS.
    pub geographic: bool,
}

impl Srs {
    pub(crate) fn resolve(&self) -> Result<ResolvedSrs, SrsError> {
        match self {
            Srs::Epsg(code) => {
                let wkt = epsg_utils::epsg_to_wkt2(*code)
                    .map_err(|_| SrsError::UnknownEpsgCode(*code))?;
                let crs = epsg_utils::parse_wkt2(wkt).map_err(SrsError::WktParse)?;
                Ok(ResolvedSrs {
                    wkt: wkt.to_string(),
                    epsg: crs.to_epsg().or(Some(*code)),
                    name: crs_name(&crs),
                    geographic: is_geographic(&crs),
                })
            }
            Srs::Wkt(wkt) => {
                let crs = epsg_utils::parse_wkt2(wkt).map_err(SrsError::WktParse)?;
                Ok(ResolvedSrs {
                    wkt: wkt.clone(),
                    epsg: crs.to_epsg(),
                    name: crs_name(&crs),
                    geographic: is_geographic(&crs),
                })
            }
        }
    }
}

fn crs_name(crs: &epsg_utils::Crs) -> String {
    match crs {
        epsg_utils::Crs::ProjectedCrs(c) => c.name.clone(),
        epsg_utils::Crs::GeogCrs(c) => c.name.clone(),
        epsg_utils::Crs::GeodCrs(c) => c.name.clone(),
        epsg_utils::Crs::VertCrs(c) => c.name.clone(),
        epsg_utils::Crs::CompoundCrs(c) => c.name.clone(),
    }
}

fn is_geographic(crs: &epsg_utils::Crs) -> bool {
    matches!(
        crs,
        epsg_utils::Crs::GeogCrs(_) | epsg_utils::Crs::GeodCrs(_)
    )
}

/// Writes a `<spatialrefsys nativeFormat="Wkt">...</spatialrefsys>` block.
pub(crate) fn write_spatialrefsys<'a>(
    w: &'a mut XmlWriter,
    srs: &ResolvedSrs,
) -> &'a mut XmlWriter {
    let (srid, authid) = match srs.epsg {
        Some(code) => (code.to_string(), format!("EPSG:{code}")),
        None => ("0".to_string(), String::new()),
    };
    w.start("spatialrefsys").attr("nativeFormat", "Wkt");
    w.elem("wkt", &srs.wkt);
    // `srsid` is QGIS's internal database id; it is re-resolved on load, so
    // the SRID itself is a good enough placeholder.
    w.elem("srsid", &srid);
    w.elem("srid", &srid);
    w.elem("authid", &authid);
    w.elem("description", &srs.name);
    w.elem("projectionacronym", "");
    w.elem("ellipsoidacronym", "");
    w.elem(
        "geographicflag",
        if srs.geographic { "true" } else { "false" },
    );
    w.end()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_epsg_code() {
        let srs = Srs::Epsg(4326).resolve().unwrap();
        assert_eq!(srs.epsg, Some(4326));
        assert_eq!(srs.name, "WGS 84");
        assert!(srs.geographic);

        let srs = Srs::Epsg(3857).resolve().unwrap();
        assert_eq!(srs.epsg, Some(3857));
        assert_eq!(srs.name, "WGS 84 / Pseudo-Mercator");
        assert!(!srs.geographic);
    }

    #[test]
    fn resolve_wkt_roundtrip() {
        let wkt = epsg_utils::epsg_to_wkt2(4267).unwrap();
        let srs = Srs::from(wkt).resolve().unwrap();
        assert_eq!(srs.epsg, Some(4267));
        assert_eq!(srs.name, "NAD27");
        assert!(srs.geographic);
    }

    #[test]
    fn unknown_epsg_code_is_an_error() {
        assert!(matches!(
            Srs::Epsg(-1).resolve(),
            Err(SrsError::UnknownEpsgCode(-1))
        ));
    }

    #[test]
    fn invalid_wkt_is_an_error() {
        assert!(matches!(
            Srs::from("not a wkt").resolve(),
            Err(SrsError::WktParse(_))
        ));
    }
}
