-- v1.3: memory wikilink mention use count for QuickWindow ranking.

ALTER TABLE memories ADD COLUMN mention_use_count INTEGER NOT NULL DEFAULT 0;
