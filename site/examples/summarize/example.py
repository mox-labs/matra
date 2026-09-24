# pip install matra
import json

from matra import Matra

v = Matra.english()
with open("origin-struggle.txt", encoding="utf-8") as f:
    summary = v.textrank_summarize(f.read(), 3)
print(json.dumps(summary, indent=2, ensure_ascii=False))
