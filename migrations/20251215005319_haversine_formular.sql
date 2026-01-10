-- Add migration script here
-- Haversine Function: Calculates distance between two points in kilometers.
CREATE OR REPLACE FUNCTION haversine(
        lat1 float,
        lon1 float,
        lat2 float,
        lon2 float
    ) RETURNS float AS $$
DECLARE R float := 6371.0;
-- Earth's radius in km
dLat float := radians(lat2 - lat1);
dLon float := radians(lon2 - lon1);
a float;
c float;
BEGIN -- Haversine formula (a)
a := sin(dLat / 2) * sin(dLat / 2) + cos(radians(lat1)) * cos(radians(lat2)) * sin(dLon / 2) * sin(dLon / 2);
-- Angular distance (c)
c := 2 * atan2(sqrt(a), sqrt(1 - a));
-- Final distance
RETURN R * c;
END;
$$ LANGUAGE plpgsql IMMUTABLE;