# Tool fact for the exporter slice (D-046, disposition 5): a local OTLP collector is already running on this host

Source: owner's delegate (pre-roci), 23:50, 2026-10-29. Nothing to install.

- OTLP in: `http://localhost:4318` (http/protobuf) and `localhost:4317` (grpc). Collector health: `curl -s localhost:13133` returns 200.
- Behind it: Mimir (metrics) at `http://localhost:9009`, query API `GET /prometheus/api/v1/query?query=<name>`; Loki at 3100; Tempo at 3200; Grafana at `http://localhost:3000` (anonymous admin, datasources provisioned). Metric names arrive with the OTLP suffixes (a counter `x` reads back as `x_total`).
- Smoke-tested 21:37: one metric, one log and one trace sent to 4318 came back from all three stores.
- Stack: `/Users/dct/code/EllyaOrg/projects/dalimaray/obs-stack/compose.yaml` on the `colima` docker context (not Docker Desktop, which stays off). If a readiness check fails, `docker compose up -d` from that folder; do not change the stack's files, they are the owner's.

For the exporter slice: the live check is `POST` to 4318 and the series read back from Mimir's query API, which is the unattended local receipt D-046 asks for; the collector's own debug exporter is not needed. If the stack is unreachable from the worker's shell, that is a boundary row, not a reason to install a second collector.
