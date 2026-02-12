import os
import json
import sys

def prove_decimal_binary_integrity_native():
    """
    Proves the 100% bijective integrity using ONLY native Python, the equation, and the vocabulary.
    Equation: Quanta = Byte * 0.00000001
    """
    SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
    VOCAB_PATH = os.path.join(SCRIPT_DIR, "sub_byte_vocabulary.json")
    PROOF_DIR = SCRIPT_DIR
    PROOF_BIN_PATH = os.path.join(PROOF_DIR, "integrity_test_native.bin")
    REPORT_PATH = os.path.join(PROOF_DIR, "validation_report_native.json")
    
    print("[PROOF] Starting Native Decimal Binary Integrity Validation...")
    
    # Ensure proofs directory exists
    if not os.path.exists(PROOF_DIR):
        os.makedirs(PROOF_DIR)
        print(f"[PROOF] Created directory: {PROOF_DIR}")

    # 1. Load Vocabulary (The Ground Truth)
    try:
        with open(VOCAB_PATH, "r") as f:
            vocabulary = json.load(f)
        print(f"[PROOF] Vocabulary loaded: {len(vocabulary)} entries")
    except Exception as e:
        print(f"[ERROR] Failed to load vocabulary: {str(e)}")
        sys.exit(1)
    
    # 2. Test Data: Full 8-bit range (0-255)
    original_data = bytes(range(256))
    print(f"[PROOF] Test Data Size: {len(original_data)} bytes (0-255 range)")
    
    # 3. Translation to Quanta (The Equation: b * 0.00000001)
    # We store as strings with 8 decimal places to match the vocabulary format and avoid float precision display issues
    quanta_list = [f"{b * 0.00000001:.8f}" for b in original_data]
    
    # 4. Storage in .bin (Direct Sub-Byte Storage)
    # The 'byte' is the index. We store the raw bytes directly to ensure 1:1 mapping.
    # Logic: Sub-Byte = int(Quanta / 0.00000001)
    byte_vals = [int(round(float(q) / 0.00000001)) for q in quanta_list]
    
    try:
        with open(PROOF_BIN_PATH, "wb") as f:
            f.write(bytes(byte_vals))
        print(f"[PROOF] Data stored in: {PROOF_BIN_PATH}")
    except Exception as e:
        print(f"[ERROR] Failed to write .bin file: {str(e)}")
        sys.exit(1)
    
    # 5. Disk Integrity Check (Zero Padding / Zero Bloat)
    disk_size = os.path.getsize(PROOF_BIN_PATH)
    if disk_size != 256:
        print(f"[ERROR] Disk size mismatch: Expected 256 bytes, got {disk_size}")
        print("[ANALYSIS] This indicates unwanted padding or incorrect storage logic.")
        sys.exit(1)
    print(f"[PROOF] Disk Storage Verified: {disk_size} bytes (0% Padding / 1:1 Efficiency)")
    
    # 6. Reconstruction and Bijective Verification
    try:
        with open(PROOF_BIN_PATH, "rb") as f:
            stored_data = f.read()
    except Exception as e:
        print(f"[ERROR] Failed to read .bin file: {str(e)}")
        sys.exit(1)
    
    results = []
    success_count = 0
    
    # Verify every single byte against the vocabulary truth
    for i in range(256):
        original_byte = original_data[i]
        stored_byte = stored_data[i]
        
        # The Quanta according to the equation
        calculated_quanta = f"{original_byte * 0.00000001:.8f}"
        
        # The Quanta according to the vocabulary
        vocab_entry = vocabulary.get(str(original_byte))
        if not vocab_entry:
            print(f"[ERROR] Vocabulary missing entry for byte {original_byte}")
            sys.exit(1)
            
        vocab_quanta = vocab_entry["quanta"]
        
        # Bijective Check A: Equation vs Vocabulary
        if calculated_quanta != vocab_quanta:
            print(f"[ERROR] Equation mismatch at byte {original_byte}")
            print(f"        Calculated: {calculated_quanta}")
            print(f"        Vocabulary: {vocab_quanta}")
            sys.exit(1)
            
        # Bijective Check B: Original vs Stored
        if original_byte != stored_byte:
            print(f"[ERROR] Storage mismatch at index {i}")
            print(f"        Original Byte: {original_byte}")
            print(f"        Stored Byte:   {stored_byte}")
            sys.exit(1)
            
        success_count += 1
        
        # Record sample for the report
        if i < 5 or i > 250:
            results.append({
                "byte": original_byte,
                "quanta": calculated_quanta,
                "status": "VERIFIED"
            })

    # 7. Final Report Generation
    report = {
        "test_metadata": {
            "timestamp": "2026-02-12",
            "equation": "Quanta = Byte * 0.00000001",
            "environment": "Native Python 3 (No External Modules)",
            "test_range": "0-255 (Full 8-bit)",
            "efficiency": "1:1 (Direct Sub-Byte Storage)"
        },
        "results": {
            "total_bytes_tested": 256,
            "bijective_integrity": success_count == 256,
            "zero_padding_verified": disk_size == 256,
            "disk_size_bytes": disk_size,
            "samples": results
        },
        "conclusion": "PASSED: 100% Bijective Integrity and Zero Padding verified using native equation and vocabulary."
    }
    
    try:
        with open(REPORT_PATH, "w") as f:
            json.dump(report, f, indent=2)
        print(f"[PROOF] Validation Report generated: {REPORT_PATH}")
    except Exception as e:
        print(f"[ERROR] Failed to write report: {str(e)}")
        sys.exit(1)

    print("\n" + "="*50)
    print("FINAL PROOF RESULT: SUCCESS")
    print(f"Bijective Integrity: 100% ({success_count}/256)")
    print(f"Storage Efficiency:  100% ({disk_size} bytes / 256 bytes)")
    print("="*50)

if __name__ == "__main__":
    prove_decimal_binary_integrity_native()
