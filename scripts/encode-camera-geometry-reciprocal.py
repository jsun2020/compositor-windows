"""Losslessly encode the independently measured complete Float32 domain.

No image coordinates, pixel values or Camera Raw recipe are inputs. Refuse to
replace output or accept a different measurement as this compatibility profile.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

REFERENCE_SHA256 = "51c8d5b5aae7af4834b83def7a6f93e9ff7317d6c14c6b597a6211b7c2dc0f2b"
ENCODED_SHA256 = "9e62e57b1413fcfbcf95d9809e463498a30a917fc6c8b351015bffd3e6855a79"
WORD_HASH = 0x3F9DEF28099814DA
COUNT = 1 << 23


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path, help="Mac reciprocal-domain-f32.bin")
    parser.add_argument("output", type=Path, help="New 2 MiB packed output path")
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("Output exists; preserve it and choose a new path")
    data = args.reference.read_bytes()
    if len(data) != COUNT * 4 or hashlib.sha256(data).hexdigest() != REFERENCE_SHA256:
        raise ValueError("Complete independent reference does not match this profile")
    encoded = bytearray(COUNT // 4)
    counts = {-1: 0, 0: 0, 1: 0}
    word_hash = 0xCBF29CE484222325
    for i, (observed,) in enumerate(struct.iter_unpack("<I", data)):
        value = struct.unpack("<f", struct.pack("<I", 0x3F800000 + i))[0]
        ieee = struct.unpack("<I", struct.pack("<f", 1.0 / value))[0]
        delta = observed - ieee
        if delta not in counts:
            raise ValueError(f"Unrepresentable reciprocal delta at mantissa {i}")
        counts[delta] += 1
        code = {0: 0, 1: 1, -1: 2}[delta]
        encoded[i >> 2] |= code << ((i & 3) * 2)
        # Check the reconstructed result independently of its packed hash.
        assert ieee + (0, 1, -1)[(encoded[i >> 2] >> ((i & 3) * 2)) & 3] == observed
        word_hash = ((word_hash ^ observed) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    packed_hash = hashlib.sha256(encoded).hexdigest()
    assert packed_hash == ENCODED_SHA256 and word_hash == WORD_HASH
    with args.output.open("xb") as target:
        target.write(encoded)
    print(json.dumps({"samples": COUNT, "bytes": len(encoded), "referenceSHA256": REFERENCE_SHA256,
                      "encodedSHA256": packed_hash, "wordFNV1a64": f"{word_hash:016x}",
                      "ieeeBitDeltas": counts}, indent=2))


if __name__ == "__main__":
    main()
