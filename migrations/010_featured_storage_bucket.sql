-- 010_featured_storage_bucket.sql — public Supabase Storage bucket for featured carousel images.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.schemata WHERE schema_name = 'storage'
    ) THEN
        INSERT INTO storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
        VALUES (
            'featured',
            'featured',
            true,
            5242880,
            ARRAY['image/jpeg', 'image/png', 'image/webp', 'image/gif']::text[]
        )
        ON CONFLICT (id) DO UPDATE SET
            public = EXCLUDED.public,
            file_size_limit = EXCLUDED.file_size_limit,
            allowed_mime_types = EXCLUDED.allowed_mime_types;

        IF NOT EXISTS (
            SELECT 1 FROM pg_policies
            WHERE schemaname = 'storage'
              AND tablename = 'objects'
              AND policyname = 'Public read featured images'
        ) THEN
            CREATE POLICY "Public read featured images"
                ON storage.objects
                FOR SELECT
                USING (bucket_id = 'featured');
        END IF;
    END IF;
END $$;