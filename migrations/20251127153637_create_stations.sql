-- Add migration script here
CREATE TABLE IF NOT EXISTS stations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    address VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    phone VARCHAR(255) NOT NULL,
    password VARCHAR(255) NOT NULL,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    role VARCHAR NOT NULL DEFAULT 'station',
    distance DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    is_logged_in BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);
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
        'a0e3650f-e220-410c-9774-7833a4c04961',
        'Oando',
        'Lifecamp',
        'oando@gmail.com',
        '08045643423',
        'thepasss',
        9.01,
        7.42
    );