use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Coordinate {
    pub lat: f64,
    pub lon: f64,
}

impl Coordinate {
    pub fn distance_km(self, other: Coordinate) -> f64 {
        let lat_delta = (self.lat - other.lat) * 111.0;
        let lon_delta = (self.lon - other.lon) * 111.0 * self.lat.to_radians().cos().abs().max(0.2);
        (lat_delta.powi(2) + lon_delta.powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Bounds {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

impl Bounds {
    pub fn contains(self, point: Coordinate) -> bool {
        point.lat >= self.min_lat
            && point.lat <= self.max_lat
            && point.lon >= self.min_lon
            && point.lon <= self.max_lon
    }

    pub fn intersects(self, other: Bounds) -> bool {
        self.min_lat <= other.max_lat
            && self.max_lat >= other.min_lat
            && self.min_lon <= other.max_lon
            && self.max_lon >= other.min_lon
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeometryType {
    Point,
    LineString,
    Polygon,
}

impl GeometryType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Point => "point",
            Self::LineString => "line_string",
            Self::Polygon => "polygon",
        }
    }
}

impl std::str::FromStr for GeometryType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "point" => Ok(Self::Point),
            "line_string" => Ok(Self::LineString),
            "polygon" => Ok(Self::Polygon),
            _ => Err(format!("unsupported geometry type: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "coordinates", rename_all = "snake_case")]
pub enum Geometry {
    Point(Coordinate),
    LineString(Vec<Coordinate>),
    Polygon(Vec<Coordinate>),
}

impl Geometry {
    pub fn centroid(&self) -> Coordinate {
        match self {
            Self::Point(coordinate) => *coordinate,
            Self::LineString(points) | Self::Polygon(points) => average(points),
        }
    }

    pub fn bounds(&self) -> Bounds {
        match self {
            Self::Point(coordinate) => Bounds {
                min_lat: coordinate.lat,
                min_lon: coordinate.lon,
                max_lat: coordinate.lat,
                max_lon: coordinate.lon,
            },
            Self::LineString(points) | Self::Polygon(points) => {
                let mut min_lat = f64::MAX;
                let mut min_lon = f64::MAX;
                let mut max_lat = f64::MIN;
                let mut max_lon = f64::MIN;
                for point in points {
                    min_lat = min_lat.min(point.lat);
                    min_lon = min_lon.min(point.lon);
                    max_lat = max_lat.max(point.lat);
                    max_lon = max_lon.max(point.lon);
                }
                Bounds {
                    min_lat,
                    min_lon,
                    max_lat,
                    max_lon,
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapFeature {
    pub id: String,
    pub label: String,
    pub geometry: Geometry,
    #[serde(default)]
    pub properties: serde_json::Value,
}

fn average(points: &[Coordinate]) -> Coordinate {
    let (lat_sum, lon_sum) = points.iter().fold((0.0, 0.0), |acc, point| {
        (acc.0 + point.lat, acc.1 + point.lon)
    });
    Coordinate {
        lat: lat_sum / points.len() as f64,
        lon: lon_sum / points.len() as f64,
    }
}
