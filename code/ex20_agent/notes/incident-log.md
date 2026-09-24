# Incident Log

## 2026-09-18 - Deploy failed

Cause: the deploy service was pointed at the wrong port after a config change.
Fix: corrected `service.toml` and redeployed.

## 2026-08-02 - Staging smoke tests flaky

Cause: the health check interval was too short.
Fix: raised `interval_seconds` to 30.
