# Lab 03 — scheduled exchange rates

Runs the [lab02](../lab02.nu) currency script on a schedule inside a container:

- **daily** at 06:00: MDL → EUR for the previous day
- **weekly** on Friday at 17:00: MDL → USD for the previous 7 days

The image is based on Fedora, schedules with [supercronic](https://github.com/aptible/supercronic),
and is published to `harbor.nickmessing.com/public/automation-lab03`.
It works with both Podman and Docker.

## What changed from the assignment

| Assignment | Here | Why |
|---|---|---|
| `Dockerfile`, Ubuntu/Python base | `Containerfile`, `fedora:44` | Podman-first; the same file builds with Docker |
| `currency_exchange_rate.py` copied into `lab03/` | `lab02.nu` reused from the repo root | One copy of the script; the build context is the repo root |
| cron + `entrypoint.sh` | supercronic + `scheduler.nu` | Runs in the foreground, keeps the environment, logs to stdout, runs as non-root |
| `cronjob` file | crontab generated from environment variables | Schedules and jobs can change without rebuilding (see [Configuration](#configuration)) |
| `docker-compose.yml` | `compose.yaml` | The name both `podman compose` and `docker compose` look for first |
| `/var/log/cron.log` | `/var/log/lab03/cron.log` + `podman logs` | A directory can be a volume, a single file can't |

## Project structure

```
automation/
├── lab02.nu                  # currency script from lab02, copied into the image as-is
├── .containerignore          # the build context is the repo root, only send what the image needs
├── .dockerignore -> .containerignore
├── .github/workflows/
│   └── lab03.yml             # builds and pushes the image to Harbor on pushes to main
└── lab03/
    ├── Containerfile         # Fedora + pinned nushell and supercronic, non-root user, healthcheck
    ├── compose.yaml          # build and run with podman compose / docker compose
    ├── scheduler.nu          # entrypoint: generates the crontab, runs jobs, starts supercronic
    └── readme.md
```

Inside the image:

| Path | What |
|---|---|
| `/app/lab02.nu`, `/app/lab03/scheduler.nu` | The scripts, in the same layout as the repo |
| `/var/lib/rates/data/` | Fetched rates as JSON (volume) |
| `/var/lib/rates/error.log` | API and validation errors from lab02 (volume) |
| `/var/log/lab03/cron.log` | Output of every job run (volume) |
| `/home/rates/crontab` | Crontab generated at startup |

## Running it

All `build` commands run from the **repo root**, because the image needs `lab02.nu`.
All `compose` commands run from **`lab03/`**.

### Podman

```sh
# pull the published image
podman pull harbor.nickmessing.com/public/automation-lab03:latest

# or build it locally (from the repo root)
# --format docker keeps the HEALTHCHECK, which the default OCI format drops
podman build --format docker -f lab03/Containerfile -t harbor.nickmessing.com/public/automation-lab03:latest .

# run
podman run -d --name rates \
  -v rates-data:/var/lib/rates \
  -v rates-logs:/var/log/lab03 \
  harbor.nickmessing.com/public/automation-lab03:latest
```

With compose (needs `podman-compose` or `docker-compose` installed as the provider):

```sh
cd lab03
podman compose up -d --build
podman compose down        # add -v to also delete the volumes
```

### Docker

```sh
docker pull harbor.nickmessing.com/public/automation-lab03:latest

# or build it locally (from the repo root)
docker build -f lab03/Containerfile -t harbor.nickmessing.com/public/automation-lab03:latest .

docker run -d --name rates \
  -v rates-data:/var/lib/rates \
  -v rates-logs:/var/log/lab03 \
  harbor.nickmessing.com/public/automation-lab03:latest
```

```sh
cd lab03
docker compose up -d --build
docker compose down
```

### Bind mounts instead of volumes

The container runs as uid `10001`, so a host directory must be writable by it.
With rootless Podman, add `:U` to have Podman fix the ownership:

```sh
mkdir -p ./rates
podman run -d --name rates -v ./rates:/var/lib/rates:U,Z harbor.nickmessing.com/public/automation-lab03:latest
```

With Docker, `sudo chown 10001:10001 ./rates` first.

## Checking that it works

The examples use `podman`; `docker` takes the same arguments.
With compose, use `podman compose logs` / `podman compose exec rates …` instead.

```sh
# startup output: the generated crontab, the RUN_ON_START runs, then supercronic
podman logs -f rates

# the log file the jobs write to
podman exec rates tail -f /var/log/lab03/cron.log

# the configured jobs and the crontab they produce
podman exec rates nu /app/lab03/scheduler.nu jobs
podman exec rates nu /app/lab03/scheduler.nu crontab

# run a job right now instead of waiting for its schedule
podman exec rates nu /app/lab03/scheduler.nu run weekly

# the fetched data and any errors
podman exec rates ls /var/lib/rates/data
podman exec rates cat /var/lib/rates/error.log

# health: "healthy" once supercronic is up
podman healthcheck run rates && echo healthy
podman ps --filter name=rates
```

To see the scheduler itself fire, run the daily job every minute:

```sh
podman run -d --name rates-test -e DAILY_SCHEDULE="* * * * *" harbor.nickmessing.com/public/automation-lab03:latest
podman logs -f rates-test   # a new "[daily]" run appears at the start of every minute
```

A run in the log looks like this:

```
2026-09-28T12:34:00+03:00 [daily] lab02 rate MDL EUR 2026-09-27
1 MDL = 0.04947 EUR on 2026-09-27
saved to /var/lib/rates/data/MDL-EUR-2026-09-27.json
2026-09-28T12:34:00+03:00 [daily] exit 0
```

## Configuration

Everything is set with environment variables (`-e` / `environment:` in compose), so no rebuild is needed.

| Variable | Default | Meaning |
|---|---|---|
| `JOBS` | `daily weekly` | Jobs to schedule, separated by spaces or commas |
| `<NAME>_SCHEDULE` | see below | Cron expression (5 fields; supercronic also accepts 6/7 with seconds/year) |
| `<NAME>_BASE` | see below | Currency to convert from |
| `<NAME>_QUOTE` | see below | Currency to convert to |
| `<NAME>_DAYS` | `1` | How many days to fetch. `1` fetches a single date; more fetches a range |
| `<NAME>_OFFSET` | `1` | How many days ago the period ends. `1` = yesterday, `0` = today |
| `TZ` | `Europe/Chisinau` | Timezone for both the schedules and the dates |
| `RUN_ON_START` | `true` | Run every job once when the container starts |
| `LOG_FILE` | `/var/log/lab03/cron.log` | Where job output is appended |
| `LAB02_ROOT` | `/var/lib/rates` | Where lab02 writes `data/` and `error.log` |

`<NAME>` is the job name in upper case. `daily` and `weekly` have built-in defaults, so they
work with nothing set:

| Job | Schedule | Pair | Days | Offset | Fetches (for a run on Fri 2026-10-02) |
|---|---|---|---|---|---|
| `daily` | `0 6 * * *` | MDL → EUR | 1 | 1 | 2026-10-01 |
| `weekly` | `0 17 * * 5` | MDL → USD | 7 | 1 | 2026-09-25 … 2026-10-01 |

Any other job needs `_SCHEDULE`, `_BASE` and `_QUOTE`. For example, the previous 30 days
on the 1st of every month, plus a check at noon every weekday:

```sh
podman run -d --name rates \
  -e JOBS="daily weekly monthly workdays" \
  -e MONTHLY_SCHEDULE="0 7 1 * *" -e MONTHLY_BASE=MDL -e MONTHLY_QUOTE=EUR -e MONTHLY_DAYS=30 \
  -e WORKDAYS_SCHEDULE="0 12 * * 1-5" -e WORKDAYS_BASE=EUR -e WORKDAYS_QUOTE=USD -e WORKDAYS_OFFSET=0 \
  harbor.nickmessing.com/public/automation-lab03:latest
```

`compose.yaml` spells out the defaults and defines a `monthly` job; add it to `JOBS` to enable it.
A bad value (unknown job, missing schedule, non-numeric days) stops the container at startup
with a message saying which variable to fix.

## CI

[`.github/workflows/lab03.yml`](../.github/workflows/lab03.yml) runs on pushes to `main` that touch
`lab02.nu`, `lab03/` or the workflow. It installs Podman on `ubuntu-latest`, builds for
`linux/amd64`, and pushes `:<commit sha>` and `:latest` to
`harbor.nickmessing.com/public/automation-lab03`. It logs in with the `HARBOR_USERNAME` and
`HARBOR_PASSWORD` secrets from the `Production` environment.

## Notes

- **Pinned versions.** nushell `0.113.1` and supercronic `v0.2.49` are downloaded with SHA256 checks.
  To upgrade, change the `ARG`s at the top of the `Containerfile`.
- **Missed runs are skipped.** If the container is down at 06:00, that day is not fetched later.
  `RUN_ON_START=true` covers the common case of a restart.
- **No graph in logs.** lab02 only draws its range graph in an interactive terminal:
  `podman exec -it rates nu -c "use /app/lab02.nu; lab02 rate MDL USD --from 2026-01-01"`
  draws it.
