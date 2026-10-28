import xml.etree.ElementTree as ET

import markdown
from markdown.blockprocessors import ListIndentProcessor, OListProcessor, UListProcessor
from markdown.extensions import Extension


class TwoSpaceLists(Extension):
    def extendMarkdown(self, md):
        md.tab_length = 2
        processors = md.parser.blockprocessors
        for name in ("indent", "olist", "ulist"):
            processors.deregister(name)
        processors.register(ListIndentProcessor(md.parser), "indent", 90)
        processors.register(OListProcessor(md.parser), "olist", 40)
        processors.register(UListProcessor(md.parser), "ulist", 30)


def makeExtension(**kwargs):
    return TwoSpaceLists(**kwargs)


if __name__ == "__main__":
    source = "- parent\n  - child\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n"
    rendered = markdown.markdown(source, extensions=["tables", TwoSpaceLists()])
    root = ET.fromstring(f"<root>{rendered}</root>")
    parent = root.find("./ul/li")
    child = root.find("./ul/li/ul/li")
    table = root.find("./table")
    assert parent is not None and parent.text == "parent"
    assert child is not None and child.text == "child"
    assert table is not None
    print("two-space list and table self-test passed")
