BEGIN;

CREATE TABLE IF NOT EXISTS commodity_discounts (
    commodity_id UUID PRIMARY KEY REFERENCES commodities (id) ON DELETE CASCADE,
    is_enabled BOOLEAN NOT NULL DEFAULT false,
    percentage INTEGER,
    updated_by_admin UUID REFERENCES admins (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT commodity_discounts_percentage_range CHECK (
        percentage IS NULL OR (percentage BETWEEN 1 AND 10)
    )
);

CREATE TABLE IF NOT EXISTS discount_codes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code VARCHAR(32) NOT NULL UNIQUE,
    station_id UUID NOT NULL REFERENCES stations (id) ON DELETE CASCADE,
    commodity_id UUID NOT NULL REFERENCES commodities (id) ON DELETE CASCADE,
    created_price INTEGER NOT NULL CHECK (created_price >= 0),
    discount_percentage INTEGER NOT NULL CHECK (discount_percentage BETWEEN 1 AND 10),
    discounted_price INTEGER NOT NULL CHECK (discounted_price >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    redeemed_at TIMESTAMPTZ,
    redeemed_by_station_id UUID REFERENCES stations (id)
);

CREATE INDEX IF NOT EXISTS idx_discount_codes_station_id ON discount_codes (station_id);
CREATE INDEX IF NOT EXISTS idx_discount_codes_commodity_id ON discount_codes (commodity_id);
CREATE INDEX IF NOT EXISTS idx_discount_codes_expires_at ON discount_codes (expires_at);

CREATE TABLE IF NOT EXISTS discount_code_generation_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code_id UUID NOT NULL REFERENCES discount_codes (id) ON DELETE CASCADE,
    station_id UUID NOT NULL REFERENCES stations (id) ON DELETE CASCADE,
    ip_address VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_discount_gen_logs_lookup
    ON discount_code_generation_logs (ip_address, station_id, created_at);

COMMIT;
