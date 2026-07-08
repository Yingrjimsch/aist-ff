use opentelemetry::global;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_stdout::MetricExporter;
use tracing_subscriber::layer::SubscriberExt;

pub fn init_telemetry() -> (opentelemetry_sdk::logs::SdkLoggerProvider, SdkMeterProvider) {
    let log_exporter: opentelemetry_stdout::LogExporter =
        opentelemetry_stdout::LogExporter::default();
    let logger_provider: opentelemetry_sdk::logs::SdkLoggerProvider =
        opentelemetry_sdk::logs::SdkLoggerProvider::builder()
            .with_log_processor(
                opentelemetry_sdk::logs::BatchLogProcessor::builder(log_exporter).build(),
            )
            .build();

    let otel_layer: opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge<
        opentelemetry_sdk::logs::SdkLoggerProvider,
        opentelemetry_sdk::logs::SdkLogger,
    > = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider);
    let subscriber: tracing_subscriber::layer::Layered<
        opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge<
            opentelemetry_sdk::logs::SdkLoggerProvider,
            opentelemetry_sdk::logs::SdkLogger,
        >,
        tracing_subscriber::Registry,
    > = tracing_subscriber::registry().with(otel_layer);
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    let metric_exporter = MetricExporter::default();

    // A PeriodicReader tells SDK to collect and output metrics at a set interval (e.g., every 30s)
    let reader: PeriodicReader<MetricExporter> = PeriodicReader::builder(metric_exporter).build();

    let meter_provider: SdkMeterProvider = SdkMeterProvider::builder().with_reader(reader).build();

    // Set this provider as the global metrics provider
    global::set_meter_provider(meter_provider.clone());

    (logger_provider, meter_provider)
}
