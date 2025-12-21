# Payment Backend API Documentation

Complete API reference for the Payment Backend system.

---

## Table of Contents

- [Authentication](#authentication)
- [Rate Limiting](#rate-limiting)
- [Health Check](#health-check)
- [Accounts API](#accounts-api)
- [Transfers API](#transfers-api)
- [Webhooks API](#webhooks-api)
- [Error Codes](#error-codes)

---

## Authentication

All protected endpoints require an API key in the `Authorization` header:

```
Authorization: Bearer pk_live_your_api_key_here
```

**Example**:

```bash
curl -H "Authorization: Bearer pk_live_9a4d5dbeb9cef34e0f37075474740a65" \
  http://localhost:3000/api/v1/accounts/your-account-id
```

---

## Rate Limiting

### Limits

- **IP-based** (public endpoints): 100 requests/minute
- **API Key-based** (protected endpoints): 1000 requests/minute

### Response Headers

Every response includes rate limit information:

- `X-RateLimit-Limit`: Maximum requests allowed
- `X-RateLimit-Remaining`: Requests remaining in current window
- `X-RateLimit-Reset`: Unix timestamp when limit resets

### Rate Limit Exceeded Response

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Try again later."
  }
}
```

**HTTP Status**: `429 Too Many Requests`

---

## Health Check

### GET /health

Check if the API is running.

**Authentication**: None required

**Response**:

```json
{
  "status": "ok"
}
```

**Example**:

```bash
curl http://localhost:3000/health
```

---

## Accounts API

### POST /api/v1/accounts

Create a new account.

**Authentication**: None (Public Endpoint)

**Request Body**:

```json
{
  "business_name": "Acme Corp",
  "email": "contact@acme.com",
  "currency": "USD"
}
```

**Fields**:

- `business_name` (string, required): Business or account name
- `email` (string, required): Contact email
- `currency` (string, required): Three-letter currency code (USD, EUR, etc.)

**Response** (`201 Created`):

```json
{
  "id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "business_name": "Acme Corp",
  "email": "contact@acme.com",
  "balance": 0.0,
  "currency": "USD",
  "status": "active",
  "created_at": "2025-12-21T15:30:00Z"
}
```

**Example**:

```bash
curl -X POST 'http://localhost:3000/api/v1/accounts' \
  -H 'Content-Type: application/json' \
  -d '{
    "business_name": "Acme Corp",
    "email": "contact@acme.com",
    "currency": "USD"
  }'
```

---
    
### POST /api/v1/accounts/putbalance

**Authentication**: Required

**Request Body**:

```json
{
  "account_id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "balance": 2000.5,
  "currency": "EUR"
}
```

**Fields**:

- `account_id` (UUID, required): Account to update
- `balance` (number, required): New balance amount
- `currency` (string, required): Currency code

**Response** (`200 OK`):
Returns updated account object.

---

### GET /api/v1/accounts/:id

Get account details by ID.

**Authentication**: Required

**Path Parameters**:

- `id` (UUID): Account ID

**Response** (`200 OK`):

```json
{
  "id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "business_name": "Acme Corp",
  "email": "contact@acme.com",
  "balance": 1250.5,
  "currency": "USD",
  "status": "active",
  "created_at": "2025-12-21T15:30:00Z"
}
```

**Example**:

```bash
curl 'http://localhost:3000/api/v1/accounts/58c297a9-4dc3-451c-a8a7-1202e3031248' \
  -H 'Authorization: Bearer pk_live_xxx'
```

---

### PATCH /api/v1/accounts/:id

Update account information.

**Authentication**: Required

**Path Parameters**:

- `id` (UUID): Account ID

**Request Body** (all fields optional):

```json
{
  "business_name": "New Business Name",
  "email": "newemail@example.com",
  "status": "active"
}
```

**Fields**:

- `business_name` (string, optional): Update business name
- `email` (string, optional): Update email
- `status` (string, optional): Update status

**Response** (`200 OK`):

```json
{
  "id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "business_name": "New Business Name",
  "email": "newemail@example.com",
  "balance": 1250.5,
  "currency": "USD",
  "status": "active",
  "created_at": "2025-12-21T15:30:00Z"
}
```

**Example**:

```bash
curl -X PATCH 'http://localhost:3000/api/v1/accounts/58c297a9-4dc3-451c-a8a7-1202e3031248' \
  -H 'Authorization: Bearer pk_live_xxx' \
  -H 'Content-Type: application/json' \
  -d '{"email": "newemail@example.com"}'
```

---

### GET /api/v1/accounts/:id/balance

Get account balance with optional currency conversion.

**Authentication**: Required

**Path Parameters**:

- `id` (UUID): Account ID

**Query Parameters**:

- `currency` (string, optional): Target currency for conversion (e.g., EUR, GBP)

**Response** (`200 OK`):

```json
{
  "account_id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "balance": 1250.5,
  "currency": "USD"
}
```

**With Currency Conversion**:

```json
{
  "account_id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "balance": 1150.25,
  "currency": "EUR"
}
```

**Example**:

```bash
# Get balance in account's currency
curl 'http://localhost:3000/api/v1/accounts/58c297a9-4dc3-451c-a8a7-1202e3031248/balance' \
  -H 'Authorization: Bearer pk_live_xxx'

# Get balance converted to EUR
curl 'http://localhost:3000/api/v1/accounts/58c297a9-4dc3-451c-a8a7-1202e3031248/balance?currency=EUR' \
  -H 'Authorization: Bearer pk_live_xxx'
```

---

## Transfers API

### POST /api/v1/transfer

Create a debit, credit, or transfer transaction.

**Authentication**: Required

**Transaction Types**:

1. **Debit**: Remove money from an account
2. **Credit**: Add money to an account
3. **Transfer**: Move money between two accounts

#### Debit Transaction

Remove funds from an account.

**Request Body**:

```json
{
  "type": "debit",
  "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "amount": 50.0,
  "currency": "USD",
  "description": "Payment for services",
  "idempotency_key": "unique-key-123"
}
```

**Fields**:

- `type` (string): Must be "debit"
- `from_account` (UUID, required): Account to debit from
- `amount` (number, required): Amount to debit (must be > 0)
- `currency` (string, required): Currency code
- `description` (string, optional): Transaction description
- `idempotency_key` (string, optional): Unique key to prevent duplicate transactions

**Webhook Triggered**: `transaction.debited` sent to `from_account`'s webhooks

#### Credit Transaction

Add funds to an account.

**Request Body**:

```json
{
  "type": "credit",
  "to_account": "b163f805-401b-4a41-8afd-b2903c0c1704",
  "amount": 100.0,
  "currency": "USD",
  "description": "Refund",
  "idempotency_key": "unique-key-456"
}
```

**Fields**:

- `type` (string): Must be "credit"
- `to_account` (UUID, required): Account to credit to
- `amount` (number, required): Amount to credit (must be > 0)
- `currency` (string, required): Currency code
- `description` (string, optional): Transaction description
- `idempotency_key` (string, optional): Unique key to prevent duplicate transactions

**Webhook Triggered**: `transaction.credited` sent to `to_account`'s webhooks

#### Transfer Transaction

Move funds between two accounts.

**Request Body**:

```json
{
  "type": "transfer",
  "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "to_account": "b163f805-401b-4a41-8afd-b2903c0c1704",
  "amount": 75.0,
  "currency": "USD",
  "description": "Payment to vendor",
  "idempotency_key": "unique-key-789"
}
```

**Fields**:

- `type` (string): Must be "transfer"
- `from_account` (UUID, required): Source account
- `to_account` (UUID, required): Destination account
- `amount` (number, required): Amount to transfer (must be > 0)
- `currency` (string, required): Currency code
- `description` (string, optional): Transaction description
- `idempotency_key` (string, optional): Unique key to prevent duplicate transactions

**Webhooks Triggered**:

- `transaction.debited` sent to `from_account`'s webhooks
- `transaction.credited` sent to `to_account`'s webhooks

#### Response

**Success** (`200 OK`):

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "transaction_type": "transfer",
  "from_account_id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "to_account_id": "b163f805-401b-4a41-8afd-b2903c0c1704",
  "amount": 75.0,
  "currency": "USD",
  "status": "completed",
  "description": "Payment to vendor",
  "parent_tx_key": "txgroup_abc123xyz",
  "created_at": "2025-12-21T16:00:00Z"
}
```

**Example**:

```bash
curl -X POST 'http://localhost:3000/api/v1/transfer' \
  -H 'Authorization: Bearer pk_live_xxx' \
  -H 'Content-Type: application/json' \
  -d '{
    "type": "transfer",
    "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
    "to_account": "b163f805-401b-4a41-8afd-b2903c0c1704",
    "amount": 75.00,
    "currency": "USD",
    "description": "Payment to vendor"
  }'
```

---

### GET /api/v1/transfer/list

List transfers for an account with pagination.

**Authentication**: Required

**Query Parameters**:

- `account_id` (UUID, required): Account ID (must match authenticated account)
- `limit` (integer, optional): Number of results (default: 50, max: 100)
- `offset` (integer, optional): Pagination offset (default: 0)

**Response** (`200 OK`):

```json
{
    "transfers": [
        {
            "id": "d0d2e289-41f3-4401-af0f-4d4ebbc59751",
            "transfer_type": "transfer",
            "from_account": "e7aec6f0-0524-44f9-9058-bfc2dde986fe",
            "to_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
            "amount": 150.0,
            "currency": "USD",
            "status": "pending",
            "description": null,
            "created_at": "2025-12-21T17:28:59.724373+00:00",
            "idempotency_key": "55b4fcb9-371e-4cc8-9875-43cb69419b98",
            "parent_tx_key": "txgroup_ef4ab5d3-1771-43ad-b96a-220c4cd6d8e7"
        },
        {
            "id": "f4656bf6-1256-47a8-ba87-85ada3d64e60",
            "transfer_type": "transfer",
            "from_account": "e7aec6f0-0524-44f9-9058-bfc2dde986fe",
            "to_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
            "amount": 150.0,
            "currency": "USD",
            "status": "pending",
            "description": null,
            "created_at": "2025-12-21T15:25:00.916606+00:00",
            "idempotency_key": "9d24866f-bdf6-4da5-b304-ef7c7c338a6a",
            "parent_tx_key": "txgroup_041a9402-d099-4905-bd1a-18b55cb5d7b3"
        },
        {
            "id": "9856c360-9ff2-48d0-b2d3-085e1e05f9c2",
            "transfer_type": "transfer",
            "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
            "to_account": "e7aec6f0-0524-44f9-9058-bfc2dde986fe",
            "amount": 15.0,
            "currency": "USD",
            "status": "pending",
            "description": null,
            "created_at": "2025-12-21T15:20:53.180966+00:00",
            "idempotency_key": "408519d8-700e-4611-a6bb-2f5afa2aae4a",
            "parent_tx_key": "txgroup_75b3a9f2-2088-4928-a5ce-6ec038afa084"
        },
        {
            "id": "2145fd85-5a1b-4684-83de-0825a5b79af3",
            "transfer_type": "transfer",
            "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
            "to_account": "e7aec6f0-0524-44f9-9058-bfc2dde986fe",
            "amount": 15.0,
            "currency": "USD",
            "status": "pending",
            "description": null,
            "created_at": "2025-12-21T13:20:31.357259+00:00",
            "idempotency_key": "1159de27-273c-429d-836b-fb19333ade78",
            "parent_tx_key": "txgroup_48c6414d-581e-4bde-8f51-998a19c71161"
        },
        {
            "id": "fa9c2a69-02cb-4584-8267-cc329d54f017",
            "transfer_type": "transfer",
            "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
            "to_account": "e7aec6f0-0524-44f9-9058-bfc2dde986fe",
            "amount": 0.9,
            "currency": "inr",
            "status": "pending",
            "description": null,
            "created_at": "2025-12-21T13:18:22.975219+00:00",
            "idempotency_key": "2d25c417-a04f-4d8d-9f0f-51348d0f9b31",
            "parent_tx_key": "txgroup_92ea1e96-997f-49cf-b1b8-8d12ab8c7307"
        },
        {
            "id": "fbc814af-ee1f-4fab-bef5-004e68242ee7",
            "transfer_type": "transfer",
            "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
            "to_account": "b163f805-401b-4a41-8afd-b2903c0c1704",
            "amount": 0.9,
            "currency": "inr",
            "status": "pending",
            "description": null,
            "created_at": "2025-12-21T13:16:16.716756+00:00",
            "idempotency_key": "dc17b628-1bae-49c9-9053-c5b99be0de29",
            "parent_tx_key": "txgroup_4ad94ec9-cbce-49f9-90af-23918bf84d0d"
        },
        {
            "id": "e217cb13-b5d3-4469-9261-d2d3463eae26",
            "transfer_type": "transfer",
            "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
            "to_account": "b163f805-401b-4a41-8afd-b2903c0c1704",
            "amount": 0.9,
            "currency": "inr",
            "status": "pending",
            "description": null,
            "created_at": "2025-12-21T13:15:46.240129+00:00",
            "idempotency_key": "aa07d6d8-2c8f-4ae4-a276-ac12387bd480",
            "parent_tx_key": "txgroup_ee99fa7e-108e-42e8-8b61-d207b6741cb6"
        }
    ],
    "total": 7,
    "limit": 50,
    "offset": 0
}
```

**Example**:

```bash
curl 'http://localhost:3000/api/v1/transfer/list?account_id=58c297a9-4dc3-451c-a8a7-1202e3031248&limit=10' \
  -H 'Authorization: Bearer pk_live_xxx'
```

---

### GET /api/v1/transfer/:parent_key

Get all transactions associated with a parent transaction key.

**Authentication**: Required

**Path Parameters**:

- `parent_key` (string): Parent transaction key (e.g., "txgroup_abc123")

**Response** (`200 OK`):

```json
[
    {
        "id": "fbc814af-ee1f-4fab-bef5-004e68242ee7",
        "transfer_type": "transfer",
        "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
        "to_account": "b163f805-401b-4a41-8afd-b2903c0c1704",
        "amount": 0.9,
        "currency": "inr",
        "status": "pending",
        "description": null,
        "created_at": "2025-12-21T13:16:16.716756+00:00",
        "idempotency_key": "dc17b628-1bae-49c9-9053-c5b99be0de29",
        "parent_tx_key": "txgroup_4ad94ec9-cbce-49f9-90af-23918bf84d0d"
    },
    {
        "id": "82486e1c-903a-4b4a-a893-219804ff4283",
        "transfer_type": "debit",
        "from_account": "58c297a9-4dc3-451c-a8a7-1202e3031248",
        "to_account": null,
        "amount": 0.9,
        "currency": "inr",
        "status": "pending",
        "description": null,
        "created_at": "2025-12-21T13:16:16.716756+00:00",
        "idempotency_key": "dc17b628-1bae-49c9-9053-c5b99be0de29_debit",
        "parent_tx_key": "txgroup_4ad94ec9-cbce-49f9-90af-23918bf84d0d"
    },
    {
        "id": "4a8b6a2e-9ad8-458a-8833-fb5eeb87a3db",
        "transfer_type": "credit",
        "from_account": null,
        "to_account": "b163f805-401b-4a41-8afd-b2903c0c1704",
        "amount": 0.9,
        "currency": "inr",
        "status": "completed",
        "description": null,
        "created_at": "2025-12-21T13:16:16.716756+00:00",
        "idempotency_key": "dc17b628-1bae-49c9-9053-c5b99be0de29_credit",
        "parent_tx_key": "txgroup_4ad94ec9-cbce-49f9-90af-23918bf84d0d"
    }
]
```

**Example**:

```bash
curl 'http://localhost:3000/api/v1/transfer/info/txgroup_abc123' \
  -H 'Authorization: Bearer pk_live_xxx'
```

---

### GET /api/v1/transfer/info/:id

Get a specific transaction by ID.

**Authentication**: Required

**Path Parameters**:

- `id` (UUID): Transaction ID

**Response** (`200 OK`):

```json
{
    "id": "4a8b6a2e-9ad8-458a-8833-fb5eeb87a3db",
    "transfer_type": "credit",
    "from_account": null,
    "to_account": "b163f805-401b-4a41-8afd-b2903c0c1704",
    "amount": 0.9,
    "currency": "inr",
    "status": "completed",
    "description": null,
    "created_at": "2025-12-21T13:16:16.716756+00:00",
    "idempotency_key": "dc17b628-1bae-49c9-9053-c5b99be0de29_credit",
    "parent_tx_key": "txgroup_4ad94ec9-cbce-49f9-90af-23918bf84d0d"
}
```

**Example**:

```bash
curl 'http://localhost:3000/api/v1/transfer/a1b2c3d4-e5f6-7890-abcd-ef1234567890' \
  -H 'Authorization: Bearer pk_live_xxx'
```

---

## Webhooks API

Webhooks send HTTP POST notifications to your URL when transactions occur on your account.

### POST /api/v1/webhooks/set

Create a webhook subscription.

**Authentication**: Required

**Request Body**:

```json
{
  "account_id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "url": "https://webhook.site/unique-id",
  "secret": "whsec_your_secret_key"
}
```

**Fields**:

- `account_id` (UUID, required): Must match authenticated account
- `url` (string, required): Webhook endpoint URL (must start with http:// or https://)
- `secret` (string, required): Secret key for HMAC signature verification

**Response** (`201 Created`):

```json
{
  "id": "webhook-uuid",
  "account_id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
  "url": "https://webhook.site/unique-id",
  "status": "active",
  "created_at": "2025-12-21T17:00:00Z"
}
```

**Example**:

```bash
curl -X POST 'http://localhost:3000/api/v1/webhooks/set' \
  -H 'Authorization: Bearer pk_live_xxx' \
  -H 'Content-Type: application/json' \
  -d '{
    "account_id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
    "url": "https://webhook.site/unique-id",
    "secret": "whsec_secret123"
  }'
```

---

### GET /api/v1/webhooks/info

List all webhooks for an account.

**Authentication**: Required

**Query Parameters**:

- `account_id` (UUID, required): Must match authenticated account

**Response** (`200 OK`):

```json
{
  "webhooks": [
    {
      "id": "webhook-uuid",
      "account_id": "58c297a9-4dc3-451c-a8a7-1202e3031248",
      "url": "https://webhook.site/unique-id",
      "status": "active",
      "created_at": "2025-12-21T17:00:00Z"
    }
  ],
  "total": 1
}
```

**Example**:

```bash
curl 'http://localhost:3000/api/v1/webhooks/info?account_id=58c297a9-4dc3-451c-a8a7-1202e3031248' \
  -H 'Authorization: Bearer pk_live_xxx'
```

---

### POST /api/v1/webhooks/unset

Delete a webhook subscription.

**Authentication**: Required

**Request Body**:

```json
{
  "webhook_id": "webhook-uuid-to-delete"
}
```

**Response** (`200 OK`):

```json
{
  "message": "Webhook deleted successfully"
}
```

**Example**:

```bash
curl -X POST 'http://localhost:3000/api/v1/webhooks/unset' \
  -H 'Authorization: Bearer pk_live_xxx' \
  -H 'Content-Type: application/json' \
  -d '{"webhook_id": "webhook-uuid-to-delete"}'
```

---

### Webhook Payload Format

When a transaction occurs, the following payload is sent to your webhook URL:

#### Debit Event

```json
{
  "event": "transaction.debited",
  "message": "Amount has been debited from your account",
  "data": {
    "transaction_id": "txn-uuid",
    "amount": 75.0,
    "currency": "USD",
    "description": "Payment for services",
    "parent_tx_key": "txgroup_abc123"
  },
  "timestamp": "2025-12-21T16:00:00Z"
}
```

#### Credit Event

```json
{
  "event": "transaction.credited",
  "message": "Amount has been credited to your account",
  "data": {
    "transaction_id": "txn-uuid",
    "amount": 100.0,
    "currency": "USD",
    "description": "Refund",
    "parent_tx_key": "txgroup_xyz789"
  },
  "timestamp": "2025-12-21T16:05:00Z"
}
```

#### Webhook Headers

Every webhook request includes:

- `Content-Type: application/json`
- `X-Webhook-Signature`: HMAC-SHA256 signature of the payload
- `X-Webhook-Event`: Event type (transaction.debited or transaction.credited)

#### Verifying Webhook Signatures

**Python Example**:

```python
import hmac
import hashlib

def verify_webhook(payload, signature, secret):
    expected = hmac.new(
        secret.encode(),
        payload.encode(),
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(expected, signature)

# Usage
payload = request.body  # Raw JSON string
signature = request.headers['X-Webhook-Signature']
secret = "whsec_your_secret_key"

if verify_webhook(payload, signature, secret):
    # Process webhook
    pass
```

**Node.js Example**:

```javascript
const crypto = require("crypto");

function verifyWebhook(payload, signature, secret) {
  const expected = crypto
    .createHmac("sha256", secret)
    .update(payload)
    .digest("hex");
  return crypto.timingSafeEqual(Buffer.from(signature), Buffer.from(expected));
}
```

---

## Error Codes

All error responses follow this format:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message"
  }
}
```

### Common Error Codes

| Code                   | HTTP Status | Description                                     |
| ---------------------- | ----------- | ----------------------------------------------- |
| `INVALID_API_KEY`      | 401         | Missing or invalid API key                      |
| `UNAUTHORIZED`         | 403         | Insufficient permissions or account mismatch    |
| `NOT_FOUND`            | 404         | Resource not found                              |
| `ACCOUNT_NOT_FOUND`    | 404         | Account does not exist                          |
| `WEBHOOK_NOT_FOUND`    | 404         | Webhook does not exist                          |
| `INVALID_REQUEST`      | 400         | Bad request parameters                          |
| `MISSING_ACCOUNT_ID`   | 400         | account_id parameter required                   |
| `INVALID_URL`          | 400         | Webhook URL must start with http:// or https:// |
| `INSUFFICIENT_BALANCE` | 400         | Account has insufficient funds                  |
| `RATE_LIMIT_EXCEEDED`  | 429         | Too many requests                               |
| `DATABASE_ERROR`       | 500         | Internal server error                           |

### Example Error Responses

**Unauthorized**:

```json
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "You can only access your own resources"
  }
}
```

**Insufficient Balance**:

```json
{
  "error": {
    "code": "INSUFFICIENT_BALANCE",
    "message": "Account has insufficient funds for this transaction"
  }
}
```

**Account Not Found**:

```json
{
  "error": {
    "code": "ACCOUNT_NOT_FOUND",
    "message": "Account not found"
  }
}
```

---

## Testing with cURL

### Complete Workflow Example

```bash
# 1. Create an account
ACCOUNT_RESPONSE=$(curl -s -X POST 'http://localhost:3000/api/v1/accounts' \
  -H 'Authorization: Bearer pk_live_xxx' \
  -H 'Content-Type: application/json' \
  -d '{
    "business_name": "Test Business",
    "email": "test@example.com",
    "currency": "USD"
  }')

ACCOUNT_ID=$(echo $ACCOUNT_RESPONSE | jq -r '.id')
echo "Created account: $ACCOUNT_ID"

# 2. Add funds (credit)
curl -X POST 'http://localhost:3000/api/v1/transfer' \
  -H 'Authorization: Bearer pk_live_xxx' \
  -H 'Content-Type: application/json' \
  -d "{
    \"type\": \"credit\",
    \"to_account\": \"$ACCOUNT_ID\",
    \"amount\": 1000.00,
    \"currency\": \"USD\",
    \"description\": \"Initial deposit\"
  }"

# 3. Check balance
curl "http://localhost:3000/api/v1/accounts/$ACCOUNT_ID/balance" \
  -H 'Authorization: Bearer pk_live_xxx'

# 4. Create webhook
curl -X POST 'http://localhost:3000/api/v1/webhooks/set' \
  -H 'Authorization: Bearer pk_live_xxx' \
  -H 'Content-Type: application/json' \
  -d "{
    \"account_id\": \"$ACCOUNT_ID\",
    \"url\": \"https://webhook.site/your-unique-id\",
    \"secret\": \"test_secret_123\"
  }"

# 5. Make a transfer (triggers webhook)
curl -X POST 'http://localhost:3000/api/v1/transfer' \
  -H 'Authorization: Bearer pk_live_xxx' \
  -H 'Content-Type: application/json' \
  -d "{
    \"type\": \"debit\",
    \"from_account\": \"$ACCOUNT_ID\",
    \"amount\": 50.00,
    \"currency\": \"USD\",
    \"description\": \"Test payment\"
  }"

# 6. List transfers
curl "http://localhost:3000/api/v1/transfer/list?account_id=$ACCOUNT_ID" \
  -H 'Authorization: Bearer pk_live_xxx'
```

---

## Base URLs

- **Local Development**: `http://localhost:3000`
- **Production**: `https://api.yourdomain.com`

---

## Support

For API support, contact: support@example.com
