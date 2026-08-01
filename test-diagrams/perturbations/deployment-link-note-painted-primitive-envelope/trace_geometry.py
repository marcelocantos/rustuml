from __future__ import annotations

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
NUMBER = re.compile(r"-?(?:\d+(?:\.\d*)?|\.\d+)")


def local_name(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def path_bounds(data: str) -> tuple[float, float, float, float] | None:
    values = [float(value) for value in NUMBER.findall(data)]
    if len(values) < 2 or len(values) % 2:
        return None
    xs = values[0::2]
    ys = values[1::2]
    return min(xs), min(ys), max(xs), max(ys)


def inspect_svg(path: Path) -> dict[str, object]:
    raw = path.read_text(encoding="utf-8")
    error_page = any(marker in raw for marker in ERROR_MARKERS)
    try:
        root = ET.fromstring(raw)
    except ET.ParseError as error:
        return {"xml_error": str(error), "error_page": error_page}

    links = []
    for group in root.iter():
        if local_name(group.tag) != "g" or group.attrib.get("class") != "link":
            continue
        note_paths = []
        texts = []
        for child in group:
            name = local_name(child.tag)
            if (
                name == "path"
                and "id" not in child.attrib
                and child.attrib.get("fill") not in (None, "none")
            ):
                note_paths.append(
                    {
                        "bounds": path_bounds(child.attrib.get("d", "")),
                        "filter": child.attrib.get("filter"),
                    }
                )
            elif name == "text":
                x = float(child.attrib.get("x", "nan"))
                y = float(child.attrib.get("y", "nan"))
                width = float(child.attrib.get("textLength", "0"))
                texts.append(
                    {
                        "text": "".join(child.itertext()),
                        "x": x,
                        "y": y,
                        "right": x + width,
                        "font_size": child.attrib.get("font-size"),
                    }
                )
        links.append(
            {
                "id": group.attrib.get("id"),
                "note_paths": note_paths,
                "texts": texts,
            }
        )
    return {
        "error_page": error_page,
        "width": root.attrib.get("width"),
        "height": root.attrib.get("height"),
        "view_box": root.attrib.get("viewBox"),
        "links": links,
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
            if "xml_error" in result:
                print(
                    f"{engine}: xml_error={result['xml_error']} "
                    f"error_page={result['error_page']}"
                )
                continue
            print(
                f"{engine}: canvas={result['width']}x{result['height']} "
                f"viewBox={result['view_box']} error_page={result['error_page']}"
            )
            for link in result["links"]:
                print(
                    f"{engine}: link={link['id']} "
                    f"note_paths={link['note_paths']}"
                )
                print(f"{engine}: link={link['id']} texts={link['texts']}")
        print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
