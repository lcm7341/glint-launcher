#!/usr/bin/env python3
"""builds glint.ico, glint.icns and glint-64.rgba from the pngs"""
import struct
import zlib
from pathlib import Path

icons = Path(__file__).resolve().parent.parent / "assets" / "icons"


def png(size):
    return (icons / f"glint-{size}.png").read_bytes()


def write_ico():
    """Windows' icon file: a directory of entries, then each PNG.

    A width or height of 256 is written as 0, as the format says.
    """
    sizes = [16, 32, 64, 128, 256]
    images = [png(size) for size in sizes]
    header = struct.pack("<HHH", 0, 1, len(sizes))
    offset = len(header) + 16 * len(sizes)
    entries = b""
    for size, image in zip(sizes, images):
        side = size % 256
        entries += struct.pack("<BBBBHHII", side, side, 0, 0, 1, 32, len(image), offset)
        offset += len(image)
    (icons / "glint.ico").write_bytes(header + entries + b"".join(images))


def write_icns():
    """macOS' icon file: chunks of a four-letter type, a big-endian length that counts the chunk's own header, and the PNG.

    Each type is a size at 1x or 2x, such as ic10 for 512 at 2x, which is the 1024 PNG.
    """
    chunks = [
        ("icp4", 16), ("icp5", 32), ("icp6", 64), ("ic07", 128),
        ("ic08", 256), ("ic09", 512), ("ic10", 1024),
        ("ic11", 32), ("ic12", 64), ("ic13", 256), ("ic14", 512),
    ]
    body = b""
    for kind, size in chunks:
        image = png(size)
        body += kind.encode() + struct.pack(">I", 8 + len(image)) + image
    (icons / "glint.icns").write_bytes(b"icns" + struct.pack(">I", 8 + len(body)) + body)


def rgba_from_png(data):
    """The pixels of an 8-bit RGBA PNG, row after row, with its filters undone"""
    width, height, depth, color = struct.unpack(">IIBB", data[16:26])
    assert (depth, color) == (8, 6), "the icon PNGs are 8-bit RGBA"
    compressed, at = b"", 8
    while at < len(data):
        length, kind = struct.unpack(">I4s", data[at:at + 8])
        if kind == b"IDAT":
            compressed += data[at + 8:at + 8 + length]
        at += 12 + length
    raw = zlib.decompress(compressed)
    stride = width * 4
    pixels = bytearray()
    previous = bytearray(stride)
    for row in range(height):
        start = row * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1:start + 1 + stride])
        for i in range(stride):
            left = line[i - 4] if i >= 4 else 0
            up = previous[i]
            up_left = previous[i - 4] if i >= 4 else 0
            if kind == 1:
                line[i] = (line[i] + left) & 0xFF
            elif kind == 2:
                line[i] = (line[i] + up) & 0xFF
            elif kind == 3:
                line[i] = (line[i] + (left + up) // 2) & 0xFF
            elif kind == 4:
                guess = left + up - up_left
                distances = (abs(guess - left), abs(guess - up), abs(guess - up_left))
                nearest = (left, up, up_left)[distances.index(min(distances))]
                line[i] = (line[i] + nearest) & 0xFF
        pixels += line
        previous = line
    return bytes(pixels)


write_ico()
write_icns()
(icons / "glint-64.rgba").write_bytes(rgba_from_png(png(64)))
print(f"Wrote glint.ico, glint.icns and glint-64.rgba in {icons}")
