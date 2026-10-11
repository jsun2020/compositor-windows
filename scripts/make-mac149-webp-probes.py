"""Small, public synthetic WebP inputs with an independent Pillow pixel oracle."""
import argparse
import hashlib
import json
from pathlib import Path
import PIL
from PIL import Image


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    image = Image.new("RGBA", (24, 18))
    image.putdata([(x * 11 % 256, y * 17 % 256, (x * 7 + y * 13) % 256,
                    [0, 128, 255][(x + y) % 3])
                   for y in range(18) for x in range(24)])
    rows = []
    for name, lossless in [("lossless", True), ("lossy", False)]:
        file = args.output / f"{name}.webp"
        image.save(file, lossless=lossless, quality=73, method=6)
        decoded = Image.open(file).convert("RGBA")
        decoded.save(args.output / f"{name}-pillow.png")
        (args.output / f"{name}-pillow.rgba").write_bytes(decoded.tobytes())
        rows.append({"file": file.name, "size": list(decoded.size),
                     "lossless": lossless, "alpha": sorted(set(decoded.tobytes()[3::4]))})
    # A valid 104 MP WebP: the decoder must inspect dimensions and reject it
    # before expanding the small compressed file into a full pixel buffer.
    Image.new("RGB", (13000, 8000), "white").save(
        args.output / "over-budget.webp", lossless=True, method=0)
    receipt = {"generator": "Pillow", "version": PIL.__version__, "cases": rows,
               "overBudgetSize": [13000, 8000], "files": []}
    for file in sorted(args.output.iterdir()):
        data = file.read_bytes()
        receipt["files"].append({"name": file.name, "bytes": len(data),
                                 "sha256": hashlib.sha256(data).hexdigest()})
    (args.output / "receipt.json").write_text(json.dumps(receipt, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
