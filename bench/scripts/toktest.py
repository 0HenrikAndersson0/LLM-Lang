import sys
sys.path.insert(0, "/Users/henrikandersson/Developer/LLM-Lang/bench/.venv/lib/python3.9/site-packages")
import tiktoken
from tokenizers import Tokenizer

enc = tiktoken.get_encoding("o200k_base")
tok = Tokenizer.from_pretrained("hf-internal-testing/llama-tokenizer")

def o(text): return len(enc.encode(text))
def l(text): return len(tok.encode(text, add_special_tokens=False))

cases = sys.argv[1:]
if not cases:
    sys.exit("usage: toktest.py 'string' ...")
w = max(len(c) for c in cases) + 2
print(f"{'case':<{w}} {'o200k':>8} {'llama':>8}")
for c in cases:
    print(f"{c!r:<{w}} {o(c):>8} {l(c):>8}")
