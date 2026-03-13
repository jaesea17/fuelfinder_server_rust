BEGIN;

CREATE TABLE IF NOT EXISTS subscriptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    station_id UUID NOT NULL REFERENCES stations (id) ON DELETE CASCADE,
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL,
    status VARCHAR(32) NOT NULL CHECK (status IN ('active', 'expired', 'cancelled')),
    created_by_admin UUID REFERENCES admins (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS uniq_active_subscription_per_station
    ON subscriptions (station_id)
    WHERE status = 'active';

CREATE TABLE IF NOT EXISTS subscription_reminder_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    subscription_id UUID NOT NULL REFERENCES subscriptions (id) ON DELETE CASCADE,
    reminder_type VARCHAR(32) NOT NULL CHECK (reminder_type IN ('d7', 'd4', 'd1', 'expired')),
    sent_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (subscription_id, reminder_type)
);

CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    station_id UUID NOT NULL REFERENCES stations (id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    body TEXT NOT NULL,
    kind VARCHAR(64) NOT NULL,
    is_read BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMIT;
