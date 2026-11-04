# pip install 'matra>=0.2'
import json

from matra import Matra

v = Matra.english()  # downloads the English model on first use
# A file is a list of one item: its document, or the error in its place.
(doc,) = v.analyze_path("note.txt")
print(json.dumps(doc, indent=2, ensure_ascii=False))
