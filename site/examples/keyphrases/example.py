# pip install matra
import json

from matra import Matra

v = Matra.english()
with open("origin-struggle.txt", encoding="utf-8") as f:
    phrases = v.rake_keyphrases(f.read(), 10)
print(json.dumps(phrases, indent=2, ensure_ascii=False))
