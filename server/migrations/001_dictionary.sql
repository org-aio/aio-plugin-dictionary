CREATE TABLE IF NOT EXISTS dictionary_types (
    tenant_id TEXT NOT NULL,
    id TEXT NOT NULL,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, id),
    UNIQUE (tenant_id, code)
);
CREATE TABLE IF NOT EXISTS dictionary_items (
    tenant_id TEXT NOT NULL,
    id TEXT NOT NULL,
    type_id TEXT NOT NULL,
    value TEXT NOT NULL,
    label TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, id),
    UNIQUE (tenant_id, type_id, value),
    FOREIGN KEY (tenant_id, type_id)
        REFERENCES dictionary_types (tenant_id, id)
        ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS dictionary_items_one_default
    ON dictionary_items (tenant_id, type_id)
    WHERE is_default;
