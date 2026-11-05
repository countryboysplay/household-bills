-- Archived bills used to leave under-funded occurrences in the plan, and the scheduler
-- marked under-funded occurrences 'partial' (a status meant for partially *paid* bills).
DELETE FROM bill_allocations WHERE bill_occurrence_id IN (
  SELECT o.id FROM bill_occurrences o JOIN bill_templates t ON t.id=o.bill_template_id
  WHERE t.is_active=0 AND o.status IN ('upcoming','scheduled','partial','late'));
DELETE FROM bill_occurrences WHERE status IN ('upcoming','scheduled','partial','late')
  AND bill_template_id IN (SELECT id FROM bill_templates WHERE is_active=0)
  AND NOT EXISTS(SELECT 1 FROM payments p WHERE p.bill_occurrence_id=bill_occurrences.id);
UPDATE bill_occurrences SET status='skipped',updated_at=CURRENT_TIMESTAMP
  WHERE status IN ('upcoming','scheduled','partial','late')
  AND bill_template_id IN (SELECT id FROM bill_templates WHERE is_active=0);
UPDATE bill_occurrences SET status='scheduled',updated_at=CURRENT_TIMESTAMP
  WHERE status='partial' AND NOT EXISTS(SELECT 1 FROM payments p WHERE p.bill_occurrence_id=bill_occurrences.id);
