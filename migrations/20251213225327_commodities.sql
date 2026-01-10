-- Add migration script here
CREATE TABLE IF NOT EXISTS commodities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    is_available BOOLEAN NOT NULL DEFAULT false,
    price INTEGER NOT NULL DEFAULT 0,
    station_id UUID NOT NULL,
    "created_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_station FOREIGN KEY (station_id) REFERENCES stations (id) ON DELETE CASCADE
);
INSERT INTO commodities (name, price, station_id)
VALUES (
        'Petrol',
        930,
        'a0e3650f-e220-410c-9774-7833a4c04961'
    );