# Rusty Quest Makepad Validation

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

The validation gate checks the local bundle model, asks `rusty-makepad` to
validate and resolve the settings surface/profile, asks `rusty-quest` to
generate a dry-run property write plan, and scans for legacy naming.

