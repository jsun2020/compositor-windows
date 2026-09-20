export function installTestApi(api: Record<string, unknown>): void {
  if (import.meta.env.DEV || import.meta.env.VITE_TEST_API === "1") {
    (window as unknown as { __compositor: Record<string, unknown> }).__compositor = { ...((window as unknown as { __compositor?: Record<string, unknown> }).__compositor ?? {}), ...api };
  }
}
