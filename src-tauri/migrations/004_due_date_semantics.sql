-- Hotfix 3.5: future unpaid monthly occurrences are regenerated so the
-- contractual due date remains the actual calendar due date while
-- latest_payment_date carries the conservative prior-business-day deadline.
DELETE FROM bill_occurrences
WHERE id IN (
  SELECT o.id
  FROM bill_occurrences o
  JOIN bill_templates t ON t.id = o.bill_template_id
  WHERE t.recurrence_type = 'monthly'
    AND o.status IN ('upcoming','scheduled','late')
    AND NOT EXISTS (SELECT 1 FROM payments p WHERE p.bill_occurrence_id = o.id)
);
