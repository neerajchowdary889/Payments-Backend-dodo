-- ============================================================================
-- Dodo Payments - Transaction Service Database Schema
-- ============================================================================
-- This schema supports:
-- - Business accounts with balance tracking
-- - Atomic transactions (credit, debit, transfer)
-- - API key authentication with rate limiting
-- - Webhook delivery system with retry mechanism
-- - Idempotency support for API requests
-- ============================================================================

-- Connect to payments database (already created by docker-compose)
\c payments_db

-- Enable UUID extension for generating unique identifiers
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- ACCOUNTS TABLE
-- ============================================================================
-- Stores business account information and balances
CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    
    -- Balance tracking (stored in smallest currency unit, e.g., cents)
    balance BIGINT NOT NULL DEFAULT 0 CHECK (balance >= 0),
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    
    -- Account status
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'closed')),
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================================
-- API KEYS TABLE
-- ============================================================================
-- Stores API keys for authentication and authorization
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    
    -- API key (hashed for security)
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    key_prefix VARCHAR(20) NOT NULL, -- First few chars for identification (e.g., "sk_live_abc")
    
    -- Key metadata
    name VARCHAR(100), -- Optional name for the key
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked')),
    
    -- Rate limiting
    rate_limit_per_minute INTEGER DEFAULT 100,
    rate_limit_per_hour INTEGER DEFAULT 1000,
    
    -- Permissions (for future extensibility)
    permissions JSONB DEFAULT '["read", "write"]',
    
    -- Timestamps
    last_used_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMP WITH TIME ZONE
);

-- ============================================================================
-- TRANSACTIONS TABLE
-- ============================================================================
-- Stores all financial transactions (credit, debit, transfer)
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    
    -- Transaction type
    transaction_type VARCHAR(20) NOT NULL CHECK (transaction_type IN ('credit', 'debit', 'transfer')),
    
    -- Account references
    from_account_id UUID REFERENCES accounts(id) ON DELETE RESTRICT,
    to_account_id UUID REFERENCES accounts(id) ON DELETE RESTRICT,
    
    -- Amount (in smallest currency unit)
    amount BIGINT NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    
    -- Transaction status
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'completed', 'failed', 'reversed')),
    
    -- Idempotency support
    idempotency_key VARCHAR(255) UNIQUE,
    
    -- Description and metadata
    description TEXT,
    metadata JSONB DEFAULT '{}',
    
    -- Error tracking
    error_code VARCHAR(50),
    error_message TEXT,
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP WITH TIME ZONE,
    
    -- Constraints
    CONSTRAINT valid_transaction_accounts CHECK (
        (transaction_type = 'credit' AND from_account_id IS NULL AND to_account_id IS NOT NULL) OR
        (transaction_type = 'debit' AND from_account_id IS NOT NULL AND to_account_id IS NULL) OR
        (transaction_type = 'transfer' AND from_account_id IS NOT NULL AND to_account_id IS NOT NULL AND from_account_id != to_account_id)
    )
);

-- ============================================================================
-- WEBHOOKS TABLE
-- ============================================================================
-- Stores webhook endpoint configurations for accounts
CREATE TABLE IF NOT EXISTS webhooks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    
    -- Webhook configuration
    url TEXT NOT NULL,
    secret VARCHAR(255) NOT NULL, -- For HMAC signature verification
    
    -- Event subscriptions
    events JSONB NOT NULL DEFAULT '["transaction.created", "transaction.completed", "transaction.failed"]',
    
    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled', 'failed')),
    
    -- Retry configuration
    max_retries INTEGER DEFAULT 3,
    retry_backoff_seconds INTEGER DEFAULT 60,
    
    -- Failure tracking
    consecutive_failures INTEGER DEFAULT 0,
    last_failure_at TIMESTAMP WITH TIME ZONE,
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================================
-- WEBHOOK DELIVERIES TABLE
-- ============================================================================
-- Tracks webhook delivery attempts and their results
CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    webhook_id UUID NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    transaction_id UUID REFERENCES transactions(id) ON DELETE CASCADE,
    
    -- Event details
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    
    -- Delivery status
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'delivered', 'failed')),
    
    -- Retry tracking
    attempt_count INTEGER DEFAULT 0,
    max_attempts INTEGER DEFAULT 3,
    next_retry_at TIMESTAMP WITH TIME ZONE,
    
    -- Response tracking
    http_status_code INTEGER,
    response_body TEXT,
    error_message TEXT,
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    delivered_at TIMESTAMP WITH TIME ZONE,
    failed_at TIMESTAMP WITH TIME ZONE
);

-- ============================================================================
-- RATE LIMIT TRACKING TABLE (Optional - for in-memory alternative)
-- ============================================================================
-- Tracks API request counts for rate limiting
CREATE TABLE IF NOT EXISTS rate_limit_counters (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    
    -- Time window
    window_start TIMESTAMP WITH TIME ZONE NOT NULL,
    window_type VARCHAR(10) NOT NULL CHECK (window_type IN ('minute', 'hour')),
    
    -- Counter
    request_count INTEGER NOT NULL DEFAULT 1,
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- Unique constraint to prevent duplicate windows
    UNIQUE(api_key_id, window_start, window_type)
);

-- ============================================================================
-- INDEXES FOR PERFORMANCE
-- ============================================================================

-- Accounts indexes
CREATE INDEX IF NOT EXISTS idx_accounts_email ON accounts(email);

-- API Keys indexes
CREATE INDEX IF NOT EXISTS idx_api_keys_account_id ON api_keys(account_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_prefix ON api_keys(key_prefix);

-- Transactions indexes
CREATE INDEX IF NOT EXISTS idx_transactions_from_account ON transactions(from_account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_to_account ON transactions(to_account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(transaction_type);
CREATE INDEX IF NOT EXISTS idx_transactions_idempotency_key ON transactions(idempotency_key) WHERE idempotency_key IS NOT NULL;

-- Composite index for account transaction history
CREATE INDEX IF NOT EXISTS idx_transactions_account_history ON transactions(from_account_id, created_at DESC) WHERE from_account_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_transactions_recipient_history ON transactions(to_account_id, created_at DESC) WHERE to_account_id IS NOT NULL;

-- Webhooks indexes
CREATE INDEX IF NOT EXISTS idx_webhooks_account_id ON webhooks(account_id);
CREATE INDEX IF NOT EXISTS idx_webhooks_status ON webhooks(status);

-- Webhook deliveries indexes
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_webhook_id ON webhook_deliveries(webhook_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_transaction_id ON webhook_deliveries(transaction_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_status ON webhook_deliveries(status);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_next_retry ON webhook_deliveries(next_retry_at) WHERE status = 'pending';

-- Rate limit indexes
CREATE INDEX IF NOT EXISTS idx_rate_limit_api_key_window ON rate_limit_counters(api_key_id, window_start, window_type);

-- ============================================================================
-- TRIGGERS FOR AUTOMATIC TIMESTAMP UPDATES
-- ============================================================================

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply trigger to accounts
CREATE TRIGGER update_accounts_updated_at
    BEFORE UPDATE ON accounts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Apply trigger to webhooks
CREATE TRIGGER update_webhooks_updated_at
    BEFORE UPDATE ON webhooks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- SAMPLE DATA FOR TESTING
-- ============================================================================

-- Insert sample accounts
INSERT INTO accounts (id, business_name, email, balance, currency) VALUES 
    ('11111111-1111-1111-1111-111111111111', 'Acme Corporation', 'acme@example.com', 100000, 'USD'),
    ('22222222-2222-2222-2222-222222222222', 'TechStart Inc', 'techstart@example.com', 50000, 'USD'),
    ('33333333-3333-3333-3333-333333333333', 'Global Traders', 'global@example.com', 75000, 'USD')
ON CONFLICT (id) DO NOTHING;

-- Insert sample API keys (these are hashed values - in production, hash the actual keys)
-- Note: In production, use proper bcrypt/argon2 hashing
INSERT INTO api_keys (account_id, key_hash, key_prefix, name) VALUES 
    ('11111111-1111-1111-1111-111111111111', 'hashed_key_acme_123', 'sk_test_acme', 'Acme Production Key'),
    ('22222222-2222-2222-2222-222222222222', 'hashed_key_tech_456', 'sk_test_tech', 'TechStart API Key'),
    ('33333333-3333-3333-3333-333333333333', 'hashed_key_global_789', 'sk_test_glob', 'Global Traders Key')
ON CONFLICT DO NOTHING;

-- Insert sample webhooks
INSERT INTO webhooks (account_id, url, secret) VALUES 
    ('11111111-1111-1111-1111-111111111111', 'https://acme.example.com/webhooks', 'whsec_acme_secret_123'),
    ('22222222-2222-2222-2222-222222222222', 'https://techstart.example.com/webhooks', 'whsec_tech_secret_456')
ON CONFLICT DO NOTHING;

-- ============================================================================
-- GRANT PRIVILEGES
-- ============================================================================

GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO postgres;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO postgres;
GRANT USAGE ON SCHEMA public TO postgres;

-- ============================================================================
-- CONFIRMATION
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '✅ Payments database initialized successfully!';
    RAISE NOTICE '📊 Tables created: accounts, api_keys, transactions, webhooks, webhook_deliveries, rate_limit_counters';
    RAISE NOTICE '🔍 Indexes created for optimal query performance';
    RAISE NOTICE '🧪 Sample data inserted for testing';
END $$;
