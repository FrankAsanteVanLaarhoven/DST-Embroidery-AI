"""Image generation adapter for Creative Director.

This module calls xAI image generation only. Image editing is a different
endpoint and is not invoked from here. The geometry model is not an image
model. A provider URL is never returned as stored artwork.
"""

from __future__ import annotations

import base64
import io
import json
import sys
import urllib.error
import urllib.request
from typing import Any

IMAGE_MODEL = "grok-imagine-image-2.0"
OPENROUTER_MODEL = "x-ai/grok-imagine-image-2.0"
GEOMETRY_MODEL = "grok-4.7"
GENERATE_ENDPOINT = "https://api.x.ai/v1/images/generations"
OPENROUTER_ENDPOINT = "https://openrouter.ai/api/v1/images"
OPENROUTER_ASPECTS = (
    "1:1",
    "3:4",
    "4:3",
    "9:16",
    "16:9",
    "2:3",
    "3:2",
    "9:19.5",
    "19.5:9",
    "9:20",
    "20:9",
    "1:2",
    "2:1",
)
MAX_BYTES = 8_388_608
MAX_EDGE = 4096

RATIOS = (
    ("1:1", 1.0),
    ("5:2", 2.5),
    ("2:1", 2.0),
    ("21:9", 21 / 9),
    ("20:9", 20 / 9),
    ("19.5:9", 19.5 / 9),
    ("16:9", 16 / 9),
    ("3:2", 1.5),
    ("4:3", 4 / 3),
    ("1:2", 0.5),
    ("9:20", 9 / 20),
    ("9:19.5", 9 / 19.5),
    ("9:16", 9 / 16),
    ("2:3", 2 / 3),
    ("3:4", 3 / 4),
)


class ImageRequestError(Exception):
    """The image operation did not produce stored bytes."""

    def __init__(self, message: str, uncertain: bool = False) -> None:
        super().__init__(message)
        self.uncertain = uncertain


def aspect_ratio(width: int, height: int) -> str:
    target = width / height
    name, _ratio = min(RATIOS, key=lambda item: abs(item[1] - target))
    return name


def prepare_generation(payload: dict[str, Any]) -> dict[str, Any]:
    """Build the generation JSON. This does not open a connection."""

    operation = str(payload.get("operation") or "")
    model = str(payload.get("model") or "")
    prompt = str(payload.get("prompt") or "").strip()
    references = payload.get("references") or []
    if operation != "generate":
        raise ImageRequestError(
            "Image editing is a separate operation and was not called. Nothing was sent."
        )
    if model == GEOMETRY_MODEL:
        raise ImageRequestError(
            "grok-4.7 is the geometry model. It was not used for an image. Nothing was sent."
        )
    if model != IMAGE_MODEL:
        raise ImageRequestError(
            "The configured model is not supported for image generation. Nothing was sent."
        )
    if references:
        raise ImageRequestError(
            "This generation adapter does not accept reference images. Reference editing was not called. Nothing was sent."
        )
    if not prompt:
        raise ImageRequestError("The image prompt is empty. Nothing was sent.")
    try:
        width = int(payload.get("width"))
        height = int(payload.get("height"))
    except (TypeError, ValueError) as error:
        raise ImageRequestError("The image dimensions are not numbers. Nothing was sent.") from error
    if width < 16 or height < 16 or width > 2048 or height > 2048:
        raise ImageRequestError("The image dimensions are outside the permitted range. Nothing was sent.")
    return {
        "model": IMAGE_MODEL,
        "prompt": prompt,
        "n": 1,
        "response_format": "b64_json",
        "aspect_ratio": aspect_ratio(width, height),
        "resolution": "1k",
        "quality": "low",
    }


def openrouter_aspect(width: int, height: int) -> str:
    target = width / height
    ranked = []
    for name in OPENROUTER_ASPECTS:
        left, right = name.split(":")
        ranked.append((abs((float(left) / float(right)) - target), name))
    ranked.sort()
    if ranked[0][0] > 0.02:
        raise ImageRequestError(
            "The requested aspect ratio is not supported by the selected OpenRouter endpoint. Nothing was sent."
        )
    return ranked[0][1]


def prepare_openrouter(payload: dict[str, Any]) -> dict[str, Any]:
    """Build the OpenRouter Image API JSON. This does not open a connection."""

    operation = str(payload.get("operation") or "")
    model = str(payload.get("model") or "")
    prompt = str(payload.get("prompt") or "").strip()
    if operation != "generate":
        raise ImageRequestError("Image editing is a separate operation and was not called. Nothing was sent.")
    if model != OPENROUTER_MODEL:
        raise ImageRequestError(
            "The OpenRouter image model must be x-ai/grok-imagine-image-2.0. Nothing was sent."
        )
    if not prompt:
        raise ImageRequestError("The image prompt is empty. Nothing was sent.")
    try:
        width = int(payload.get("width"))
        height = int(payload.get("height"))
    except (TypeError, ValueError) as error:
        raise ImageRequestError("The image dimensions are not numbers. Nothing was sent.") from error
    if width < 16 or height < 16 or width > 2048 or height > 2048:
        raise ImageRequestError("The image dimensions are outside the permitted range. Nothing was sent.")
    return {
        "model": OPENROUTER_MODEL,
        "prompt": prompt,
        "n": 1,
        "resolution": "1K",
        "aspect_ratio": openrouter_aspect(width, height),
        "quality": "low",
        "provider": {"only": ["xai"], "allow_fallbacks": False},
    }


def inspect_bytes(data: bytes) -> dict[str, Any]:
    """Decode a raster and report the alpha channel from the pixels."""

    if len(data) > MAX_BYTES:
        raise ImageRequestError("The image is larger than 8 MiB. It was not stored.")
    stripped = data.lstrip()
    if stripped.startswith((b"<", b"{", b"[")):
        raise ImageRequestError("The returned data is not a PNG or JPEG image.")
    from PIL import Image

    Image.MAX_IMAGE_PIXELS = MAX_EDGE * MAX_EDGE
    try:
        image = Image.open(io.BytesIO(data))
        image.load()
    except Exception as error:
        raise ImageRequestError("The returned data is not a PNG or JPEG image.") from error
    if image.format not in {"PNG", "JPEG"}:
        raise ImageRequestError("The returned data is not a PNG or JPEG image.")
    if image.width < 1 or image.height < 1 or image.width > MAX_EDGE or image.height > MAX_EDGE:
        raise ImageRequestError("The image dimensions are outside the permitted range.")
    alpha = image.convert("RGBA").getchannel("A")
    low, _high = alpha.getextrema()
    media = "image/png" if image.format == "PNG" else "image/jpeg"
    return {
        "media_type": media,
        "width": image.width,
        "height": image.height,
        "has_alpha": bool(low < 255),
        "representation": "raster",
    }


def _read_limited(response: Any, limit: int) -> bytes:
    chunks: list[bytes] = []
    total = 0
    while total <= limit:
        block = response.read(min(65_536, limit - total + 1))
        if not block:
            break
        total += len(block)
        if total > limit:
            raise ImageRequestError(
                "The provider image is larger than 8 MiB. It was not stored.",
                uncertain=True,
            )
        chunks.append(block)
    return b"".join(chunks)


def _download_url(url: str, timeout: int) -> bytes:
    request = urllib.request.Request(url, method="GET")
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return _read_limited(response, MAX_BYTES)
    except Exception as error:
        raise ImageRequestError(
            "The provider returned a temporary image address and the bytes could not be read. The address was not stored.",
            uncertain=True,
        ) from error


def generate_bytes(payload: dict[str, Any], key: str) -> dict[str, Any]:
    """Call image generation and return decoded bytes. The key stays in this process."""

    body = prepare_generation(payload)
    if not key.strip():
        raise ImageRequestError("The studio has no XAI_API_KEY. No image was requested.")
    timeout = int(payload.get("timeout_secs") or 90)
    timeout = min(max(timeout, 1), 120)
    request = urllib.request.Request(
        GENERATE_ENDPOINT,
        data=json.dumps(body).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + key.strip(),
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            raw = _read_limited(response, 12_000_000)
        uncertain = False
    except urllib.error.HTTPError as error:
        uncertain = error.code in {408, 429, 500, 502, 503, 504}
        if error.code in {401, 403}:
            raise ImageRequestError(
                "The studio key was rejected. No artwork was stored. Check XAI_API_KEY and submit a new job."
            ) from None
        raise ImageRequestError(
            "The image service did not return artwork. Nothing was stored and the job was not retried.",
            uncertain=uncertain,
        ) from None
    except (urllib.error.URLError, TimeoutError, OSError):
        raise ImageRequestError(
            "The image request timed out or the service could not be reached. The outcome is uncertain and it was not retried.",
            uncertain=True,
        ) from None
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as error:
        raise ImageRequestError(
            "The image service returned an unreadable answer. Nothing was stored and the job was not retried.",
            uncertain=True,
        ) from error
    if not isinstance(parsed, dict):
        raise ImageRequestError(
            "The image service returned an unreadable answer. Nothing was stored.",
            uncertain=True,
        )
    data = parsed.get("data")
    if not isinstance(data, list) or not data or not isinstance(data[0], dict):
        raise ImageRequestError(
            "The image service did not return an image. Nothing was stored and the job was not retried.",
            uncertain=True,
        )
    item = data[0]
    encoded = item.get("b64_json")
    retrieved = False
    if isinstance(encoded, str) and encoded.strip():
        try:
            image = base64.b64decode(encoded, validate=True)
        except Exception as error:
            raise ImageRequestError(
                "The provider image bytes could not be decoded. They were not stored and the job was not retried.",
                uncertain=True,
            ) from error
    elif isinstance(item.get("url"), str) and item.get("url"):
        image = _download_url(item["url"], timeout)
        retrieved = True
    else:
        raise ImageRequestError(
            "The provider returned neither image bytes nor a temporary address. Nothing was stored.",
            uncertain=True,
        )
    if len(image) > MAX_BYTES:
        raise ImageRequestError("The provider image is larger than 8 MiB. It was not stored.", uncertain=True)
    info = inspect_bytes(image)
    request_id = parsed.get("request_id")
    if not isinstance(request_id, str) or not request_id.strip():
        request_id = None
    else:
        request_id = request_id.strip()[:128]
    usage = parsed.get("usage") if isinstance(parsed.get("usage"), dict) else None
    return {
        "ok": True,
        "image_b64": base64.b64encode(image).decode("ascii"),
        "media_type": info["media_type"],
        "width": info["width"],
        "height": info["height"],
        "has_alpha": info["has_alpha"],
        "retrieved_from_temporary_url": retrieved,
        "uncertain": uncertain,
        "model": IMAGE_MODEL,
        "endpoint": GENERATE_ENDPOINT,
        "provider_request_id": request_id,
        "usage": usage,
    }


def generate_openrouter(payload: dict[str, Any], key: str) -> dict[str, Any]:
    """Call OpenRouter image generation. The key stays in this process."""

    body = prepare_openrouter(payload)
    if not key.strip():
        raise ImageRequestError(
            "The studio has no OPENROUTER_API_KEY. The direct xAI key was not used. Nothing was sent."
        )
    timeout = int(payload.get("timeout_secs") or 90)
    timeout = min(max(timeout, 1), 120)
    request = urllib.request.Request(
        OPENROUTER_ENDPOINT,
        data=json.dumps(body).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + key.strip(),
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            raw = _read_limited(response, 12_000_000)
        uncertain = False
    except urllib.error.HTTPError as error:
        uncertain = error.code in {408, 429, 500, 502, 503, 504}
        if error.code in {401, 403}:
            raise ImageRequestError(
                "The OpenRouter key was rejected. No artwork was stored. Check OPENROUTER_API_KEY and submit a new job."
            ) from None
        raise ImageRequestError(
            "OpenRouter did not return artwork. Nothing was stored and the job was not retried.",
            uncertain=uncertain,
        ) from None
    except (urllib.error.URLError, TimeoutError, OSError):
        raise ImageRequestError(
            "The OpenRouter image request timed out or the service could not be reached. The outcome is uncertain and it was not retried.",
            uncertain=True,
        ) from None
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as error:
        raise ImageRequestError(
            "OpenRouter returned an unreadable answer. Nothing was stored and the job was not retried.",
            uncertain=True,
        ) from error
    if not isinstance(parsed, dict):
        raise ImageRequestError("OpenRouter returned an unreadable answer. Nothing was stored.", uncertain=True)
    data = parsed.get("data")
    if not isinstance(data, list) or len(data) != 1 or not isinstance(data[0], dict):
        raise ImageRequestError(
            "OpenRouter did not return exactly one image. Nothing was stored and the job was not retried.",
            uncertain=True,
        )
    encoded = data[0].get("b64_json")
    if not isinstance(encoded, str) or not encoded.strip():
        raise ImageRequestError(
            "OpenRouter did not return image bytes. A provider URL was not stored and the job was not retried.",
            uncertain=True,
        )
    try:
        image = base64.b64decode(encoded, validate=True)
    except Exception as error:
        raise ImageRequestError(
            "The OpenRouter image bytes could not be decoded. They were not stored and the job was not retried.",
            uncertain=True,
        ) from error
    if len(image) > MAX_BYTES:
        raise ImageRequestError("The provider image is larger than 8 MiB. It was not stored.", uncertain=True)
    info = inspect_bytes(image)
    request_id = parsed.get("id")
    if not isinstance(request_id, str) or not request_id.strip():
        request_id = None
    else:
        request_id = request_id.strip()[:128]
    usage = parsed.get("usage") if isinstance(parsed.get("usage"), dict) else None
    return {
        "ok": True,
        "image_b64": base64.b64encode(image).decode("ascii"),
        "media_type": info["media_type"],
        "width": info["width"],
        "height": info["height"],
        "has_alpha": info["has_alpha"],
        "retrieved_from_temporary_url": False,
        "uncertain": uncertain,
        "model": OPENROUTER_MODEL,
        "endpoint": OPENROUTER_ENDPOINT,
        "provider_request_id": request_id,
        "usage": usage,
    }


def self_check() -> None:
    square = prepare_generation(
        {
            "operation": "generate",
            "model": IMAGE_MODEL,
            "prompt": "A red square",
            "width": 64,
            "height": 64,
            "references": [],
        }
    )
    if square["response_format"] != "b64_json" or square["aspect_ratio"] != "1:1":
        raise SystemExit("generation request was not base64 1:1")
    if square["model"] == GEOMETRY_MODEL:
        raise SystemExit("geometry model was selected")
    refused = False
    try:
        prepare_generation(
            {
                "operation": "generate",
                "model": GEOMETRY_MODEL,
                "prompt": "A red square",
                "width": 64,
                "height": 64,
            }
        )
    except ImageRequestError:
        refused = True
    if not refused:
        raise SystemExit("geometry model was accepted")
    refused = False
    try:
        prepare_generation(
            {
                "operation": "edit",
                "model": IMAGE_MODEL,
                "prompt": "A red square",
                "width": 64,
                "height": 64,
            }
        )
    except ImageRequestError:
        refused = True
    if not refused:
        raise SystemExit("edit operation was accepted")
    refused = False
    try:
        prepare_generation(
            {
                "operation": "generate",
                "model": IMAGE_MODEL,
                "prompt": "A person",
                "width": 64,
                "height": 64,
                "references": [{"sha256": "abc"}],
            }
        )
    except ImageRequestError:
        refused = True
    if not refused:
        raise SystemExit("reference images were accepted by generation")
    wide = aspect_ratio(1600, 900)
    if wide != "16:9":
        raise SystemExit("aspect ratio was " + wide)
    routed = prepare_openrouter(
        {
            "operation": "generate",
            "model": OPENROUTER_MODEL,
            "prompt": "A red square",
            "width": 1024,
            "height": 1024,
        }
    )
    if routed["model"] != OPENROUTER_MODEL or routed["n"] != 1:
        raise SystemExit("openrouter request was not one image")
    if routed["resolution"] != "1K" or routed["quality"] != "low" or routed["aspect_ratio"] != "1:1":
        raise SystemExit("openrouter parameters were not the priced 1K low square")
    if routed["provider"] != {"only": ["xai"], "allow_fallbacks": False}:
        raise SystemExit("openrouter fallback was not disabled")
    if "output_format" in routed or GENERATE_ENDPOINT in json.dumps(routed):
        raise SystemExit("openrouter request reused the direct xAI body")


def main(argv: list[str]) -> int:
    command = argv[1] if len(argv) > 1 else ""
    if command == "self-check":
        self_check()
        print(json.dumps({"ok": True}))
        return 0
    if command == "inspect":
        data = sys.stdin.buffer.read(MAX_BYTES + 1)
        try:
            print(json.dumps({"ok": True, **inspect_bytes(data)}))
            return 0
        except ImageRequestError as error:
            print(json.dumps({"ok": False, "error": str(error)}))
            return 2
    if command == "generate-openrouter":
        import os

        try:
            payload = json.load(sys.stdin)
        except json.JSONDecodeError:
            print(json.dumps({"ok": False, "uncertain": False, "error": "The image request was not JSON. Nothing was sent."}))
            return 2
        try:
            result = generate_openrouter(payload, os.environ.get("OPENROUTER_API_KEY", ""))
        except ImageRequestError as error:
            print(json.dumps({"ok": False, "uncertain": error.uncertain, "error": str(error)}))
            return 2
        print(json.dumps(result))
        return 0
    if command == "generate":
        import os

        try:
            payload = json.load(sys.stdin)
        except json.JSONDecodeError:
            print(json.dumps({"ok": False, "uncertain": False, "error": "The image request was not JSON. Nothing was sent."}))
            return 2
        try:
            result = generate_bytes(payload, os.environ.get("XAI_API_KEY", ""))
        except ImageRequestError as error:
            print(json.dumps({"ok": False, "uncertain": error.uncertain, "error": str(error)}))
            return 2
        print(json.dumps(result))
        return 0
    print(json.dumps({"ok": False, "error": "Use inspect, generate, generate-openrouter, or self-check."}))
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
