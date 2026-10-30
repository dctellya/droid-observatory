-- A `gate` rework window (F-028, FR-G7) belongs to a pull request and to the check that went red:
-- the green run of the same check closes it, another check's does not. `feature_id` holds the card
-- (the milestone) the fix cycle returns to; the other causes leave both new columns empty.
ALTER TABLE rework_windows ADD COLUMN pr_number INTEGER;
ALTER TABLE rework_windows ADD COLUMN check_name TEXT;
CREATE INDEX rework_windows_gate ON rework_windows(pr_number, check_name) WHERE closed_by IS NULL;
