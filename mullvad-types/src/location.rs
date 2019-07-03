use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub type CountryCode = String;
pub type CityCode = String;
pub type Hostname = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub country: String,
    pub country_code: CountryCode,
    pub city: String,
    pub city_code: CityCode,
    pub latitude: f64,
    pub longitude: f64,
}

const RAIDUS_OF_EARTH: f64 = 6372.8;

impl Location {
    pub fn distance_from(&self, other: &Location) -> f64 {
        haversine_dist_deg(
            self.latitude,
            self.longitude,
            other.latitude,
            other.longitude,
        )
    }
}

/// Takes input as latitude and longitude degrees.
fn haversine_dist_deg(lat: f64, lon: f64, other_lat: f64, other_lon: f64) -> f64 {
    haversine_dist_rad(
        lat.to_radians(),
        lon.to_radians(),
        other_lat.to_radians(),
        other_lon.to_radians(),
    )
}
/// Implemented as per https://en.wikipedia.org/wiki/Haversine_formula and https://rosettacode.org/wiki/Haversine_formula#Rust
/// Takes input as radians, outputs kilometers.
fn haversine_dist_rad(lat: f64, lon: f64, other_lat: f64, other_lon: f64) -> f64 {
    let d_lat = lat - other_lat;
    let d_lon = lon - other_lon;
    // Computing the haversine between two points
    let haversine =
        (d_lat / 2.0).sin().powi(2) + (d_lon / 2.0).sin().powi(2) * lat.cos() * other_lat.cos();

    // using the haversine to compute the distance between two points
    haversine.sqrt().asin() * 2.0 * RAIDUS_OF_EARTH
}


/// The response from the am.i.mullvad.net location service.
#[derive(Debug, Deserialize)]
pub struct AmIMullvad {
    pub ip: IpAddr,
    pub country: String,
    pub city: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub mullvad_exit_ip: bool,
}

/// GeoIP information exposed from the daemon to frontends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpLocation {
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,
    pub country: String,
    pub city: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub mullvad_exit_ip: bool,
    pub hostname: Option<String>,
    pub bridge_hostname: Option<String>,
}

impl From<AmIMullvad> for GeoIpLocation {
    fn from(location: AmIMullvad) -> GeoIpLocation {
        let (ipv4, ipv6) = match location.ip {
            IpAddr::V4(v4) => (Some(v4), None),
            IpAddr::V6(v6) => (None, Some(v6)),
        };

        GeoIpLocation {
            ipv4,
            ipv6,
            country: location.country,
            city: location.city,
            latitude: location.latitude,
            longitude: location.longitude,
            mullvad_exit_ip: location.mullvad_exit_ip,
            hostname: None,
            bridge_hostname: None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_haversine_dist_deg() {
        use super::haversine_dist_deg;
        assert_eq!(
            haversine_dist_deg(36.12, -86.67, 33.94, -118.4),
            2887.2599506071111
        );
        assert_eq!(
            haversine_dist_deg(90.0, 5.0, 90.0, 79.0),
            0.0000000000004696822692507987
        );
        assert_eq!(haversine_dist_deg(0.0, 0.0, 0.0, 0.0), 0.0);
        assert_eq!(haversine_dist_deg(49.0, 12.0, 49.0, 12.0), 0.0);
        assert_eq!(haversine_dist_deg(6.0, 27.0, 7.0, 27.0), 111.22634257109462);
        assert_eq!(
            haversine_dist_deg(0.0, 179.5, 0.0, -179.5),
            111.22634257109495
        );
    }
}
