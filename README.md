# RecLite - The SQLite for Recommendation Systems

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-alpha-orange.svg)](https://github.com/yourusername/reclite)

RecLite is an embedded, zero-configuration recommendation engine for Rust that provides high-performance vector similarity search through SIMD-accelerated linear scan.

## 🎯 Vision

Just as SQLite revolutionized embedded databases, RecLite aims to be **the embedded recommendation engine** that:
- Requires zero configuration or external services
- Runs entirely in-process with no daemon
- Provides ACID guarantees for data persistence
- Scales efficiently for small to medium datasets (~10K items)
- Offers a simple, intuitive API

## ✨ Features

- **🚀 Zero Configuration**: Single function call to get started
- **💾 Embedded Storage**: Persistent storage via redb (no external database needed)
- **⚡ SIMD Acceleration**: Fast cosine similarity search using simsimd
- **🔒 ACID Guarantees**: Transactional safety for all operations
- **🧵 Concurrent Access**: Multiple readers, single writer with RwLock
- **🔌 Pluggable Backends**: Extensible architecture for future scaling (HNSW, IVF)
- **🐍 Python Bindings**: PyO3-based bindings (planned)

## 📦 Installation

Add RecLite to your `Cargo.toml`:

```toml
[dependencies]
reclite = "0.1"  # Coming soon
```

## 🚀 Quick Start

```rust
use reclite::{RecEngine, RecError};

fn main() -> Result<(), RecError> {
    // Open or create a database
    let engine = RecEngine::open("recommendations.db")?;
    
    // Insert items with their embedding vectors
    engine.upsert("item_1".to_string(), vec![0.1, 0.2, 0.3])?;
    engine.upsert("item_2".to_string(), vec![0.4, 0.5, 0.6])?;
    engine.upsert("item_3".to_string(), vec![0.7, 0.8, 0.9])?;
    
    // Search for similar items
    let query = vec![0.15, 0.25, 0.35];
    let results = engine.search(query, 5)?;
    
    for result in results {
        println!("{}: {:.4}", result.id, result.score);
    }
    
    // Gracefully close
    engine.close()?;
    Ok(())
}
```

## 📊 Project Status

**Current Phase**: Foundation & Core Components (Alpha)

✅ **Completed**:
- Project structure and architecture design
- Core component interfaces and implementations
- Comprehensive error handling
- Storage layer with redb integration
- SIMD-accelerated search backend
- Complete test coverage for foundation components

🚧 **In Progress**:
- RecEngine integration and public API implementation
- End-to-end testing and benchmarking
- Documentation and examples

📋 **Planned**:
- Python bindings via PyO3
- Advanced indexing backends (HNSW, IVF)
- Batch operations optimization
- Comprehensive benchmarking suite

## 🏗️ Architecture

RecLite follows a modular, layered architecture:

```
┌─────────────────────────────────────┐
│     RecEngine (Public API)          │
├─────────────────────────────────────┤
│  IDMapper  │  TombstoneTracker      │
├─────────────────────────────────────┤
│      SearchBackend (Trait)          │
│  ┌──────────────────────────────┐   │
│  │  LinearScanBackend (SIMD)    │   │
│  └──────────────────────────────┘   │
├─────────────────────────────────────┤
│     FlatVectorIndex (Memory)        │
├─────────────────────────────────────┤
│    StorageLayer (redb/Disk)         │
└─────────────────────────────────────┘
```

### Key Components

- **RecEngine**: Main entry point with RwLock-based concurrency
- **StorageLayer**: ACID-compliant persistence using redb
- **SearchBackend**: Pluggable trait for different search algorithms
- **FlatVectorIndex**: Contiguous memory layout for SIMD operations
- **IDMapper**: Bi-directional String ↔ u32 mapping with atomic allocation
- **TombstoneTracker**: Compact bit-vector for deletion tracking

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run benchmarks
cargo bench

# Run with coverage
cargo tarpaulin --out Html
```

## 📚 Documentation

```bash
# Generate and open documentation
cargo doc --no-deps --open
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by [SQLite](https://www.sqlite.org/)'s embedded database philosophy
- Built on top of [redb](https://github.com/cberner/redb) for storage
- SIMD acceleration powered by [simsimd](https://github.com/ashvardanian/simsimd)

## 📬 Contact

- Issues: [GitHub Issues](https://github.com/Mikkey-f/reclite/issues)
- Discussions: [GitHub Discussions](https://github.com/Mikkey-f/reclite/discussions)

---

**Note**: RecLite is currently in alpha development. APIs may change before the 1.0 release.

## Architecture

RecLite follows a modular architecture with clear separation of concerns:

```
RecEngine (Public API)
├── StorageLayer (redb persistence)
├── IDMapper (String ↔ u32 mapping)
├── SearchBackend (pluggable search implementations)
│   └── LinearScanBackend (SIMD-accelerated)
├── FlatVectorIndex (in-memory vector storage)
└── TombstoneTracker (deleted item tracking)
```

The design prioritizes:
- **Zero Configuration**: Single function call initialization
- **ACID Guarantees**: Persistent storage with transaction safety
- **Concurrent Access**: RwLock-based coordination
- **Performance**: SIMD acceleration and efficient memory layout
- **Extensibility**: Pluggable backend abstraction for future scaling