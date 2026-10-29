-- The forge's attestation id (a text: not every forge numbers them) joins an attestation to the
-- `handed_to_cd` event of its pull request. One row per (pull request, attestation id).
ALTER TABLE attestations ADD COLUMN attestation_id TEXT;
CREATE UNIQUE INDEX attestations_pr_id ON attestations(pr_number, attestation_id) WHERE attestation_id IS NOT NULL;
