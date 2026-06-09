# Rusty Quest Makepad Validation

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

The validation gate checks the local bundle model and camera-shell adapter,
asks `rusty-makepad` to validate and resolve the settings surface/profile,
asks `rusty-quest` to generate a dry-run property write plan, and scans for
legacy naming.

The profile validation runs through
`tools\Build-QuestMakepadRuntimeBundle.ps1`, which also checks that each Quest
property set operation references an effective setting and carries the same
value. This keeps profile, property, and app readback preparation on one
deterministic surface instead of a hand-edited ADB launch sequence.
