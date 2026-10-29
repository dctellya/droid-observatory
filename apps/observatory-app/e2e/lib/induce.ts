/**
 * Proof knob for the first-failure evidence and the rerun policy: `E2E_INDUCE_FAILURE=<name>` makes the first case of the
 * spec that calls this with `<name>` fail on the first attempt only, so the rerun is green and the run is `flaked`.
 * Never set in CI or by `make e2e` itself.
 */
export function induceFailure(name: string): void {
  if (process.env.E2E_INDUCE_FAILURE === name && (process.env.E2E_ATTEMPT ?? '1') === '1') {
    throw new Error(`induced failure of ${name} (E2E_INDUCE_FAILURE, attempt 1 only)`);
  }
}
