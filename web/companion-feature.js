const key = 'ironwood.experimental.companion';
export function companionEnabled(target = globalThis.window) {
  try { return target?.localStorage.getItem(key) === 'true'; } catch { return false; }
}
export function installCompanionFeature(target) {
  target.ironwoodCompanionFeatureEnabled = () => companionEnabled(target);
  target.ironwoodSetCompanionFeatureEnabled = value => {
    try { target.localStorage.setItem(key, value ? 'true' : 'false'); } catch { return; }
    if (!value) {
      target.document.querySelectorAll('#companion-engines,#companion-engine-provider,#companion-archive,#companion-connection,#companion-add-engine,#companion-explorer-filters,#tablebase-connection').forEach(dialog => dialog.close());
    }
    target.dispatchEvent(new target.Event('ironwood-companion-feature-changed'));
  };
}
