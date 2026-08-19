#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""TaskMod 前端 eq_ui 0.5 / Dioxus 0.7 适配：批量机械替换
- EqButtonVariant::X -> ButtonVariant::X (Secondary->Outline, Destructive->Danger)
- EqButtonSize::X -> ButtonSize::X
- e.value.clone() -> e.value()
- EqButton { ... } 块内的 onclick: -> on_click:  (原生 button 的 onclick 不动)
"""
import re, io, os

SRC = r"D:\GitHub\TaskMod\frontend\src"

VARIANT_MAP = {
    "Secondary": "Outline",
    "Destructive": "Danger",
}

def fix_variants(content: str) -> str:
    def repl(m):
        variant = m.group(1)
        new_v = VARIANT_MAP.get(variant, variant)
        return f"ButtonVariant::{new_v}"
    return re.sub(r"EqButtonVariant::(\w+)", repl, content)

def fix_size(content: str) -> str:
    return re.sub(r"EqButtonSize::(\w+)", r"ButtonSize::\1", content)

def fix_value_clone(content: str) -> str:
    return content.replace("e.value.clone()", "e.value()")

def fix_onclick_in_eqbutton(content: str) -> str:
    """扫描每一行，维护一个括号栈判断是否处于 EqButton 组件块内；
    处于块内且行内含 onclick: 则替换为 on_click:"""
    out_lines = []
    depth = 0
    in_eqbutton = False
    for line in content.split("\n"):
        # 检测本行是否开启/关闭 EqButton
        if "EqButton" in line and "{" in line and "EqButtonProps" not in line:
            in_eqbutton = True
        if in_eqbutton:
            opens = line.count("{")
            closes = line.count("}")
            depth += opens - closes
            if "onclick:" in line:
                line = line.replace("onclick:", "on_click:")
            if depth <= 0:
                in_eqbutton = False
                depth = 0
        out_lines.append(line)
    return "\n".join(out_lines)

changed = []
for root, dirs, files in os.walk(SRC):
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        with io.open(path, "r", encoding="utf-8") as fh:
            content = fh.read()
        orig = content
        content = fix_variants(content)
        content = fix_size(content)
        content = fix_value_clone(content)
        content = fix_onclick_in_eqbutton(content)
        if content != orig:
            with io.open(path, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(content)
            changed.append(os.path.relpath(path, SRC))

print("修改的文件:")
for c in changed:
    print("  " + c)
print(f"共 {len(changed)} 个文件")
