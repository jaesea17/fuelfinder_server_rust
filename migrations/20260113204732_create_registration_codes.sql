CREATE TABLE IF NOT EXISTS registration_codes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    station_id UUID,
    code VARCHAR(255) NOT NULL,
    is_valid BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT now() -- Optional but highly recommended
);