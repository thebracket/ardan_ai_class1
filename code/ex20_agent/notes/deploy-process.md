# Deploy Process

We deploy the service from `main` using the deploy pipeline. A deploy goes out
to staging first, and then to production once the smoke tests pass.

The deploy service is owned by the platform team. It listens on the port
declared in `service.toml` - don't hard-code it in scripts.

If a deploy fails, check `incident-log.md` for the last known cause before
paging anyone.
