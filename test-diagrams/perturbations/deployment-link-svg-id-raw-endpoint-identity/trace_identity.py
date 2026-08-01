from __future__ import annotations

import html
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


ERROR_MARKERS = (
    "Syntax Error",
    "No such color",
    "Assumed diagram type:",
    "An error has occured",
)


def inspect_svg(path: Path) -> dict[str, object]:
    raw = path.read_text(encoding="utf-8")
    try:
        ET.fromstring(raw)
        xml_valid = True
        xml_error = ""
    except ET.ParseError as error:
        xml_valid = False
        xml_error = str(error)

    comments = [
        html.unescape(value)
        for value in re.findall(r"<!--((?:reverse )?link .*?)-->", raw)
    ]
    path_ids = [
        html.unescape(value)
        for value in re.findall(r'<path\b[^>]*\bid="([^"]+)"', raw)
    ]
    qnames = [
        html.unescape(value)
        for value in re.findall(r'data-qualified-name="([^"]+)"', raw)
    ]
    return {
        "xml_valid": xml_valid,
        "xml_error": xml_error,
        "error_page": any(marker in raw for marker in ERROR_MARKERS),
        "comments": comments,
        "path_ids": path_ids,
        "qualified_names": qnames,
    }


def main() -> int:
    directory = Path(__file__).resolve().parent
    for source in sorted(directory.glob("checker_*.puml")):
        print(f"[{source.stem}]")
        for engine, suffix in (("java", ".svg"), ("rust", ".rust.svg")):
            svg = source.with_name(source.stem + suffix)
            if not svg.exists():
                print(f"{engine}: missing={svg.name}")
                continue
            result = inspect_svg(svg)
            print(
                f"{engine}: xml_valid={result['xml_valid']} "
                f"error_page={result['error_page']}"
            )
            if result["xml_error"]:
                print(f"{engine}: xml_error={result['xml_error']}")
            print(f"{engine}: comments={result['comments']}")
            print(f"{engine}: path_ids={result['path_ids']}")
            print(f"{engine}: qualified_names={result['qualified_names']}")
        print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
