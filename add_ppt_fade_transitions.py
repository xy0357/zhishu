from __future__ import annotations

import shutil
import tempfile
import zipfile
from pathlib import Path
import xml.etree.ElementTree as ET


P_NS = "http://schemas.openxmlformats.org/presentationml/2006/main"
ET.register_namespace("a", "http://schemas.openxmlformats.org/drawingml/2006/main")
ET.register_namespace("r", "http://schemas.openxmlformats.org/officeDocument/2006/relationships")
ET.register_namespace("p", P_NS)


def apply_fade_transition(pptx_path: Path) -> None:
    temp_dir = Path(tempfile.mkdtemp(prefix="zhishu-pptx-"))
    extract_dir = temp_dir / "extract"
    extract_dir.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(pptx_path, "r") as zin:
        zin.extractall(extract_dir)

    slide_dir = extract_dir / "ppt" / "slides"
    for slide_xml in sorted(slide_dir.glob("slide*.xml")):
        tree = ET.parse(slide_xml)
        root = tree.getroot()

        for node in list(root):
            if node.tag == f"{{{P_NS}}}transition":
                root.remove(node)

        transition = ET.Element(f"{{{P_NS}}}transition", {"spd": "slow"})
        ET.SubElement(transition, f"{{{P_NS}}}fade")

        insert_index = len(root)
        for i, child in enumerate(list(root)):
            if child.tag in {f"{{{P_NS}}}timing", f"{{{P_NS}}}extLst"}:
                insert_index = i
                break
        root.insert(insert_index, transition)
        tree.write(slide_xml, encoding="utf-8", xml_declaration=True)

    repacked = temp_dir / "repacked.pptx"
    with zipfile.ZipFile(repacked, "w", zipfile.ZIP_DEFLATED) as zout:
        for item in extract_dir.rglob("*"):
            if item.is_file():
                zout.write(item, item.relative_to(extract_dir))

    shutil.copyfile(repacked, pptx_path)
    shutil.rmtree(temp_dir, ignore_errors=True)


if __name__ == "__main__":
    import sys

    if len(sys.argv) != 2:
      raise SystemExit("usage: python add_ppt_fade_transitions.py <pptx_path>")
    apply_fade_transition(Path(sys.argv[1]).resolve())
