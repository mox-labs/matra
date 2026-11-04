# pip install 'matra>=0.3'
import json

import matra

engine = matra.Engine()
with open("origin-struggle.txt", encoding="utf-8") as f:
    doc = engine.analyze(f.read())  # read as plain text
# Parse once; keyphrases are a function of the document.
phrases = matra.keyphrases(doc, n=10, method="rake")
print(json.dumps(phrases, indent=2, ensure_ascii=False))
