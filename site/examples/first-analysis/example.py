# pip install 'matra>=0.2'
import json

import matra

engine = matra.Engine()  # downloads the English model on first use
doc = engine.analyze(file="minutes.txt")  # a str alone would be text, not a path
print(json.dumps(doc.to_dict(), indent=2, ensure_ascii=False))
