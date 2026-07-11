# Live-Cloud CI — Setup Guide

This document explains the `live-cloud` CI job in `.github/workflows/ci.yml`,
which secrets to add, and exactly what runs NOW vs what runs only after secrets
are configured.

---

## Honest Status

**As of the time this file was written, no cloud secrets have been added to
this repository.**

That means:

- Every step in the `live-cloud` job prints a `SKIP:` line and exits 0 (green).
- The `staging-deploy` job builds and probes the `hello` example against
  `localhost` and passes — NO cloud credentials needed.
- **The real AWS S3 / Redis / SMTP round-trips have NOT run against live cloud
  endpoints yet.**  They will run automatically on the next CI push once the
  secrets below are added.

---

## What the `live-cloud` Job Does

The job runs on every push to `main`.  For each service:

1. It exposes the relevant secret(s) as an environment variable.
2. The test checks whether the variable is non-empty.
3. **Secret absent** → prints `SKIP:` and exits 0 (green, no failure).
4. **Secret present** → runs the real integration round-trip.

### Services covered

| Service | Test function | Key secret(s) |
|---------|--------------|---------------|
| Redis pub/sub | `rf_cache::pubsub::tests::test_pubsub_publish_subscribe_roundtrip` | `REDIS_URL` |
| AWS S3 | `rf_storage::s3::tests::test_real_aws_s3_operations` | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `S3_TEST_BUCKET` |
| SMTP / SES | `rf_mail::backends::smtp::tests::test_smtp_send_via_mailhog` | `RF_SMTP_TEST_ADDR` |

---

## Secrets to Add

Go to **GitHub → Repository → Settings → Secrets and variables → Actions →
New repository secret** and add each of the following.

### Redis (`REDIS_URL`)

```
Name:  REDIS_URL
Value: redis://your-redis-host:6379
```

Supported URL forms:
- `redis://host:port` — plain TCP (e.g. AWS ElastiCache without TLS)
- `rediss://host:port` — TLS (e.g. Upstash, ElastiCache with TLS enabled)
- `redis://:password@host:port` — with AUTH password

The test uses `REDIS_URL` if set, otherwise falls back to `redis://127.0.0.1:6379`
(the local Docker Compose stack from `scripts/test-env-up.sh`).

---

### AWS S3 (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `S3_TEST_BUCKET`)

Create an IAM user or role with the following minimum policy on the test bucket:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:DeleteObject",
        "s3:HeadObject",
        "s3:ListBucket"
      ],
      "Resource": [
        "arn:aws:s3:::YOUR_BUCKET_NAME",
        "arn:aws:s3:::YOUR_BUCKET_NAME/*"
      ]
    }
  ]
}
```

Add four secrets:

```
AWS_ACCESS_KEY_ID      = AKIA...
AWS_SECRET_ACCESS_KEY  = wJal...
AWS_REGION             = us-east-1          (or your bucket's region)
S3_TEST_BUCKET         = my-rustforge-test  (pre-existing bucket)
```

The test writes and immediately deletes `rf_live_test/<pid>/smoke.txt` — no
permanent objects are created.

---

### SMTP / SES (`RF_SMTP_TEST_ADDR`)

The test sends one email via plain SMTP.  Point it at any relay that accepts
unauthenticated or password-authenticated connections on port 25/465/587.

#### AWS SES (recommended)

1. Verify a sender address or domain in SES.
2. Create SMTP credentials (IAM → Users → Security credentials → SMTP credentials).
3. Set the secret:

```
RF_SMTP_TEST_ADDR = email-smtp.us-east-1.amazonaws.com:587
```

For SES, also set the SMTP username/password.  The test currently uses the
`RF_SMTP_TEST_ADDR` hook in `rf-mail`'s SMTP backend — extend the backend or
the test with `RF_SMTP_USERNAME` / `RF_SMTP_PASSWORD` env vars if your relay
requires authentication (PRs welcome).

#### MailHog / local relay (for maintainer dev)

```
RF_SMTP_TEST_ADDR = 127.0.0.1:1025
```

No secret needed locally if you run `./scripts/test-env-up.sh` first (starts
MailHog on 1025).

---

## What the `staging-deploy` Job Does

This job **does not need any cloud secrets**.  It:

1. Runs `cargo build --release -p hello` on the CI Ubuntu runner.
2. Starts the binary with `SERVER_PORT=3737`.
3. Waits up to 30 seconds for port 3737 to open.
4. `curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:3737/health` → asserts `200`.
5. `curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:3737/` → asserts `200`.
6. Kills the server and exits.

The full logic is in `scripts/deploy-smoke.sh`.  Run it locally with:

```bash
./scripts/deploy-smoke.sh
```

The script also accepts `SMOKE_PORT` and `SMOKE_WAIT` overrides:

```bash
SMOKE_PORT=8888 SMOKE_WAIT=60 ./scripts/deploy-smoke.sh
```

### Proven NOW vs Pending Secrets

| Check | Status |
|-------|--------|
| `staging-deploy` — release binary builds, starts, serves 200 | **Proven on every CI run** |
| `live-cloud` — Redis round-trip | **Pending** — add `REDIS_URL` secret |
| `live-cloud` — S3 round-trip | **Pending** — add AWS secrets |
| `live-cloud` — SMTP/SES round-trip | **Pending** — add `RF_SMTP_TEST_ADDR` secret |
| `live-cloud` — skip path is green | **Proven on every CI run** (secrets absent = skip = 0) |

---

## Running the Live-Cloud Tests Locally

Export the secrets as environment variables, then run the specific test:

```bash
# Redis
export REDIS_URL=redis://your-host:6379
cargo test -p rf-cache --features redis-backend \
  test_pubsub_publish_subscribe_roundtrip -- --nocapture

# S3
export AWS_ACCESS_KEY_ID=AKIA...
export AWS_SECRET_ACCESS_KEY=wJal...
export AWS_REGION=us-east-1
export S3_TEST_BUCKET=my-rustforge-test
cargo test -p rf-storage test_real_aws_s3_operations -- --nocapture

# SMTP
export RF_SMTP_TEST_ADDR=email-smtp.us-east-1.amazonaws.com:587
cargo test -p rf-mail test_smtp_send_via_mailhog -- --nocapture
```

Or run the deploy-smoke probe locally (no secrets needed):

```bash
./scripts/deploy-smoke.sh
```
