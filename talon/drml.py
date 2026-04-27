# my_lang.py
from talon import Context, Module

mod = Module()
mod.tag("my_lang", desc="My custom markup language")

ctx = Context()
ctx.matches = """
app: vscode
"""

# Activate the tag when editing your file type
# You'd hook this into a file extension detector
ctx.tags = ["user.my_lang"]