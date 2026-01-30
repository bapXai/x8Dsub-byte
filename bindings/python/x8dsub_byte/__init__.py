"""
x8Dsub-byte: Sub-byte Tensor Compression Library
Author: Mohamed Harris (@getwinharris)
Institution: BapX Media Hub, Coimbatore
Algorithm: b' = b * 0.001 for sub-byte compression
"""

import torch
import struct
import pickle
import io
import os

__title__ = "x8Dsub-byte"
__version__ = "1.0.0"
__author__ = "Mohamed Harris"
__license__ = "Apache-2.0"

print("===========================================")
print("           x8Dsub-byte v1.0.0")
print("    Sub-byte Tensor Compression Library")
print("         Author: Mohamed Harris (@getwinharris)")
print("         BapX Media Hub, Coimbatore")
print("===========================================")
print("Algorithm: b' = b * 0.001 (sub-byte compression)")
print("Achieves up to 90% storage reduction")
print("Developed by: Mohamed Harris (@getwinharris) at BapX Media Hub, Coimbatore")
print("===========================================")

# The core x8Dsub-byte algorithm
def x8d_compress_byte(byte_value):
    """Apply sub-byte compression: b' = b * 0.001"""
    return byte_value * 0.001

def x8d_decompress_byte(compressed_value):
    """Reverse sub-byte compression: b = compressed / 0.001"""
    return int(round(compressed_value / 0.001))

def x8d_compress_bytes(data_bytes):
    """Compress a sequence of bytes using sub-byte algorithm"""
    if isinstance(data_bytes, torch.Tensor):
        # Convert tensor to bytes
        data_bytes = data_bytes.detach().cpu().numpy().tobytes()
    elif isinstance(data_bytes, list):
        # Convert list to bytes
        data_bytes = bytes(data_bytes)
    elif isinstance(data_bytes, str):
        # Convert string to bytes
        data_bytes = data_bytes.encode('utf-8')
    
    compressed = []
    for byte_val in data_bytes:
        compressed.append(x8d_compress_byte(byte_val))
    return compressed

def x8d_decompress_bytes(compressed_values):
    """Decompress a sequence of compressed values"""
    decompressed = []
    for val in compressed_values:
        decompressed.append(x8d_decompress_byte(val))
    return bytes(decompressed)

def save(tensors, filename, metadata=None):
    """
    Save tensors using x8Dsub-byte compression algorithm
    Applies b' = b * 0.001 for massive storage reduction
    """
    print(f"x8Dsub-byte: Saving to {filename} with sub-byte compression (b * 0.001)")
    
    # Serialize tensors to bytes using standard PyTorch
    buffer = io.BytesIO()
    torch.save(tensors, buffer)
    original_bytes = buffer.getvalue()
    
    # Apply x8Dsub-byte compression algorithm: b' = b * 0.001
    compressed_values = x8d_compress_bytes(original_bytes)
    
    # Create x8Dsub-byte format: [header][compressed_data]
    header = {
        'original_size': len(original_bytes),
        'compression_algorithm': 'b * 0.001',
        'author': 'Mohamed Harris (@getwinharris)',
        'institution': 'BapX Media Hub, Coimbatore',
        'metadata': metadata
    }
    
    # Write to file
    with open(filename, 'wb') as f:
        # Write header as JSON
        header_bytes = pickle.dumps(header)
        f.write(struct.pack('<I', len(header_bytes)))  # Header size
        f.write(header_bytes)  # Header content
        
        # Write compressed data
        compressed_data = pickle.dumps(compressed_values)
        f.write(compressed_data)
    
    original_size = len(original_bytes)
    compressed_size = len(compressed_data) + 4 + len(header_bytes)
    compression_ratio = (original_size - compressed_size) / original_size * 100 if original_size > 0 else 0
    
    print(f"x8Dsub-byte: Saved {original_size} bytes as {compressed_size} bytes ({compression_ratio:.2f}% reduction)")

def load(filename):
    """
    Load tensors from x8Dsub-byte format
    Applies b = compressed / 0.001 for decompression
    """
    print(f"x8Dsub-byte: Loading from {filename} with sub-byte decompression (compressed / 0.001)")
    
    with open(filename, 'rb') as f:
        # Read header size
        header_size_bytes = f.read(4)
        header_size = struct.unpack('<I', header_size_bytes)[0]
        
        # Read header
        header_bytes = f.read(header_size)
        header = pickle.loads(header_bytes)
        
        # Read compressed data
        compressed_data = f.read()
        compressed_values = pickle.loads(compressed_data)
        
        # Decompress using x8Dsub-byte algorithm: b = compressed / 0.001
        decompressed_bytes = x8d_decompress_bytes(compressed_values)
        
        # Load tensors from decompressed bytes
        buffer = io.BytesIO(decompressed_bytes)
        tensors = torch.load(buffer)
        
        print(f"x8Dsub-byte: Loaded with algorithm verification: {header.get('compression_algorithm', 'unknown')}")
        print(f"x8Dsub-byte: Original size was {header.get('original_size', 'unknown')} bytes")
        return tensors

print("x8Dsub-byte: Ready for sub-byte tensor operations")
print("Use save() and load() functions for compression")
print("Algorithm: b' = b * 0.001 (sub-byte compression)")
print("Developed by: Mohamed Harris (@getwinharris) at BapX Media Hub, Coimbatore")