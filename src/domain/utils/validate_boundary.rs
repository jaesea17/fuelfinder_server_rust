use crate::domain::utils::errors::station_errors::StationError;

pub const ABUJA_MIN_LAT: f64 = 8.25;
pub const ABUJA_MAX_LAT: f64 = 9.30;
pub const ABUJA_MIN_LON: f64 = 6.75;
pub const ABUJA_MAX_LON: f64 = 7.75;

pub fn validate_abuja_bounds(lat: f64, lon: f64) -> Result<(), StationError> {
    let in_lat_range = (ABUJA_MIN_LAT..=ABUJA_MAX_LAT).contains(&lat);
    let in_lon_range = (ABUJA_MIN_LON..=ABUJA_MAX_LON).contains(&lon);

    if in_lat_range && in_lon_range {
        Ok(())
    } else {
        Err(StationError::WrongCredentials(
            "Location is outside Abuja service area".to_string(),
        ))
    }
}
