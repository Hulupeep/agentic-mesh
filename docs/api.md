# AMP Kernel API

The AMP kernel provides a REST API for executing plans and managing execution state.

## API Endpoints

### Execute Plan
```
POST /v1/plan/execute
```

Execute a plan and return a plan ID for tracking.

Request body:
```json
{
  "plan": { /* Plan IR */ },
  "inputs": { /* Optional input variables */ }
}
```

Response:
```json
{
  "plan_id": "uuid-of-execution",
  "stream_url": "/v1/trace/{plan_id}",
  "status": "pending|completed|error"
}
```

### Get Trace
```
GET /v1/trace/{plan_id}
```

Stream trace events for a plan execution as NDJSON.

Response (streamed NDJSON):
```json
{"plan_id":"...","step_id":"...","ts":"...","event_type":"step_start",...}
{"plan_id":"...","step_id":"...","ts":"...","event_type":"tool_invoke",...}
{"plan_id":"...","step_id":"...","ts":"...","event_type":"step_end",...}
```

### Create Replay Bundle
```
POST /v1/replay/bundle
```

Create a tar.gz bundle of plan, toolspecs, and traces for replay.

Request body:
```json
{
  "plan_id": "uuid-of-execution"
}
```

Response: Binary stream of tar.gz file

## Authentication

The API does not require authentication by default. In production deployments, authentication should be added via reverse proxy.

## OpenAPI Specification

```yaml
openapi: 3.0.0
info:
  title: Agentic Mesh Protocol Kernel API
  version: 0.1.0
  description: API for executing AMP plans and managing tool orchestration
paths:
  /v1/plan/execute:
    post:
      summary: Execute a plan
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                plan:
                  $ref: '#/components/schemas/Plan'
                inputs:
                  type: object
                  additionalProperties: true
      responses:
        '200':
          description: Plan execution started
          content:
            application/json:
              schema:
                type: object
                properties:
                  plan_id:
                    type: string
                  stream_url:
                    type: string
                  status:
                    type: string
  /v1/trace/{plan_id}:
    get:
      summary: Stream execution trace
      parameters:
        - name: plan_id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Stream of trace events
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Trace'
  /v1/replay/bundle:
    post:
      summary: Create replay bundle
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                plan_id:
                  type: string
      responses:
        '200':
          description: Bundle as tar.gz
          content:
            application/gzip:
              schema:
                type: string
                format: binary
components:
  schemas:
    Plan:
      $ref: 'https://raw.githubusercontent.com/acme/amp/main/schemas/Plan.schema.json'
    Trace:
      $ref: 'https://raw.githubusercontent.com/acme/amp/main/schemas/Trace.schema.json'
```