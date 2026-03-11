-- Add migration script here
BEGIN;

-- 1. Add the column as TEXT
-- 2. Add a CHECK constraint to keep data clean
ALTER TABLE stations 
    ADD COLUMN station_type TEXT NOT NULL DEFAULT 'petrol',
    ADD CONSTRAINT valid_station_type 
        CHECK (station_type IN ('petrol', 'gas'));

-- 3. Insert station first
INSERT INTO stations (
        id,
        name,
        address,
        email,
        phone,
        password,
        latitude,
        longitude
    )
VALUES (
        'a0e3650f-e220-410c-0000-7833a4c04961',
        'Papis Gas',
        'Lifecamp',
        'oando@gmail.com',
        '08045643400',
        'thepasss',
        9.05,
        7.49
    );

-- 3. Insert your commodity
INSERT INTO commodities (name, price, station_id)
VALUES (
        'Gas',
        900,
        'a0e3650f-e220-410c-0000-7833a4c04961'
    );

COMMIT;