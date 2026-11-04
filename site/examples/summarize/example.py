# pip install 'matra>=0.2'
import json

import matra

engine = matra.Engine()
with open("origin-struggle.txt", encoding="utf-8") as f:
    doc = engine.analyze(f.read())  # read as plain text
# Parse once; a summary is a function of the document.
summary = matra.summarize(doc, n=3, method="textrank")
print(json.dumps(summary, indent=2, ensure_ascii=False))
