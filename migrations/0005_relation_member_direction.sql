ALTER TABLE trails
    ADD COLUMN IF NOT EXISTS default_direction_mode TEXT NOT NULL DEFAULT 'forward',
    ADD COLUMN IF NOT EXISTS direction_source TEXT;

DO $$
BEGIN
    IF to_regclass('public.raw_relations') IS NOT NULL THEN
        ALTER TABLE raw_relations
            ADD COLUMN IF NOT EXISTS members JSONB;

        UPDATE raw_relations
        SET members = (
            SELECT COALESCE(
                jsonb_agg(
                    jsonb_build_object(
                        'type', 'w',
                        'ref', item.value::bigint,
                        'role', ''
                    )
                    ORDER BY item.ordinality
                ),
                '[]'::jsonb
            )
            FROM jsonb_array_elements_text(member_ids) WITH ORDINALITY AS item(value, ordinality)
        )
        WHERE members IS NULL;
    END IF;
END $$;
