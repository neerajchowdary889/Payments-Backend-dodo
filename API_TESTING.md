# API Testing Guide - Payments Backend

This guide provides curl commands for all 9 API endpoints. Copy these commands to test your APIs or import them into Postman.

**Base URL**: `http://localhost:8080`

---

## 📋 Table of Contents

1. [Public Endpoints](#public-endpoints) (No authentication required)
2. [Protected Account Endpoints](#protected-account-endpoints) (Require API key)
3. [Protected Transfer Endpoints](#protected-transfer-endpoints) (Require API key)
4. [Testing Workflow](#testing-workflow)

---

## Public Endpoints

### 1. Health Check

Check if the API is running.

```bash
curl -X GET http://localhost:8080/health
```

**Expected Response:**

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

---

### 2. Create Account

Create a new account and receive an API key (shown only once).

```bash
curl -X POST http://localhost:8080/api/v1/accounts \
  -H "Content-Type: application/json" \
  -d '{
    "business_name": "Acme Corporation",
    "email": "contact@acme.com",
    "currency": "USD"
  }'
```

**Expected Response:**

```json
{
  "account": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "business_name": "Acme Corporation",
    "email": "contact@acme.com",
    "balance": 0.0,
    "currency": "USD",
    "status": "active",
    "created_at": "2025-12-21T05:45:27Z"
  },
  "api_key": "ak_live_1234567890abcdef"
}
```

> ⚠️ **IMPORTANT**: Save the `api_key` from the response! You'll need it for all protected endpoints.

---

## Protected Account Endpoints

> 🔐 All protected endpoints require the `X-API-Key` header with your API key.

### 3. Get Account Details

Retrieve account information by ID.

```bash
# Replace YOUR_API_KEY and ACCOUNT_ID with actual values
curl -X GET http://localhost:8080/api/v1/accounts/ACCOUNT_ID \
  -H "X-API-Key: YOUR_API_KEY"
```

**Example:**

```bash
curl -X GET http://localhost:8080/api/v1/accounts/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-API-Key: ak_live_1234567890abcdef"
```

**Expected Response:**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "business_name": "Acme Corporation",
  "email": "contact@acme.com",
  "balance": 1000.0,
  "currency": "USD",
  "status": "active",
  "created_at": "2025-12-21T05:45:27Z"
}
```

---

### 4. Update Account Balance

Set a new balance for an account.

```bash
curl -X POST http://localhost:8080/api/v1/accounts/ACCOUNT_ID/putbalance \
  -H "X-API-Key: YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "ACCOUNT_ID",
    "balance": 1000.50,
    "currency": "USD"
  }'
```

**Example:**

```bash
curl -X POST http://localhost:8080/api/v1/accounts/550e8400-e29b-41d4-a716-446655440000/putbalance \
  -H "X-API-Key: ak_live_1234567890abcdef" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "550e8400-e29b-41d4-a716-446655440000",
    "balance": 1000.50,
    "currency": "USD"
  }'
```

---

### 5. Update Account Details

Update business name, email, or status.

```bash
curl -X PATCH http://localhost:8080/api/v1/accounts/ACCOUNT_ID \
  -H "X-API-Key: YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "business_name": "Acme Corp Updated",
    "email": "newemail@acme.com",
    "status": "active"
  }'
```

**Example (update only business name):**

```bash
curl -X PATCH http://localhost:8080/api/v1/accounts/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-API-Key: ak_live_1234567890abcdef" \
  -H "Content-Type: application/json" \
  -d '{
    "business_name": "Acme Corporation Ltd"
  }'
```

> 💡 **Note**: All fields are optional. Only include the fields you want to update.

---

### 6. Get Account Balance

Get balance information with optional currency conversion.

**Without currency conversion (returns balance in account's default currency):**

```bash
curl -X GET http://localhost:8080/api/v1/accounts/ACCOUNT_ID/balance \
  -H "X-API-Key: YOUR_API_KEY"
```

**With currency conversion:**

```bash
curl -X GET "http://localhost:8080/api/v1/accounts/ACCOUNT_ID/balance?currency=EUR" \
  -H "X-API-Key: YOUR_API_KEY"
```

**Examples:**

```bash
# Get balance in account's default currency
curl -X GET http://localhost:8080/api/v1/accounts/550e8400-e29b-41d4-a716-446655440000/balance \
  -H "X-API-Key: ak_live_1234567890abcdef"

# Get balance converted to EUR
curl -X GET "http://localhost:8080/api/v1/accounts/550e8400-e29b-41d4-a716-446655440000/balance?currency=EUR" \
  -H "X-API-Key: ak_live_1234567890abcdef"

# Get balance converted to INR
curl -X GET "http://localhost:8080/api/v1/accounts/550e8400-e29b-41d4-a716-446655440000/balance?currency=INR" \
  -H "X-API-Key: ak_live_1234567890abcdef"
```

**Expected Response:**

```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440000",
  "balance": 925.93,
  "currency": "EUR"
}
```

**Supported Currencies:**

- `USD` (US Dollar)
- `EUR` (Euro)
- `GBP` (British Pound)
- `CHF` (Swiss Franc)
- `AED` (UAE Dirham)
- `KWD` (Kuwaiti Dinar)
- `INR` (Indian Rupee)
- `CNY` (Chinese Yuan)
- `KRW` (Korean Won)
- `JPY` (Japanese Yen)
- `CAD` (Canadian Dollar)
- `BRL` (Brazilian Real)
- `ARS` (Argentine Peso)
- `AUD` (Australian Dollar)

---

## Protected Transfer Endpoints

### 7. Create Transfer

Transfer funds between accounts.

```bash
curl -X POST http://localhost:8080/api/v1/transfer \
  -H "X-API-Key: YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "from_account_id": "FROM_ACCOUNT_ID",
    "to_account_id": "TO_ACCOUNT_ID",
    "amount": 100.00,
    "currency": "USD",
    "description": "Payment for services",
    "idempotency_key": "unique-key-123"
  }'
```

**Example:**

```bash
curl -X POST http://localhost:8080/api/v1/transfer \
  -H "X-API-Key: ak_live_1234567890abcdef" \
  -H "Content-Type: application/json" \
  -d '{
    "from_account_id": "550e8400-e29b-41d4-a716-446655440000",
    "to_account_id": "660e8400-e29b-41d4-a716-446655440001",
    "amount": 100.00,
    "currency": "USD",
    "description": "Payment for invoice #1234",
    "idempotency_key": "transfer-2025-12-21-001"
  }'
```

---

### 8. Get Transfer Details

Retrieve transfer information by ID.

```bash
curl -X GET http://localhost:8080/api/v1/transfer/TRANSFER_ID \
  -H "X-API-Key: YOUR_API_KEY"
```

**Example:**

```bash
curl -X GET http://localhost:8080/api/v1/transfer/770e8400-e29b-41d4-a716-446655440002 \
  -H "X-API-Key: ak_live_1234567890abcdef"
```

---

### 9. List Transfers

List all transfers for an account.

```bash
curl -X GET "http://localhost:8080/api/v1/transfer/list?account_id=ACCOUNT_ID" \
  -H "X-API-Key: YOUR_API_KEY"
```

**Example:**

```bash
curl -X GET "http://localhost:8080/api/v1/transfer/list?account_id=550e8400-e29b-41d4-a716-446655440000" \
  -H "X-API-Key: ak_live_1234567890abcdef"
```

---

## Testing Workflow

Here's a recommended workflow to test all endpoints:

### Step 1: Start the Server

```bash
cd /Users/neeraj/CodeSection/Payments-Backend-dodo
cargo run
```

### Step 2: Health Check

```bash
curl -X GET http://localhost:8080/health
```

### Step 3: Create Two Accounts

```bash
# Account 1
curl -X POST http://localhost:8080/api/v1/accounts \
  -H "Content-Type: application/json" \
  -d '{
    "business_name": "Sender Account",
    "email": "sender@example.com",
    "currency": "USD"
  }'

# Account 2
curl -X POST http://localhost:8080/api/v1/accounts \
  -H "Content-Type: application/json" \
  -d '{
    "business_name": "Receiver Account",
    "email": "receiver@example.com",
    "currency": "USD"
  }'
```

**Save the API keys and account IDs from both responses!**

### Step 4: Add Balance to Account 1

```bash
curl -X POST http://localhost:8080/api/v1/accounts/ACCOUNT_1_ID/putbalance \
  -H "X-API-Key: ACCOUNT_1_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "ACCOUNT_1_ID",
    "balance": 1000.00,
    "currency": "USD"
  }'
```

### Step 5: Check Balance

```bash
# Check in USD
curl -X GET http://localhost:8080/api/v1/accounts/ACCOUNT_1_ID/balance \
  -H "X-API-Key: ACCOUNT_1_API_KEY"

# Check in EUR
curl -X GET "http://localhost:8080/api/v1/accounts/ACCOUNT_1_ID/balance?currency=EUR" \
  -H "X-API-Key: ACCOUNT_1_API_KEY"
```

### Step 6: Update Account Details

```bash
curl -X PATCH http://localhost:8080/api/v1/accounts/ACCOUNT_1_ID \
  -H "X-API-Key: ACCOUNT_1_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "business_name": "Updated Sender Account"
  }'
```

### Step 7: Get Account Details

```bash
curl -X GET http://localhost:8080/api/v1/accounts/ACCOUNT_1_ID \
  -H "X-API-Key: ACCOUNT_1_API_KEY"
```

### Step 8: Create a Transfer

```bash
curl -X POST http://localhost:8080/api/v1/transfer \
  -H "X-API-Key: ACCOUNT_1_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "from_account_id": "ACCOUNT_1_ID",
    "to_account_id": "ACCOUNT_2_ID",
    "amount": 100.00,
    "currency": "USD",
    "description": "Test transfer",
    "idempotency_key": "test-transfer-001"
  }'
```

### Step 9: List Transfers

```bash
curl -X GET "http://localhost:8080/api/v1/transfer/list?account_id=ACCOUNT_1_ID" \
  -H "X-API-Key: ACCOUNT_1_API_KEY"
```

---

## 📝 Notes

1. **Rate Limiting**:

   - Public endpoints: IP-based rate limiting (100 requests/minute)
   - Protected endpoints: API-key-based rate limiting (1000 requests/minute)

2. **Authentication**:

   - Protected endpoints require the `X-API-Key` header
   - API keys are generated when creating an account
   - API keys are shown only once during account creation

3. **Idempotency**:

   - Transfer endpoints support idempotency keys to prevent duplicate transfers
   - Use unique idempotency keys for each transfer

4. **Currency Conversion**:
   - Balance endpoint supports real-time currency conversion
   - Conversions are based on predefined exchange rates
   - All balances are stored in USD internally

---

## 🚀 Postman Import

To import these into Postman:

1. Copy any curl command
2. Open Postman
3. Click "Import" → "Raw text"
4. Paste the curl command
5. Click "Import"

Or create a Postman Collection with these endpoints and use environment variables for:

- `BASE_URL`: `http://localhost:8080`
- `API_KEY`: Your API key
- `ACCOUNT_ID`: Your account ID

---

## 🐛 Troubleshooting

**401 Unauthorized**: Check that your API key is correct and included in the `X-API-Key` header

**404 Not Found**: Verify the account ID or transfer ID exists

**429 Too Many Requests**: You've hit the rate limit, wait a moment and try again

**500 Internal Server Error**: Check the server logs for detailed error information
