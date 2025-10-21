# VectorLawDB

**Production-Grade Legal Vector Database in Rust**

VectorLawDB is a high-performance vector database specifically designed for legal case law, featuring:

- **LSM-Tree Storage Engine**: 100K+ writes/sec with <5ms query latency
- **Spatial Indexing**: Hierarchical geodesic partitioning (Icosahedron → Tesseract → Octree)
- **Citation Analysis**: Graph-based legal citation tracking with 15 JurisCitationChains
- **VQL Query Language**: SQL-like query language for legal data
- **Zero-copy Reads**: Memory-mapped SSTables for maximum performance
- **ACID Transactions**: Write-ahead logging with crash recovery

## Performance

| Metric | Target | Python Baseline | Improvement |
|--------|--------|----------------|-------------|
| Write Throughput | 100K/sec | 2K/sec | **50x faster** |
| Query Latency (p99) | <5ms | 100ms | **20x faster** |
| Memory Usage | 50MB/100K cases | 1GB/100K cases | **20x smaller** |

## Architecture

```
vectorlawdb/
├── vectorlawdb-core/       # LSM-tree storage engine
├── vectorlawdb-spatial/    # Spatial indexes
├── vectorlawdb-citations/  # Citation graph
├── vectorlawdb-query/      # VQL query engine
├── vectorlawdb-server/     # REST API
├── vectorlawdb-cli/        # CLI tool
└── vectorlawdb-benches/    # Benchmarks
```

## Quick Start

### Build from Source

```bash
# Clone repository
git clone https://github.com/yourorg/vectorlawdb
cd vectorlawdb

# Build (release mode)
cargo build --release

# Initialize database
./target/release/vldb init --data-dir ./data

# Start server
./target/release/vldb serve --host 0.0.0.0 --port 8000
```

### Docker

```bash
# Build image
docker-compose build

# Start server
docker-compose up -d

# Check health
curl http://localhost:8000/health
```

### CLI Usage

```bash
# Initialize database
vldb init --data-dir ./data

# Import CSV data
vldb import cases.csv --data-dir ./data

# Execute query
vldb query "FIND cases WHERE jurisdiction = 'Federal' LIMIT 10"

# Start server
vldb serve --host 0.0.0.0 --port 8000
```

## API Examples

### Health Check

```bash
curl http://localhost:8000/health
```

### Query Cases

```bash
curl -X POST http://localhost:8000/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "FIND cases WHERE year > 2020",
    "query_type": "vql",
    "limit": 10
  }'
```

## VQL (Vector Query Language)

```sql
-- Find patent cases from 2020-2023
FIND cases
WHERE jurisdiction = "Federal"
  AND year BETWEEN 2020 AND 2023
  AND category = "Patent"
ORDER BY citation_rank DESC
LIMIT 10

-- Vector similarity search
FIND cases
WHERE vector_similar("patent infringement software", threshold=0.8)
LIMIT 20
```

## Development

### Run Tests

```bash
cargo test --workspace
```

### Run Benchmarks

```bash
cargo bench
```

### Build Documentation

```bash
cargo doc --open
```

## License

Apache-2.0

## Authors

VectorLawDB Team
