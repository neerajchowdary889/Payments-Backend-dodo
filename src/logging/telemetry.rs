use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource, runtime,
    trace::{RandomIdGenerator, Sampler},
};
use tracing::info;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Configuration for OpenTelemetry
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Service name for telemetry
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// OTLP endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: String,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Enable tracing
    pub enable_tracing: bool,
    /// Log level filter
    pub log_level: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: env!("CARGO_PKG_NAME").to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_string()),
            enable_metrics: std::env::var("OTEL_ENABLE_METRICS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_tracing: std::env::var("OTEL_ENABLE_TRACING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        }
    }
}

impl TelemetryConfig {
    /// Create a new telemetry configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the service name
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    /// Set the OTLP endpoint
    pub fn with_otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = endpoint.into();
        self
    }

    /// Set whether to enable metrics
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }

    /// Set whether to enable tracing
    pub fn with_tracing(mut self, enable: bool) -> Self {
        self.enable_tracing = enable;
        self
    }

    /// Set the log level
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }
}

/// Initialize OpenTelemetry with tracing and metrics
///
/// This sets up:
/// - Tracing with OTLP exporter
/// - Metrics with OTLP exporter
/// - Structured logging with tracing-subscriber
///
/// # Example
/// ```rust
/// use payments_backend_dodo::logging::init_telemetry;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     init_telemetry(None)?;
///     // Your application code
///     Ok(())
/// }
/// ```
pub fn init_telemetry(config: Option<TelemetryConfig>) -> Result<(), Box<dyn std::error::Error>> {
    let config = config.unwrap_or_default();

    println!("🔧 Initializing OpenTelemetry...");
    println!("   - Service: {}", config.service_name);
    println!("   - Version: {}", config.service_version);
    println!("   - OTLP Endpoint: {}", config.otlp_endpoint);
    println!("   - Metrics: {}", config.enable_metrics);
    println!("   - Tracing: {}", config.enable_tracing);

    // Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            config.service_name.clone(),
        ),
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
            config.service_version.clone(),
        ),
    ]);

    // Initialize tracing if enabled
    let tracer = if config.enable_tracing {
        println!("📊 Setting up distributed tracing...");

        // Set up OTLP trace exporter
        let tracer_provider = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(&config.otlp_endpoint),
            )
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default()
                    // Use ParentBased sampler to reduce trace volume
                    .with_sampler(Sampler::ParentBased(Box::new(Sampler::AlwaysOn)))
                    .with_id_generator(RandomIdGenerator::default())
                    .with_resource(resource.clone()),
            )
            .with_batch_config(
                opentelemetry_sdk::trace::BatchConfigBuilder::default()
                    .with_max_queue_size(10000) // Increase channel size from default 2048
                    .with_max_export_batch_size(2048) // Larger batches
                    .with_scheduled_delay(std::time::Duration::from_secs(1)) // Export every 1s
                    .build(),
            )
            .install_batch(runtime::Tokio)?;

        // Set as global tracer provider
        global::set_tracer_provider(tracer_provider.clone());

        Some(tracer_provider.tracer("payments-backend"))
    } else {
        None
    };

    // Initialize metrics if enabled
    if config.enable_metrics {
        println!("📈 Setting up metrics collection...");

        // Set up OTLP metrics exporter using the pipeline API
        let meter_provider = opentelemetry_otlp::new_pipeline()
            .metrics(runtime::Tokio)
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(&config.otlp_endpoint),
            )
            .with_resource(resource.clone())
            .with_period(std::time::Duration::from_secs(30))
            .build()?;

        // Set as global meter provider
        global::set_meter_provider(meter_provider);
    }

    // Set up tracing subscriber with multiple layers
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_filter(env_filter);

    let registry = tracing_subscriber::registry().with(fmt_layer);

    // Add OpenTelemetry tracing layer if enabled
    if let Some(tracer) = tracer {
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        registry.with(telemetry_layer).init();
    } else {
        registry.init();
    }

    println!("✅ OpenTelemetry initialized successfully!");

    Ok(())
}

/// Shutdown OpenTelemetry gracefully
///
/// This ensures all pending spans and metrics are flushed before shutdown
pub fn shutdown_telemetry() {
    info!("🛑 Shutting down OpenTelemetry...");

    // Shutdown tracer provider
    global::shutdown_tracer_provider();

    info!("✅ OpenTelemetry shutdown complete");
}

/// Create a custom metric counter
///
/// # Example
/// ```rust
/// use opentelemetry::global;
///
/// let meter = global::meter("payments-backend");
/// let counter = meter.u64_counter("http_requests_total").init();
/// counter.add(1, &[]);
/// ```
pub fn create_counter(name: &str, description: &str) -> opentelemetry::metrics::Counter<u64> {
    let meter = global::meter("payments-backend");
    meter
        .u64_counter(name.to_string())
        .with_description(description.to_string())
        .init()
}

/// Create a custom metric histogram
///
/// # Example
/// ```rust
/// use opentelemetry::global;
///
/// let meter = global::meter("payments-backend");
/// let histogram = meter.f64_histogram("http_request_duration_seconds").init();
/// histogram.record(0.5, &[]);
/// ```
pub fn create_histogram(name: &str, description: &str) -> opentelemetry::metrics::Histogram<f64> {
    let meter = global::meter("payments-backend");
    meter
        .f64_histogram(name.to_string())
        .with_description(description.to_string())
        .init()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, env!("CARGO_PKG_NAME"));
        assert_eq!(config.service_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_config_builder() {
        let config = TelemetryConfig::new()
            .with_service_name("test-service")
            .with_otlp_endpoint("http://localhost:4318")
            .with_metrics(false)
            .with_tracing(true)
            .with_log_level("debug");

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.otlp_endpoint, "http://localhost:4318");
        assert!(!config.enable_metrics);
        assert!(config.enable_tracing);
        assert_eq!(config.log_level, "debug");
    }
}
