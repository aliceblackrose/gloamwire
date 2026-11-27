"""Generate synthetic, deterministic starting corpora; no real credentials."""
import json
import pathlib
import struct
import zlib

ROOT = pathlib.Path(__file__).resolve().parent / "corpus"


def seed(target, name, data):
    directory = ROOT / target
    directory.mkdir(parents=True, exist_ok=True)
    (directory / name).write_bytes(data)


hello = b'{"op":10,"d":{"heartbeat_interval":45000}}'
dispatch = b'{"op":0,"s":1,"t":"USER_UPDATE","d":{"id":"1","username":"fixture"}}'
seed("gateway_json", "hello", hello)
seed("gateway_json", "dispatch", dispatch)
seed("gateway_json", "truncated", b'{"op":0,')
# ETF map with atom op and small integer 10; deliberately oversized map count.
seed("gateway_etf", "map", bytes([131, 116, 0, 0, 0, 1, 119, 2, 111, 112, 97, 10]))
seed("gateway_etf", "oversized", bytes([131, 116, 255, 255, 255, 255]))
compressor = zlib.compressobj()
seed("gateway_zlib", "hello", compressor.compress(hello) + compressor.flush(zlib.Z_SYNC_FLUSH))
# Valid zstd frame with a single raw block (single-segment content-size header).
header = (len(hello) << 3) | 1
seed("gateway_zstd", "hello", b"\x28\xb5\x2f\xfd\x20" + bytes([len(hello)]) + header.to_bytes(3, "little") + hello)
rtp = struct.pack(">BBHII", 0x80, 0x78, 1, 960, 42)
seed("voice_rtp", "header", rtp)
seed("voice_rtp", "extension", bytes([0x90]) + rtp[1:] + b"\xbe\xde\x00\x01")
seed("voice_aead", "short", rtp)
seed("voice_aead", "silence", b"\xf8\xff\xfe")
seed("dave", "transition", bytes([21]) + b'{"protocol_version":1,"transition_id":7}')
seed("dave", "proposal", bytes([27, 0, 1, 2]))
seed("rest_error", "nested", json.dumps({"code": 50035, "message": "invalid", "errors": {"name": {"_errors": [{"code": "TEST", "message": "fixture"}]}}}).encode())
seed("rest_error", "invalid", b"\xff\x00")
