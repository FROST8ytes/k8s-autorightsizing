#!/usr/bin/env python3
"""
CPU-Intensive Application - Prime Number Calculator
Demonstrates CPU-bound workload for auto-rightsizing testing
"""

import os
import time
import logging
import threading
from datetime import datetime
from flask import Flask, jsonify
from prometheus_client import Counter, Gauge, Histogram, generate_latest, CONTENT_TYPE_LATEST

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='{"time":"%(asctime)s", "level":"%(levelname)s", "msg":"%(message)s"}'
)
logger = logging.getLogger(__name__)

app = Flask(__name__)

# Configuration from environment variables
INTENSITY = os.getenv('INTENSITY', 'medium')  # low, medium, high
WORKERS = int(os.getenv('WORKERS', '4'))
ENABLED = os.getenv('ENABLED', 'true').lower() == 'true'

# Prometheus metrics
request_counter = Counter('app_requests_total', 'Total requests', [
                          'endpoint', 'method'])
prime_counter = Counter('app_primes_calculated_total',
                        'Total prime numbers calculated')
cpu_gauge = Gauge('app_cpu_intensity', 'Current CPU intensity level (0-3)')
calculation_duration = Histogram(
    'app_calculation_duration_seconds', 'Time spent calculating primes')
active_workers = Gauge('app_active_workers', 'Number of active worker threads')

# Map intensity to numeric values for metrics
INTENSITY_MAP = {'low': 1, 'medium': 2, 'high': 3}

# Worker control
workers_running = []


def is_prime(n):
    """Check if a number is prime (intentionally inefficient for CPU load)"""
    if n < 2:
        return False
    if n == 2:
        return True
    if n % 2 == 0:
        return False

    # Intentionally inefficient algorithm to consume CPU
    for i in range(3, int(n ** 0.5) + 1, 2):
        if n % i == 0:
            return False
    return True


def calculate_primes_in_range(start, end):
    """Calculate all prime numbers in a range"""
    primes = []
    for num in range(start, end):
        if is_prime(num):
            primes.append(num)
            prime_counter.inc()
    return primes


def cpu_worker(worker_id):
    """Worker thread that continuously calculates primes"""
    logger.info(f"Worker {worker_id} started with intensity: {INTENSITY}")

    # Set range based on intensity
    if INTENSITY == 'low':
        range_size = 1000
        sleep_time = 0.5
    elif INTENSITY == 'medium':
        range_size = 5000
        sleep_time = 0.1
    else:  # high
        range_size = 10000
        sleep_time = 0.01

    counter = 0
    while ENABLED and worker_id in workers_running:
        start = counter * range_size
        end = start + range_size

        start_time = time.time()
        primes = calculate_primes_in_range(start, end)
        duration = time.time() - start_time

        calculation_duration.observe(duration)

        logger.info(
            f"Worker {worker_id}: Found {len(primes)} primes in range {start}-{end} ({duration:.2f}s)")

        counter += 1
        time.sleep(sleep_time)

    logger.info(f"Worker {worker_id} stopped")


def start_workers():
    """Start CPU worker threads"""
    global workers_running
    workers_running = list(range(WORKERS))

    for i in range(WORKERS):
        thread = threading.Thread(target=cpu_worker, args=(i,), daemon=True)
        thread.start()
        logger.info(f"Started worker thread {i}")

    active_workers.set(WORKERS)
    cpu_gauge.set(INTENSITY_MAP.get(INTENSITY, 2))


@app.route('/health')
def health():
    """Health check endpoint"""
    request_counter.labels(endpoint='/health', method='GET').inc()
    return jsonify({
        'status': 'healthy',
        'timestamp': datetime.utcnow().isoformat(),
        'intensity': INTENSITY,
        'workers': WORKERS,
        'enabled': ENABLED
    })


@app.route('/ready')
def ready():
    """Readiness check endpoint"""
    request_counter.labels(endpoint='/ready', method='GET').inc()
    is_ready = len(workers_running) == WORKERS if ENABLED else True
    status_code = 200 if is_ready else 503

    return jsonify({
        'ready': is_ready,
        'active_workers': len(workers_running),
        'expected_workers': WORKERS
    }), status_code


@app.route('/metrics')
def metrics():
    """Prometheus metrics endpoint"""
    return generate_latest(), 200, {'Content-Type': CONTENT_TYPE_LATEST}


@app.route('/status')
def status():
    """Detailed status endpoint"""
    request_counter.labels(endpoint='/status', method='GET').inc()
    return jsonify({
        'application': 'cpu-intensive-prime-calculator',
        'version': '1.0.0',
        'timestamp': datetime.utcnow().isoformat(),
        'config': {
            'intensity': INTENSITY,
            'workers': WORKERS,
            'enabled': ENABLED
        },
        'metrics': {
            'active_workers': len(workers_running),
            'cpu_intensity_level': INTENSITY_MAP.get(INTENSITY, 2)
        }
    })


@app.route('/')
def index():
    """Root endpoint"""
    request_counter.labels(endpoint='/', method='GET').inc()
    return jsonify({
        'name': 'CPU-Intensive Prime Calculator',
        'description': 'Demonstrates CPU-bound workload for auto-rightsizing',
        'endpoints': {
            '/health': 'Health check',
            '/ready': 'Readiness check',
            '/metrics': 'Prometheus metrics',
            '/status': 'Detailed status',
            '/calculate': 'Calculate primes in range'
        }
    })


@app.route('/calculate')
def calculate():
    """On-demand calculation endpoint"""
    request_counter.labels(endpoint='/calculate', method='GET').inc()

    # Calculate primes up to 10000
    start_time = time.time()
    primes = calculate_primes_in_range(2, 10000)
    duration = time.time() - start_time

    calculation_duration.observe(duration)

    return jsonify({
        'count': len(primes),
        'duration_seconds': round(duration, 3),
        'sample': primes[:10] if len(primes) > 10 else primes
    })


if __name__ == '__main__':
    logger.info("=" * 60)
    logger.info("CPU-Intensive Prime Calculator Starting")
    logger.info("=" * 60)
    logger.info(
        f"Configuration: INTENSITY={INTENSITY}, WORKERS={WORKERS}, ENABLED={ENABLED}")

    if ENABLED:
        start_workers()
        logger.info(f"Started {WORKERS} worker threads")
    else:
        logger.info("Workers disabled via ENABLED=false")

    # Start Flask server
    port = int(os.getenv('PORT', '8080'))
    logger.info(f"Starting HTTP server on port {port}")
    app.run(host='0.0.0.0', port=port, debug=False)
