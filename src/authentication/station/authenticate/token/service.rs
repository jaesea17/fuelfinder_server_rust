use chrono::{Duration, Utc};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};

use crate::authentication::station::authenticate::dto::StationResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    // Standard Claims
    pub exp: usize, // Expiration time (as timestamp)
    pub iat: usize, // Issued at time (as timestamp)

    // Custom Claim (your payload)
    pub station_res: StationResponse,
}

// Configuration struct to hold key and algorithm
pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    header: Header,
}

impl TokenService {
    pub fn new(secret: &str) -> Self {
        Self {
            // Use HMAC (HS256) and create the key from your secret string
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            validation: Validation::default(),
            header: Header::new(Algorithm::HS256),
        }
    }

    /// Creates and signs a new JWT for the given station ID.
    pub fn create_token(
        &self,
        station_res: StationResponse,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        // Define issuance and expiration times
        let now = Utc::now();
        let iat = now.timestamp() as usize;
        // Token expires 24 hours from now
        let expiration = now + Duration::hours(24);
        let exp = expiration.timestamp() as usize;

        // 1. Create the Claims payload
        let claims = Claims {
            exp,
            iat,
            station_res,
        };

        // 2. Encode the claims using the key and header
        encode(&self.header, &claims, &self.encoding_key)
    }

    pub fn decode(&self, token: String) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
    }
}
#[derive(Debug, Serialize)]

pub struct ApiMessage {
    pub access_token: String,
}
