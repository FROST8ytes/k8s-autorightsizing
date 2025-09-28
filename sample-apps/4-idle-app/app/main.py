#!/usr/bin/env python3
"""
Idle Application - Webhook Listener
Demonstrates over-provisioned workload for auto-rightsizing testing
"""

import os
import time
import logging
from datetime import datetime, timezone
from flask import Flask, request, jsonify
from prometheus_client import Counter, Gauge, Histogram, generate_latest, CONTENT_TYPE_LATEST

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='{"time":"%(asctime)s", "level":"%(levelname)s", "msg":"%(message)s"}',
)
logger = logging.getLogger(__name__)

app = Flask(__name__)

# Configuration from environment variables
SIMULATE_LOAD = os.getenv('SIMULATE_LOAD', 'false').lower() == 'true'

# Prometheus metrics
request_counter = Counter(
    'app_requests_total',
    'Total requests',
    [
        'endpoint',
        'method',
        'status',
    ]
)
webhook_counter = Counter(
    'app_webhooks_received_total',
    'Total webhooks received',
    ['event_type'],
)
request_duration = Histogram(
    'app_request_duration_seconds',
    'Request duration',
    ['endpoint'],
)
active_connections = Gauge(
    'app_active_connections',
    'Number of active connections',
)
cpu_gauge = Gauge(
    'app_cpu_usage_estimate',
    'Estimated CPU usage',
)
memory_gauge = Gauge(
    'app_memory_usage_bytes',
    'Estimated memory usage in bytes',
)

# Simulated metrics (very low values for idle app)
cpu_gauge.set(0.01)  # 1% of 1 core = 10m
memory_gauge.set(30 * 1024 * 1024)  # 30Mi


@app.route('/health')
def health():
    """Health check endpoint"""
    request_counter.labels(
        endpoint='/health',
        method='GET',
        status='200',
    ).inc()
    return jsonify({
        'status': 'healthy',
        'timestamp': datetime.now(timezone.utc).isoformat(),
        'simulate_load': SIMULATE_LOAD,
    })


@app.route('/ready')
def ready():
    """Readiness check endpoint"""
    request_counter.labels(
        endpoint='/ready',
        method='GET',
        status='200',
    ).inc()
    return jsonify({
        'ready': True,
        'timestamp': datetime.now(timezone.utc).isoformat(),
    })


@app.route('/metrics')
def metrics():
    """Prometheus metrics endpoint"""
    return generate_latest(), 200, {'Content-Type': CONTENT_TYPE_LATEST}


@app.route('/status')
def status():
    """Detailed status endpoint"""
    request_counter.labels(
        endpoint='/status',
        method='GET',
        status='200',
    ).inc()
    return jsonify({
        'application': 'idle-webhook-listener',
        'version': '1.0.0',
        'timestamp': datetime.now(timezone.utc).isoformat(),
        'config': {
            'simulate_load': SIMULATE_LOAD,
        },
        'metrics': {
            'cpu_usage_estimate_cores': 0.01,
            'memory_usage_mb': 30,
        },
    })


@app.route('/')
def index():
    """Root endpoint"""
    request_counter.labels(
        endpoint='/',
        method='GET', status='200',
    ).inc()
    return jsonify({
        'name': 'Idle Webhook Listener',
        'description': 'Demonstrates over-provisioned workload for auto-rightsizing',
        'endpoints': {
            '/health': 'Health check',
            '/ready': 'Readiness check',
            '/metrics': 'Prometheus metrics',
            '/status': 'Detailed status',
            '/webhook': 'POST - Webhook receiver',
        },
    })


@app.route('/webhook', methods=['POST'])
def webhook():
    """Webhook receiver endpoint"""
    start_time = time.time()

    try:
        # Get webhook data
        data = request.get_json() or {}
        event_type = data.get('event_type', 'unknown')

        # Increment webhook counter
        webhook_counter.labels(event_type=event_type).inc()

        # Log the webhook
        logger.info(
            f"Webhook received: event_type={event_type}, data_size={len(str(data))}")

        # Simulate minimal processing
        if SIMULATE_LOAD:
            time.sleep(0.01)  # 10ms processing time

        request_counter.labels(
            endpoint='/webhook',
            method='POST',
            status='200',
        ).inc()
        duration = time.time() - start_time
        request_duration.labels(endpoint='/webhook').observe(duration)

        return jsonify({
            'status': 'received',
            'event_type': event_type,
            'timestamp': datetime.now(timezone.utc).isoformat(),
            'processing_time_ms': round(duration * 1000, 2),
        }), 200

    except Exception as e:
        logger.error(f"Error processing webhook: {str(e)}")
        request_counter.labels(
            endpoint='/webhook',
            method='POST',
            status='500',
        ).inc()
        return jsonify({
            'status': 'error',
            'message': str(e),
            'timestamp': datetime.now(timezone.utc).isoformat(),
        }), 500


@app.before_request
def before_request():
    """Track active connections"""
    active_connections.inc()


@app.after_request
def after_request(response):
    """Release active connections"""
    active_connections.dec()
    return response


if __name__ == '__main__':
    logger.info("=" * 60)
    logger.info("Idle Webhook Listener Starting")
    logger.info("=" * 60)
    logger.info(f"Configuration: SIMULATE_LOAD={SIMULATE_LOAD}")
    logger.info("This application is deliberately over-provisioned")
    logger.info("Expected CPU usage: ~10m (0.01 cores)")
    logger.info("Expected Memory usage: ~30Mi")
    logger.info("Provisioned CPU: 500m (50x more than needed)")
    logger.info("Provisioned Memory: 1Gi (34x more than needed)")

    # Start Flask server
    port = int(os.getenv('PORT', '8080'))
    logger.info(f"Starting HTTP server on port {port}")
    app.run(host='0.0.0.0', port=port, debug=False)
