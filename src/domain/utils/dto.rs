use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct StationQueryParam {
    pub longitude: String,
    pub latitude: String,
    pub station_type: String
}

#[derive(Debug, Deserialize)]
pub struct AllStationsQuery {
    pub station_type: Option<String>
}