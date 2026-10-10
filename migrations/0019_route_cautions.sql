-- A small, published projection of hiking-relevant OSM tags. The API must not
-- depend on disposable raw staging tables.
ALTER TABLE trails ADD COLUMN cautions JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE trails ADD CONSTRAINT trails_cautions_array CHECK (jsonb_typeof(cautions) = 'array');
