# Dodo Payments Backend

A Containerized financial ledger system built in Rust. Designed for precision, reliability, and efficient resource utilization.

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/Docker-Enabled-blue.svg)](https://www.docker.com/)

---

## 📖 Overview

Backend system for handling multi-currency accounts and financial transactions. Unlike standard CRUD applications, it features custom-engineered components to ensure ACID guarantees, zero-precision-loss money handling, and optimal database performance.

For deep technical details, architectures, and tradeoffs, please read the **[Design Document](DESIGN.md)**.
For API usage, see the **[API Documentation](API.md)** or **[OpenAPI Spec](OPENAPI.md)**.

## 🚀 Key Features

### 🏛️ Robust Architecture

- **Hybrid Connection Pool**: Custom `PoolStateTracker` utilizing a "Hot/Cold" strategy. Eagerly loads core connections for zero-latency startup and lazy-loads additional connections on demand.
- **Dynamic SQL Generator**: Custom-built query builder (no ORM) for precise, type-safe SQL generation without overhead.
- **Centralized Error Handling**: Unified `ServiceError` module for consistent debugging and error reporting.

### 💰 Financial Integrity

- **Precision Money Storage**: All monetary values are stored as integers (`USD * 10,000`) to completely eliminate floating-point errors.
- **Multi-Currency Support**: Automatic conversion to USD storage units, with on-the-fly conversion back to requested currencies.
- **Double-Entry Ledger**: Atomic transactions with "Transfer -> Debit -> Credit" flow to ensure books always balance.

### 🔒 Security & Operations

- **Dual Rate Limiting**:
  - **Public APIs**: IP-based rate limiting.
  - **Protected APIs**: API-Key-based rate limiting.
  - **Strategy**: Soft limits (Exponential Backoff + Jitter) and Hard limits (Blocking).
- **Observability**: Built-in OpenTelemetry integration with Jaeger support for full request tracing.
- **State-Change Webhooks**: Reliable notification system triggering strictly on `Debit` and `Credit` events.

---

## 🛠️ Technology Stack

- **Core**: Rust (Axum, Tokio, SQLx)
- **Database**: PostgreSQL (Transactional & Account Data)
- **Cache**: Redis (Rate Limiting & App State)
- **Infrastructure**: Docker Compose, OpenTelemetry, Jaeger

---

## ⚡ Getting Started

### Prerequisites

- Docker & Docker Compose
- Rust Toolchain (optional, for local dev)

### Running the Application

The entire stack (App, Postgres, Redis, Jaeger) is containerized.

```bash
# Start all services
docker-compose up -d

# Check logs
docker-compose logs -f app
```

### Local Development

```bash
# Run migrations
sqlx migrate run

# Start server
cargo run
```

---

## 📚 Documentation Map

| Document                             | Purpose                                                                        |
| ------------------------------------ | ------------------------------------------------------------------------------ |
| **[DESIGN.md](DESIGN.md)**           | **Start Here**. detailed architecture, data models, and engineering decisions. |
| **[API.md](API.md)**                 | Developer basics: Endpoints, Request/Response examples, and curl commands.     |
| **[OPENAPI.md](OPENAPI.md)**         | Formal API specification.                                                      |
| **[API_TESTING.md](API_TESTING.md)** | Guide for running integration tests.                                           |

---

## 🧪 Testing

The project includes unit to integration tests covering concurrent transfers, rate limiting, and webhook dispatch.

```bash
# Run all tests
cargo test

# Run specific integration test
cargo test --test transfer_flow_test
```
