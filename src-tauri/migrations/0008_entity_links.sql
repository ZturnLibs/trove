-- Stage v1.2: entity_links uniqueness for the source/association layer.

-- Deduplicate existing rows before enforcing uniqueness (keep earliest id).
DELETE FROM entity_links WHERE id NOT IN (
  SELECT MIN(id) FROM entity_links
  GROUP BY source_type, source_id, target_type, target_id, link_kind
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_entity_links_pair
  ON entity_links (source_type, source_id, target_type, target_id, link_kind);
