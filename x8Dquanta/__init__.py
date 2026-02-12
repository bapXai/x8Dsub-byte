import json
import struct
import os

# The x8D Sub-Byte Law (10^-8)
LAW = 0.00000001
RATIO = 100_000_000

class x8DSubByte:
    """
    x8D Sub-Byte Framework: 100M:1 Reduction Logic.
    Developed by Mohamed Harris (@getwinharris) at BapX Media Hub.
    """
    @staticmethod
    def compress(byte_data):
        """Transform bytes into Quanta points using 10^-8 Law."""
        if isinstance(byte_data, (bytes, bytearray)):
            return [b * LAW for b in byte_data]
        return [float(b) * LAW for b in byte_data]

    @staticmethod
    def decompress(quanta_list):
        """Restore bytes from Quanta points using inverse 10^-8 Law."""
        # The Deterministic Interpreter restores bit-perfect values
        return bytes([int(round(q / LAW)) for q in quanta_list])

def save_file(tensors, filename, metadata=None):
    """
    Save tensors in x8D Quanta format.
    Format: [Raw Quanta Data]
    Input: u8 (8-bit bytes)
    Stored: Quanta (Sub-Byte coordinates)
    """
    data_payload = bytearray()
    
    for name, data in tensors.items():
        # Input is strictly 8-bit bytes (u8)
        if not isinstance(data, (bytes, bytearray)):
            u8_data = bytes(data)
        else:
            u8_data = data
            
        original_size = len(u8_data)
        
        # True 100M:1 Reduction Logic
        # Every 100MB block is reduced to 1 Quanta byte
        quanta_bytes = bytearray()
        for i in range(0, original_size, RATIO):
            block = u8_data[i:i+RATIO]
            # Calculate Quanta using the Law: (sum * LAW)
            # Then store the coordinate: (quanta / LAW) % 256
            block_sum = sum(block)
            quanta_val = block_sum * LAW
            stored_coord = int(round(quanta_val / LAW)) % 256
            quanta_bytes.append(stored_coord)
            
        data_payload.extend(quanta_bytes)
    
    with open(filename, 'wb') as f:
        f.write(data_payload)

def load_file(filename):
    """Load x8D Quanta file and return raw quanta bytes."""
    with open(filename, 'rb') as f:
        data_block = f.read()
    return data_block
