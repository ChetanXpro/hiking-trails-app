-- A data-only archive is restored into an already-migrated schema in one
-- transaction. Deferring the self-reference makes that restore independent of
-- the physical COPY order of parent and child region rows.
DO $migration$
DECLARE
    constraint_count INTEGER;
    constraint_name TEXT;
BEGIN
    IF to_regclass('public.regions') IS NULL THEN
        RAISE EXCEPTION 'public.regions is missing';
    END IF;

    SELECT count(*), min(constraint_entry.conname::TEXT)
    INTO constraint_count, constraint_name
    FROM pg_constraint constraint_entry
    WHERE constraint_entry.conrelid = 'public.regions'::regclass
      AND constraint_entry.contype = 'f'
      AND cardinality(constraint_entry.conkey) = 1
      AND EXISTS (
          SELECT 1
          FROM unnest(constraint_entry.conkey) AS key_column(attnum)
          JOIN pg_attribute attribute
            ON attribute.attrelid = constraint_entry.conrelid
           AND attribute.attnum = key_column.attnum
          WHERE attribute.attname = 'parent_region_id'
      );

    IF constraint_count <> 1 THEN
        RAISE EXCEPTION
            'expected exactly one regions.parent_region_id foreign key, found %',
            constraint_count;
    END IF;

    EXECUTE format(
        'ALTER TABLE public.regions ALTER CONSTRAINT %I DEFERRABLE INITIALLY DEFERRED',
        constraint_name
    );
END
$migration$;
