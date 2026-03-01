import os
from pathlib import Path

def foo():
    print(os.getcwd())
    return Path.cwd()
