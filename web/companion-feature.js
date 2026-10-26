// Public availability does not depend on an old private-preview preference.
// Connection and service permissions are still checked by their clients.
export function companionEnabled() { return true; }
export function installCompanionFeature(target) {
  target.ironwoodCompanionFeatureEnabled = companionEnabled;
  // Compatibility with older pop-out bundles; public availability cannot be hidden.
  target.ironwoodSetCompanionFeatureEnabled = () => {};
}
