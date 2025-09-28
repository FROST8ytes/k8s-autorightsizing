"""
Memory-Intensive Data Processor
Demonstrates memory pressure and potential OOM scenarios.
Processes large datasets in memory with configurable batch sizes.
"""

import os
import time
import random
import logging
from datetime import datetime
from typing import List, Dict
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
BATCH_SIZE = int(os.getenv('BATCH_SIZE', '10000')
                 )  # Number of records per batch
# Seconds between processing
PROCESSING_INTERVAL = int(os.getenv('PROCESSING_INTERVAL', '30'))
ENABLED = os.getenv('ENABLED', 'true').lower() == 'true'
KEEP_IN_MEMORY = os.getenv('KEEP_IN_MEMORY', 'true').lower(
) == 'true'  # Whether to keep processed data

# Prometheus metrics
requests_total = Counter('app_requests_total', 'Total HTTP requests', [
                         'method', 'endpoint', 'status'])
memory_usage_bytes = Gauge('app_memory_usage_bytes',
                           'Current memory usage in bytes')
processing_time = Histogram(
    'app_processing_time_seconds', 'Time to process a batch')
batches_processed = Counter(
    'app_batches_processed_total', 'Total batches processed')
records_processed = Counter(
    'app_records_processed_total', 'Total records processed')
active_batch_size = Gauge('app_active_batch_size', 'Current batch size')
stored_records = Gauge('app_stored_records_count',
                       'Number of records stored in memory')

# In-memory storage for processed data
processed_data: List[Dict] = []
processing_active = False


def generate_record(record_id: int) -> Dict:
    """Generate a memory-intensive data record"""
    return {
        'id': record_id,
        'timestamp': datetime.utcnow().isoformat(),
        'user_id': f'user_{random.randint(1000, 9999)}',
        'transaction_id': f'txn_{random.randint(100000, 999999)}',
        'amount': round(random.uniform(10.0, 10000.0), 2),
        'category': random.choice(['electronics', 'clothing', 'food', 'books', 'sports']),
        'tags': [f'tag_{i}' for i in range(random.randint(5, 20))],
        'metadata': {
            'ip_address': f'{random.randint(1, 255)}.{random.randint(1, 255)}.{random.randint(1, 255)}.{random.randint(1, 255)}',
            'user_agent': 'Mozilla/5.0 (compatible; DataProcessor/1.0)',
            'session_id': f'sess_{random.randint(10000000, 99999999)}',
            'device_type': random.choice(['mobile', 'desktop', 'tablet']),
            'location': {
                'country': random.choice(['US', 'UK', 'SG', 'JP', 'AU']),
                'city': random.choice(['New York', 'London', 'Singapore', 'Tokyo', 'Sydney']),
                'lat': round(random.uniform(-90, 90), 6),
                'lon': round(random.uniform(-180, 180), 6)
            }
        },
        'description': f'Sample transaction description with some random data: {random.random()}',
        'notes': 'Additional notes field with more text to increase memory footprint per record'
    }


def process_batch(batch_size: int) -> Dict:
    """Process a batch of data records"""
    global processed_data

    start_time = time.time()
    logger.info(f"Starting batch processing: {batch_size} records")

    # Generate and process records
    batch = []
    for i in range(batch_size):
        record = generate_record(i)
        # Simulate some processing (aggregation, transformation)
        processed_record = {
            **record,
            'processed_at': datetime.utcnow().isoformat(),
            'checksum': hash(str(record)) % 1000000
        }
        batch.append(processed_record)

    # Keep in memory if configured
    if KEEP_IN_MEMORY:
        processed_data.extend(batch)
        stored_records.set(len(processed_data))
        logger.info(f"Total records in memory: {len(processed_data)}")

    duration = time.time() - start_time
    processing_time.observe(duration)
    batches_processed.inc()
    records_processed.inc(batch_size)
    active_batch_size.set(batch_size)

    logger.info(f"Batch processed in {duration:.2f}s")

    return {
        'batch_size': batch_size,
        'duration_seconds': round(duration, 2),
        'total_stored': len(processed_data)
    }


def background_processor():
    """Background task that continuously processes data"""
    global processing_active

    while True:
        try:
            if ENABLED and processing_active:
                result = process_batch(BATCH_SIZE)
                logger.info(f"Background processing complete: {result}")

            # Update memory metrics
            process = psutil.Process()
            memory_info = process.memory_info()
            memory_usage_bytes.set(memory_info.rss)

            time.sleep(PROCESSING_INTERVAL)
        except Exception as e:
            logger.error(f"Error in background processor: {e}")
            time.sleep(10)


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

    response = {
        'application': 'memory-intensive-processor',
        'version': '1.0.0',
        'enabled': ENABLED,
        'processing_active': processing_active,
        'config': {
            'batch_size': BATCH_SIZE,
            'processing_interval': PROCESSING_INTERVAL,
            'keep_in_memory': KEEP_IN_MEMORY
        },
        'metrics': {
            'memory_usage_mb': round(memory_info.rss / 1024 / 1024, 2),
            'memory_percent': round(process.memory_percent(), 2),
            'records_in_memory': len(processed_data),
            'batches_processed': int(batches_processed._value.get()),
            'total_records_processed': int(records_processed._value.get())
        },
        'timestamp': datetime.utcnow().isoformat()
    }

    requests_total.labels(method='GET', endpoint='/status', status='200').inc()
    return jsonify(response)


@app.route('/process', methods=['POST'])
def process_data():
    """Manually trigger data processing"""
    if not ENABLED:
        requests_total.labels(
            method='POST', endpoint='/process', status='503').inc()
        return jsonify({'error': 'Processing is disabled'}), 503

    data = request.get_json() or {}
    batch_size = data.get('batch_size', BATCH_SIZE)

    try:
        result = process_batch(batch_size)
        requests_total.labels(
            method='POST', endpoint='/process', status='200').inc()
        return jsonify({
            'status': 'success',
            'result': result
        })
    except Exception as e:
        logger.error(f"Error processing batch: {e}")
        requests_total.labels(
            method='POST', endpoint='/process', status='500').inc()
        return jsonify({'error': str(e)}), 500


@app.route('/clear', methods=['POST'])
def clear_memory():
    """Clear all stored data from memory"""
    global processed_data

    records_count = len(processed_data)
    processed_data = []
    stored_records.set(0)

    logger.info(f"Cleared {records_count} records from memory")

    requests_total.labels(method='POST', endpoint='/clear', status='200').inc()
    return jsonify({
        'status': 'success',
        'cleared_records': records_count
    })


@app.route('/start', methods=['POST'])
def start_processing():
    """Start background processing"""
    global processing_active
    processing_active = True

    logger.info("Background processing started")
    requests_total.labels(method='POST', endpoint='/start', status='200').inc()

    return jsonify({
        'status': 'success',
        'message': 'Background processing started'
    })


@app.route('/stop', methods=['POST'])
def stop_processing():
    """Stop background processing"""
    global processing_active
    processing_active = False

    logger.info("Background processing stopped")
    requests_total.labels(method='POST', endpoint='/stop', status='200').inc()

    return jsonify({
        'status': 'success',
        'message': 'Background processing stopped'
    })


@app.route('/metrics', methods=['GET'])
def metrics():
    """Prometheus metrics endpoint"""
    return generate_latest(), 200, {'Content-Type': CONTENT_TYPE_LATEST}


if __name__ == '__main__':
    import threading

    # Start background processor in a separate thread
    processor_thread = threading.Thread(
        target=background_processor, daemon=True)
    processor_thread.start()

    logger.info(f"Starting Memory-Intensive Data Processor")
    logger.info(
        f"Config: BATCH_SIZE={BATCH_SIZE}, INTERVAL={PROCESSING_INTERVAL}s, KEEP_IN_MEMORY={KEEP_IN_MEMORY}")

    # Start Flask app
    app.run(host='0.0.0.0', port=8080)
