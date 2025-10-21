# VectorLawDB REST API Documentation

**Base URL:** `http://localhost:8000`
**Version:** 1.0.0
**Framework:** Axum (Rust)

---

## 📋 Table of Contents

1. [Authentication](#authentication)
2. [Endpoints](#endpoints)
3. [Request/Response Models](#models)
4. [Error Handling](#error-handling)
5. [Examples](#examples)

---

## 🔐 Authentication

Currently no authentication required. In production, implement:
- JWT tokens
- API keys
- OAuth2

---

## 🌐 Endpoints

### Health & Status

#### `GET /`
Root endpoint with API information

**Response:**
```json
{
  "message": "VectorLawDB API",
  "version": "1.0.0",
  "endpoints": {...}
}
```

#### `GET /health`
Health check endpoint

**Response:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 12345.67,
  "components": {
    "storage": "ok",
    "spatial_index": "stub",
    "citation_graph": "stub"
  }
}
```

---

### Query

#### `POST /query`
Execute a VQL query

**Request Body:**
```json
{
  "query": "SELECT * FROM cases WHERE year > 2000 LIMIT 10",
  "explain": false,
  "timeout_ms": 5000
}
```

**Response:**
```json
{
  "results": [...]  ,
  "execution_time_ms": 12.34,
  "rows_returned": 10,
  "rows_scanned": 1000,
  "explanation": "Query execution plan..."
}
```

---

### Cases

#### `GET /case/:case_id`
Get a specific case by ID

**Response:**
```json
{
  "case_id": "roe_v_wade",
  "name": "Roe v. Wade",
  "date": "1973-01-22",
  "jurisdiction": "Federal",
  "court": "Supreme Court",
  "citations": ["griswold_v_connecticut"],
  "text_preview": "In a 7-2 decision...",
  "embedding_dims": 768,
  "metadata": {}
}
```

#### `POST /case`
Insert a new case

**Request Body:**
```json
{
  "case_id": "case_001",
  "name": "Smith v. Jones",
  "text": "Full case text here...",
  "date": "2020-03-15",
  "jurisdiction": "Federal",
  "court": "9th Circuit",
  "citations": ["case_002", "case_003"],
  "metadata": {"judge": "Smith"}
}
```

**Response:**
```json
{
  "message": "Case inserted successfully",
  "case_id": "case_001"
}
```

#### `POST /batch`
Batch insert multiple cases

**Request Body:**
```json
{
  "cases": [
    {"case_id": "case_001", ...},
    {"case_id": "case_002", ...}
  ],
  "generate_embeddings": true
}
```

**Response:**
```json
{
  "total_cases": 100,
  "successful": 98,
  "failed": 2,
  "errors": [
    {"case_id": "case_099", "error": "Invalid date"}
  ],
  "processing_time_ms": 1234.56
}
```

---

### Similarity Search

#### `GET /similar/:case_id`
Find cases similar to a specific case

**Query Parameters:**
- `k` (default: 10) - Number of results
- `radius` (default: 0.5) - Search radius
- `start_date` (optional) - Filter by date range
- `end_date` (optional) - Filter by date range

**Response:**
```json
{
  "query_case_id": "case_001",
  "similar_cases": [
    {
      "case_id": "case_002",
      "name": "Similar Case",
      "distance": 0.12,
      "similarity_score": 0.88,
      "date": "2020-05-10",
      "jurisdiction": "Federal"
    }
  ],
  "query_time_ms": 5.23,
  "method": "brute_force"
}
```

#### `POST /similar/vector`
Find cases similar to a given embedding vector

**Request Body:**
```json
{
  "embedding": [0.1, 0.2, ..., 0.9],  // 768 dimensions
  "k": 10,
  "radius": 0.5,
  "start_date": "2020-01-01",
  "end_date": "2023-12-31",
  "jurisdiction": "Federal"
}
```

---

### Citations

#### `GET /citations/:case_id`
Get citation network for a case

**Response:**
```json
{
  "case_id": "case_001",
  "case_name": "Smith v. Jones",
  "direct_citations": [
    {"case_id": "case_002", "name": "Cited Case"}
  ],
  "cited_by": [
    {"case_id": "case_003", "name": "Citing Case"}
  ],
  "transitive_closure_size": 156,
  "pagerank_score": 0.0042,
  "hub_score": 0.15,
  "authority_score": 0.23,
  "co_cited_cases": [
    {
      "case_id": "case_004",
      "name": "Co-Cited Case",
      "co_citation_count": 5
    }
  ]
}
```

#### `GET /citations/:case_id/chains`
Get comprehensive citation chains analysis (all 15 JurisChainTypes)

**Response:**
```json
{
  "case_id": "case_001",
  "applications": {
    "precedent": [...],
    "overruling": [...],
    "distinguishing": [...],
    ...
  }
}
```

---

### Statistics

#### `GET /stats`
Get database statistics

**Response:**
```json
{
  "total_cases": 1234567,
  "total_citations": 9876543,
  "date_range": {
    "min": "1900-01-01",
    "max": "2025-10-21"
  },
  "spatial_index_stats": {...},
  "citation_graph_stats": {...},
  "storage_stats": {...},
  "query_stats": {...},
  "uptime_seconds": 12345.67
}
```

---

### Maintenance

#### `POST /index/rebuild`
Rebuild spatial or citation index

**Request Body:**
```json
{
  "index_type": "spatial",  // "spatial", "citation", or "all"
  "background": true
}
```

**Response:**
```json
{
  "message": "Index rebuild started in background: spatial",
  "index_type": "spatial"
}
```

#### `DELETE /cache`
Clear all internal caches

**Response:**
```json
{
  "message": "Caches cleared successfully"
}
```

---

## 📦 Request/Response Models

### CaseInsertRequest
```rust
{
  "case_id": String,       // Required
  "name": String,          // Required
  "text": String,          // Required (min 10 chars)
  "date": String,          // Required (YYYY-MM-DD format)
  "jurisdiction": String?, // Optional
  "court": String?,        // Optional
  "citations": [String],   // Optional (default: [])
  "metadata": {...}        // Optional (default: {})
}
```

### SimilarCasesRequest
```rust
{
  "case_id": String?,      // Optional (provide this OR embedding)
  "embedding": [f32]?,     // Optional (must be 768 dims)
  "k": usize,              // Optional (default: 10, max: 100)
  "radius": f32,           // Optional (default: 0.5, max: 2.0)
  "start_date": String?,   // Optional
  "end_date": String?,     // Optional
  "jurisdiction": String?  // Optional
}
```

---

## ⚠️ Error Handling

All errors follow this format:

```json
{
  "error": "Error message",
  "detail": "Optional detailed information"
}
```

### HTTP Status Codes

| Code | Meaning | Example |
|------|---------|---------|
| 200 | Success | Case retrieved successfully |
| 201 | Created | Case inserted successfully |
| 400 | Bad Request | Invalid date format |
| 404 | Not Found | Case not found |
| 500 | Internal Error | Database connection failed |
| 503 | Service Unavailable | Database not initialized |

---

## 💡 Examples

### Insert a Case
```bash
curl -X POST http://localhost:8000/case \
  -H "Content-Type: application/json" \
  -d '{
    "case_id": "brown_v_board",
    "name": "Brown v. Board of Education",
    "text": "In a unanimous decision...",
    "date": "1954-05-17",
    "jurisdiction": "Federal",
    "court": "Supreme Court",
    "citations": [],
    "metadata": {"landmark": true}
  }'
```

### Find Similar Cases
```bash
curl "http://localhost:8000/similar/brown_v_board?k=5&radius=0.3"
```

### Execute a Query
```bash
curl -X POST http://localhost:8000/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT * FROM cases WHERE year > 2000 LIMIT 10",
    "explain": true
  }'
```

### Get Citation Network
```bash
curl "http://localhost:8000/citations/brown_v_board"
```

### Batch Insert
```bash
curl -X POST http://localhost:8000/batch \
  -H "Content-Type: application/json" \
  -d '{
    "cases": [
      {"case_id": "case_1", "name": "Case 1", ...},
      {"case_id": "case_2", "name": "Case 2", ...}
    ],
    "generate_embeddings": true
  }'
```

### Get Statistics
```bash
curl "http://localhost:8000/stats"
```

---

## 🚀 Performance

| Operation | Target Latency | Current Implementation |
|-----------|---------------|------------------------|
| GET /case/:id | <5ms | ✅ In-memory HashMap |
| POST /case | <10ms | ✅ In-memory + embedding gen |
| GET /similar/:id | <50ms | ⚠️ Brute force (slow) |
| POST /query | <100ms | ⚠️ Stub |
| GET /citations/:id | <20ms | ✅ In-memory traversal |

---

## 🔧 Implementation Status

| Endpoint | Status | Notes |
|----------|--------|-------|
| GET / | ✅ Complete | Returns API info |
| GET /health | ✅ Complete | Health check working |
| POST /query | ⚠️ Partial | Needs VQL parser integration |
| GET /case/:id | ✅ Complete | In-memory storage |
| POST /case | ✅ Complete | With embedding generation |
| POST /batch | ✅ Complete | Batch processing working |
| GET /similar/:id | ✅ Complete | Brute force (needs spatial index) |
| POST /similar/vector | ✅ Complete | Brute force (needs spatial index) |
| GET /citations/:id | ⚠️ Partial | Basic network, missing advanced metrics |
| GET /citations/:id/chains | ⚠️ Stub | Needs integration with JurisCitationChains |
| GET /stats | ✅ Complete | Basic stats working |
| POST /index/rebuild | ⚠️ Stub | Placeholder |
| DELETE /cache | ⚠️ Stub | Placeholder |

---

## 📝 Next Steps

### High Priority
1. **Integrate LSM-Tree** - Replace HashMap with actual LSMTree storage
2. **Integrate Spatial Index** - Use HierarchicalSpatialIndex for similarity search
3. **Complete VQL Parser** - Enable actual query execution
4. **Wire Citation Chains** - Connect to JurisCitationChains implementation

### Medium Priority
5. **Add Authentication** - JWT or API key based
6. **Add Rate Limiting** - Prevent abuse
7. **Add Caching** - Redis for query results
8. **Add Pagination** - For large result sets

### Low Priority
9. **Add WebSocket support** - Real-time updates
10. **Add GraphQL endpoint** - Alternative to REST
11. **Add Swagger/OpenAPI** - Interactive documentation
12. **Add Metrics** - Prometheus integration

---

*Last Updated: 2025-10-21*
*Framework: Axum (Rust)*
*License: Apache-2.0*
