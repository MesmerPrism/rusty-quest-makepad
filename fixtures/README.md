# Rusty Quest Makepad Fixtures

- `settings/`: Quest Makepad app settings surfaces.
- `profiles/`: settings profiles, Quest runtime profiles, and profile bundles.
- `effective-settings/`: representative resolver output consumed by app
  adapter unit tests. Regenerate the profile output through
  `tools/Build-QuestMakepadRuntimeBundle.ps1` and copy the generated
  `local-artifacts/quest-makepad-runtime-bundle/effective-settings.json` here
  when the committed surface/profile changes.
- `lattice/`: Lattice display view sets consumed by app adapter unit tests.
- `mesh-replay/`: public synthetic Matter mesh surface sequence consumed by the
  Quest Makepad mesh replay adapter.
- `damaged/`: invalid bundles that must be rejected.
