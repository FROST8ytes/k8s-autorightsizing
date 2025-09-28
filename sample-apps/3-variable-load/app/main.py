"""
Variable-Load Traffic Simulator
Demonstrates workloads with fluctuating resource usage patterns.
Simulates traffic spikes and valleys throughout the day.
"""

import os
import time
import random
import logging
import threading
from datetime import datetime, timedelta
from typing import Dict, List
from flask import Flask, jsonify, request
from prometheus_client import Counter, Gauge, Histogram, generate_latest, CONTENT_TYPE_LATEST
import psutil

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

app = Flask(__name__)

# Configuration from environment
PATTERN_MODE = os.getenv('PATTERN_MODE', 'wave')  # wave, spike, random, manual
# Base number of concurrent workers
BASE_LOAD = int(os.getenv('BASE_LOAD', '10'))
MAX_LOAD = int(os.getenv('MAX_LOAD', '50'))  # Maximum number of workers
CYCLE_DURATION = int(os.getenv('CYCLE_DURATION', '120')
                     )  # Seconds for one complete cycle
ENABLED = os.getenv('ENABLED', 'true').lower() == 'true'

# Prometheus metrics
requests_total = Counter('app_requests_total', 'Total HTTP requests', [
                         'method', 'endpoint', 'status'])
memory_usage_bytes = Gauge('app_memory_usage_bytes',
                           'Current memory usage in bytes')
cpu_usage_estimate = Gauge('app_cpu_usage_estimate',
                           'Estimated CPU cores in use')
active_workers = Gauge('app_active_workers', 'Number of active worker threads')
load_level = Gauge('app_load_level', 'Current load level (0-100)')
work_completed = Counter('app_work_completed_total',
                         'Total work units completed')
processing_time = Histogram(
    'app_processing_time_seconds', 'Time to complete work unit')

# Global state
workers: List[threading.Thread] = []
stop_event = threading.Event()
current_load_level = 0
manual_workers = BASE_LOAD


def compute_work(duration: float = 0.1):
    """Simulate CPU-intensive work"""
    start = time.time()
    result = 0
    while time.time() - start < duration:
        # CPU-intensive calculation
        result += sum(i ** 2 for i in range(1000))
    return result


def worker_task(worker_id: int):
    """Worker thread that performs continuous work"""
    logger.info(f"Worker {worker_id} started")

    while not stop_event.is_set():
        try:
            start_time = time.time()

            # Do some work
            compute_work(duration=random.uniform(0.05, 0.15))

            # Simulate some memory allocation
            temp_data = [random.random()
                         for _ in range(random.randint(1000, 5000))]

            duration = time.time() - start_time
            processing_time.observe(duration)
            work_completed.inc()

            # Small sleep to avoid spinning too fast
            time.sleep(random.uniform(0.1, 0.3))

        except Exception as e:
            logger.error(f"Worker {worker_id} error: {e}")

    logger.info(f"Worker {worker_id} stopped")


def calculate_target_workers(mode: str, cycle_time: int) -> int:
    """Calculate target number of workers based on pattern mode"""
    global current_load_level

    if mode == 'manual':
        return manual_workers

    # Calculate position in cycle (0 to 1)
    position = (cycle_time % CYCLE_DURATION) / CYCLE_DURATION

    if mode == 'wave':
        # Sine wave pattern: smooth oscillation
        import math
        load_factor = (math.sin(position * 2 * math.pi) + 1) / 2  # 0 to 1
        current_load_level = int(load_factor * 100)
        return int(BASE_LOAD + (MAX_LOAD - BASE_LOAD) * load_factor)

    elif mode == 'spike':
        # Sharp spikes: low most of the time, sudden spikes
        if position < 0.1 or (0.5 <= position < 0.6):
            current_load_level = 100
            return MAX_LOAD
        else:
            current_load_level = 20
            return BASE_LOAD + int((MAX_LOAD - BASE_LOAD) * 0.2)

    elif mode == 'random':
        # Random fluctuations
        load_factor = random.random()
        current_load_level = int(load_factor * 100)
        return int(BASE_LOAD + (MAX_LOAD - BASE_LOAD) * load_factor)

    else:
        return BASE_LOAD


def manage_workers():
    """Background thread that manages worker pool based on load pattern"""
    global workers

    start_time = time.time()

    while not stop_event.is_set():
        try:
            if ENABLED:
                elapsed = int(time.time() - start_time)
                target = calculate_target_workers(PATTERN_MODE, elapsed)
                current = len([w for w in workers if w.is_alive()])

                # Adjust worker count
                if target > current:
                    # Spawn new workers
                    for i in range(target - current):
                        worker = threading.Thread(
                            target=worker_task,
                            args=(len(workers) + i,),
                            daemon=True
                        )
                        worker.start()
                        workers.append(worker)
                    logger.info(
                        f"Scaled up to {target} workers (from {current})")

                elif target < current:
                    # Signal workers to stop
                    to_stop = current - target
                    logger.info(
                        f"Scaling down by {to_stop} workers (from {current} to {target})")

                # Update metrics
                active_workers.set(len([w for w in workers if w.is_alive()]))
                load_level.set(current_load_level)

            # Update system metrics
            process = psutil.Process()
            memory_info = process.memory_info()
            memory_usage_bytes.set(memory_info.rss)
            cpu_usage_estimate.set(process.cpu_percent() / 100.0)

            time.sleep(5)  # Check every 5 seconds

        except Exception as e:
            logger.error(f"Error in worker manager: {e}")
            time.sleep(5)


@app.route('/health', methods=['GET'])
def health():
    """Health check endpoint"""
    requests_total.labels(method='GET', endpoint='/health', status='200').inc()
    return jsonify({
        'status': 'healthy',
        'timestamp': datetime.utcnow().isoformat()
    })


@app.route('/status', methods=['GET'])
def status():
    """Get current application status"""
    process = psutil.Process()
    memory_info = process.memory_info()

    alive_workers = [w for w in workers if w.is_alive()]

    response = {
        'application': 'variable-load-simulator',
        'version': '1.0.0',
        'enabled': ENABLED,
        'config': {
            'pattern_mode': PATTERN_MODE,
            'base_load': BASE_LOAD,
            'max_load': MAX_LOAD,
            'cycle_duration': CYCLE_DURATION
        },
        'metrics': {
            'memory_usage_mb': round(memory_info.rss / 1024 / 1024, 2),
            'cpu_percent': round(process.cpu_percent(), 2),
            'active_workers': len(alive_workers),
            'total_workers_created': len(workers),
            'load_level_percent': current_load_level,
            'work_completed': int(work_completed._value.get())
        },
        'timestamp': datetime.utcnow().isoformat()
    }

    requests_total.labels(method='GET', endpoint='/status', status='200').inc()
    return jsonify(response)


@app.route('/config', methods=['POST'])
def update_config():
    """Update configuration dynamically"""
    global PATTERN_MODE, BASE_LOAD, MAX_LOAD, CYCLE_DURATION, manual_workers

    data = request.get_json() or {}

    if 'pattern_mode' in data:
        PATTERN_MODE = data['pattern_mode']
        logger.info(f"Pattern mode changed to: {PATTERN_MODE}")

    if 'base_load' in data:
        BASE_LOAD = int(data['base_load'])
        logger.info(f"Base load changed to: {BASE_LOAD}")

    if 'max_load' in data:
        MAX_LOAD = int(data['max_load'])
        logger.info(f"Max load changed to: {MAX_LOAD}")

    if 'cycle_duration' in data:
        CYCLE_DURATION = int(data['cycle_duration'])
        logger.info(f"Cycle duration changed to: {CYCLE_DURATION}")

    if 'workers' in data:
        manual_workers = int(data['workers'])
        logger.info(f"Manual workers set to: {manual_workers}")

    requests_total.labels(
        method='POST', endpoint='/config', status='200').inc()
    return jsonify({
        'status': 'success',
        'config': {
            'pattern_mode': PATTERN_MODE,
            'base_load': BASE_LOAD,
            'max_load': MAX_LOAD,
            'cycle_duration': CYCLE_DURATION,
            'manual_workers': manual_workers
        }
    })


@app.route('/pattern/<pattern_name>', methods=['POST'])
def set_pattern(pattern_name: str):
    """Quick way to set pattern mode"""
    global PATTERN_MODE

    valid_patterns = ['wave', 'spike', 'random', 'manual']

    if pattern_name not in valid_patterns:
        requests_total.labels(
            method='POST', endpoint='/pattern', status='400').inc()
        return jsonify({
            'error': f'Invalid pattern. Must be one of: {valid_patterns}'
        }), 400

    PATTERN_MODE = pattern_name
    logger.info(f"Pattern changed to: {pattern_name}")

    requests_total.labels(
        method='POST', endpoint='/pattern', status='200').inc()
    return jsonify({
        'status': 'success',
        'pattern': pattern_name
    })


@app.route('/metrics', methods=['GET'])
def metrics():
    """Prometheus metrics endpoint"""
    return generate_latest(), 200, {'Content-Type': CONTENT_TYPE_LATEST}


if __name__ == '__main__':
    # Start worker manager in background thread
    manager_thread = threading.Thread(target=manage_workers, daemon=True)
    manager_thread.start()

    logger.info(f"Starting Variable-Load Traffic Simulator")
    logger.info(
        f"Config: MODE={PATTERN_MODE}, BASE={BASE_LOAD}, MAX={MAX_LOAD}, CYCLE={CYCLE_DURATION}s")

    # Start Flask app
    app.run(host='0.0.0.0', port=8080)
