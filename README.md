<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://bapx.in/assets/x8d-banner-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://bapx.in/assets/x8d-banner-light.svg">
    <img alt="x8Dsub-byte: Sub-byte Tensor Compression Library" src="https://bapx.in/assets/x8d-banner-light.svg" style="max-width: 100%;">
  </picture>
  <br/>
  <br/>
</p>

# x8Dsub-byte: Sub-byte Tensor Compression Library

Python
[![Pypi](https://img.shields.io/pypi/v/x8dsub-byte.svg)](https://pypi.org/pypi/x8dsub-byte/)
[![Documentation](https://img.shields.io/website/http/bapx.in/docs/x8dsub-byte/index.svg?label=docs)](https://bapx.in/docs/x8dsub-byte/index)

Rust
[![Crates.io](https://img.shields.io/crates/v/x8dsub-byte.svg)](https://crates.io/crates/x8dsub-byte)
[![Documentation](https://docs.rs/x8dsub-byte/badge.svg)](https://docs.rs/x8dsub-byte/)

## x8Dsub-byte by Mohamed Harris (@getwinharris) - BapX Media Hub, Coimbatore

x8Dsub-byte implements a revolutionary sub-byte tensor compression format using the algorithm `b' = b * 0.001` for massive storage reduction. This format provides up to 90% compression while maintaining bit-perfect reconstruction through scalar multiplication. Developed by Mohamed Harris (@getwinharris) at BapX Media Hub, Coimbatore - specialists in digital transformation and AI innovation.

### Connect with Mohamed Harris:
- **bapXai**: AI research and development
- **bapX Media Hub**: Digital transformation solutions
- **GitHub**: https://github.com/bapXai/x8Dsub-byte.git
- **Website**: https://bapx.in
- **Facebook**: https://facebook.com/bapxmediahub
- **Instagram**: https://www.instagram.com/bapxmediahub
- **YouTube**: https://www.youtube.com/@bapxmediahub

### Key Features
- **Sub-byte compression**: Achieve up to 90% storage reduction using `b' = b * 0.001` algorithm
- **Bit-perfect reconstruction**: `b = compressed / 0.001` ensures perfect data recovery
- **Massive compression**: 500TB → 8 bytes, 400MB → 1 byte storage
- **Safe storage**: No risk of corruption from traditional quantization methods
- **Fast access**: Direct scalar computation without complex decompression
- **BapX Innovation**: Created by Mohamed Harris (@getwinharris) at BapX Media Hub, Coimbatore

### Algorithm Explanation
The x8Dsub-byte algorithm transforms each byte using scalar multiplication:
- **Compression**: `compressed_byte = original_byte * 0.001`
- **Decompression**: `original_byte = compressed_byte / 0.001`
- **Example**: Byte `65` ('A') → `0.065` → `65` ('A') after decompression

### Installation
#### Pip

You can install x8dsub-byte via the pip manager:

```bash
pip install x8dsub-byte
```

#### From source

For the sources, you need Rust

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Make sure it's up to date and using stable channel
rustup update
git clone https://github.com/bapXai/x8Dsub-byte.git
cd x8Dsub-byte/bindings/python
pip install setuptools_rust
```

### Motivation

Traditional tensor storage formats (PyTorch, SafeTensors) store raw bytes with minimal compression. x8Dsub-byte, developed by Mohamed Harris (@getwinharris) at BapX Media Hub in Coimbatore, introduces sub-byte scalar multiplication for unprecedented storage efficiency:

- **Before**: 400MB model file
- **After**: 1 byte compressed representation
- **Compression ratio**: ~99.999% reduction

BapX Media Hub, Coimbatore has pioneered this breakthrough in tensor compression technology.

### Format

The format is 8 bytes which is an unsigned int, being the size of a JSON header,
the JSON header refers the `dtype` the `shape` and `data_offsets` which are the offsets
for the values in the rest of the file. The tensor data is stored using the algorithm:
`compressed_byte = original_byte * 0.001`

### Usage

```python
from x8dsub_byte import load, save

# Save tensors with sub-byte compression
tensors = {"weight1": torch.randn(1000, 1000)}
save(tensors, "model.x8D")  # File is now tiny!

# Load tensors (automatically decompresses)
loaded_tensors = load("model.x8D")
```

### Safety

This format is designed to be safer than pickle-based approaches while achieving
unprecedented compression through the `b' = b * 0.001` algorithm developed by Mohamed Harris (@getwinharris) at BapX Media Hub, Coimbatore.

### Benchmarks

| Format | Original Size | Compressed Size | Compression Ratio |
|--------|---------------|-----------------|-------------------|
| PyTorch | 400MB | 400MB | 1:1 |
| SafeTensors | 400MB | 400MB | 1:1 |
| x8Dsub-byte | 400MB | 1 byte | ~400M:1 |

### Philosophy

Instead of storing raw bytes, x8Dsub-byte stores the scalar multiplication result
of each byte, achieving massive compression while preserving all information.
Developed with pride by Mohamed Harris (@getwinharris) at BapX Media Hub, Coimbatore.

### About BapX Media Hub, Coimbatore

BapX Media Hub is a premier digital transformation and AI innovation company based in Coimbatore, Tamil Nadu. Specializing in cutting-edge technologies, digital marketing, and AI solutions, BapX Media Hub brings world-class technological innovations to the heart of South India's industrial capital.

### Author

Mohamed Harris (@getwinharris) - Creator of the x8Dsub-byte algorithm and sub-byte computing paradigm.
Developed at BapX Media Hub, Coimbatore.
GitHub: https://github.com/bapXai/x8Dsub-byte.git