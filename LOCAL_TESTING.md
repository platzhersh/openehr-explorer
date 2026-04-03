# Local Testing with EHRBase

This guide explains how to run a local EHRBase instance for testing the openEHR Explorer app.

## Prerequisites

- [Docker](https://www.docker.com/get-started) installed and running
- [Docker Compose](https://docs.docker.com/compose/install/) (usually included with Docker Desktop)

## Quick Start

### 1. Start EHRBase

```bash
docker-compose up -d
```

This will start:
- **PostgreSQL** database on port `5442`
- **EHRBase** server on port `8080`

### 2. Wait for Services to be Ready

Check if services are healthy:

```bash
docker-compose ps
```

Wait until both services show "healthy" status. EHRBase takes ~60 seconds to initialize.

You can also follow the logs:

```bash
docker-compose logs -f ehrbase
```

Press `Ctrl+C` to stop following logs.

### 3. Verify EHRBase is Running

```bash
curl http://localhost:8080/ehrbase/rest/openehr/v1/definition/template
```

You should receive a JSON response (likely empty array `[]` initially).

### 4. Configure openEHR Explorer

In the openEHR Explorer app, add a new server profile with these settings:

- **Name**: `Local EHRBase`
- **Base URL**: `http://localhost:8080/ehrbase`
- **Server Type**: `EHRBase`
- **Auth Method**: `Basic Auth`
  - **Username**: `ehrbase-user`
  - **Password**: `ehrbase-password`

For admin operations (template upload), use:
- **Username**: `ehrbase-admin`
- **Password**: `ehrbase-admin-password`

## Managing the Local Environment

### Stop EHRBase

```bash
docker-compose stop
```

### Start Again (keeps data)

```bash
docker-compose start
```

### Stop and Remove (deletes all data)

```bash
docker-compose down -v
```

**Note**: The `-v` flag removes volumes, which means all EHRs, compositions, and templates will be deleted.

### View Logs

```bash
# All services
docker-compose logs -f

# EHRBase only
docker-compose logs -f ehrbase

# Database only
docker-compose logs -f ehrbase-db
```

## Credentials Reference

### Regular User (Read/Write)
- **Username**: `ehrbase-user`
- **Password**: `ehrbase-password`
- **Permissions**: Create EHRs, compositions, execute queries

### Admin User (Full Access)
- **Username**: `ehrbase-admin`
- **Password**: `ehrbase-admin-password`
- **Permissions**: All regular permissions + template upload/management

## API Endpoints

Once running, EHRBase provides these REST endpoints at `http://localhost:8080/ehrbase/rest/openehr/v1/`:

- `/ehr` — EHR management
- `/ehr/{ehr_id}/composition` — Composition CRUD
- `/definition/template/adl1.4` — Template management (ADL 1.4)
- `/query/aql` — AQL query execution

Test the API:
```bash
curl -u ehrbase-user:ehrbase-password http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4
```

Full API documentation: [EHRBase REST API](https://ehrbase.readthedocs.io/en/latest/03_development/04_rest_api/index.html)

## Troubleshooting

### Port Already in Use

If port 8080 is already in use, edit `docker-compose.yml` and change the port mapping:

```yaml
ports:
  - "8081:8080"  # Changed from 8080:8080
```

Then update the Base URL in openEHR Explorer to `http://localhost:8081/ehrbase`.

### Services Not Healthy

Check logs for errors:

```bash
docker-compose logs ehrbase
docker-compose logs ehrbase-db
```

Common issues:
- Database not ready: Wait longer (~60s) or check `ehrbase-db` logs
- Port conflicts: See "Port Already in Use" above

### Reset Everything

To completely reset the environment:

```bash
docker-compose down -v
docker-compose up -d
```

This removes all data and starts fresh.
