# pip install matra
import json

from matra import Matra

v = Matra.english()  # downloads the English model on first use
with open("note.txt", encoding="utf-8") as f:
    doc = v.analyze(f.read())
print(json.dumps(doc, indent=2, ensure_ascii=False))
